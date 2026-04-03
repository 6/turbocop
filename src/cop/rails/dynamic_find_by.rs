use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util::parent_class_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks dynamic `find_by_*` methods and suggests using `find_by` instead.
///
/// ## Investigation findings (2026-03-15)
///
/// **FP root cause (1 FP):** Hash literal arguments (`{ "key" => value }`) were not
/// filtered out. Only `KeywordHashNode` (bare keyword args) was checked, but Prism
/// represents explicit `{ ... }` hash literals as `HashNode`. RuboCop's
/// `IGNORED_ARGUMENT_TYPES = %i[hash splat]` covers both. Fixed by also checking
/// `as_hash_node()`.
///
/// **FN root cause (50 FN):** Receiverless `find_by_*` calls inside classes inheriting
/// from `ApplicationRecord` or `ActiveRecord::Base` were unconditionally skipped.
/// RuboCop uses `ActiveRecordHelper#inherit_active_record_base?` to walk up class
/// ancestors. Fixed by walking the AST to find the enclosing class and checking its
/// superclass against known AR base class patterns.
pub struct DynamicFindBy;

/// Check if a superclass name indicates ActiveRecord inheritance.
fn is_active_record_parent(parent: &[u8]) -> bool {
    parent == b"ApplicationRecord"
        || parent == b"ActiveRecord::Base"
        || parent == b"::ApplicationRecord"
        || parent == b"::ActiveRecord::Base"
}

/// Walk the AST to find the enclosing class of a given byte offset,
/// and check if it inherits from an ActiveRecord base class.
fn is_inside_active_record_class(
    source: &SourceFile,
    node_offset: usize,
    parse_result: &ruby_prism::ParseResult<'_>,
) -> bool {
    let mut finder = ArClassFinder {
        source,
        target_offset: node_offset,
        result: false,
    };
    finder.visit(&parse_result.node());
    finder.result
}

struct ArClassFinder<'a> {
    source: &'a SourceFile,
    target_offset: usize,
    result: bool,
}

impl<'a> Visit<'a> for ArClassFinder<'a> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'a>) {
        let loc = node.location();
        if self.target_offset >= loc.start_offset() && self.target_offset < loc.end_offset() {
            if let Some(parent) = parent_class_name(self.source, node) {
                if is_active_record_parent(parent) {
                    self.result = true;
                }
            }
            // Continue walking in case of nested classes
            ruby_prism::visit_class_node(self, node);
        }
    }
}

impl Cop for DynamicFindBy {
    fn name(&self) -> &'static str {
        "Rails/DynamicFindBy"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // AllowedMethods (Whitelist is deprecated alias)
        let allowed = config.get_string_array("AllowedMethods");
        let whitelist = config.get_string_array("Whitelist");
        let allowed_receivers = config.get_string_array("AllowedReceivers");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };
        let name = call.name().as_slice();
        if !name.starts_with(b"find_by_") {
            return;
        }

        // For receiverless calls, only flag inside AR model classes
        if call.receiver().is_none()
            && !is_inside_active_record_class(source, node.location().start_offset(), parse_result)
        {
            return;
        }

        // Skip if method is in AllowedMethods or Whitelist (deprecated alias)
        let name_str = std::str::from_utf8(name).unwrap_or("");
        if let Some(ref list) = allowed {
            if list.iter().any(|m| m == name_str) {
                return;
            }
        }
        if let Some(ref list) = whitelist {
            if list.iter().any(|m| m == name_str) {
                return;
            }
        }

        // Skip if receiver is in AllowedReceivers
        if let Some(ref receivers) = allowed_receivers {
            if let Some(recv) = call.receiver() {
                let recv_bytes = recv.location().as_slice();
                let recv_str = std::str::from_utf8(recv_bytes).unwrap_or("");
                if receivers.iter().any(|r| r == recv_str) {
                    return;
                }
            }
        }

        // Extract the suffix after "find_by_" (strip trailing "!" if present)
        let attr = &name[b"find_by_".len()..];
        let attr_str = std::str::from_utf8(attr).unwrap_or("...");
        let attr_base = attr_str.strip_suffix('!').unwrap_or(attr_str);

        // Split by "_and_" to determine expected column count
        let column_keywords: Vec<&str> = attr_base.split("_and_").collect();
        let expected_arg_count = column_keywords.len();

        // Validate argument count and types match dynamic finder pattern
        if let Some(args) = call.arguments() {
            let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
            // Argument count must match column count
            if arg_list.len() != expected_arg_count {
                return;
            }
            // Skip if any argument is a hash (keyword args, hash literal) or splat
            if arg_list.iter().any(|arg| {
                arg.as_keyword_hash_node().is_some()
                    || arg.as_hash_node().is_some()
                    || arg.as_splat_node().is_some()
            }) {
                return;
            }
        } else {
            // No arguments at all — only valid if there's exactly 1 column
            // (e.g., `find_by_name` with no args)
            if expected_arg_count != 0 {
                return;
            }
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let msg = format!(
            "Use `find_by({attr_str}: ...)` instead of `{}`.",
            std::str::from_utf8(name).unwrap_or("find_by_...")
        );
        diagnostics.push(self.diagnostic(source, line, column, msg));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DynamicFindBy, "cops/rails/dynamic_find_by");

    #[test]
    fn whitelist_suppresses_offense() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "Whitelist".to_string(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String(
                    "find_by_name".to_string(),
                )]),
            )]),
            ..CopConfig::default()
        };
        let source = b"User.find_by_name('foo')\n";
        let diags = run_cop_full_with_config(&DynamicFindBy, source, config);
        assert!(
            diags.is_empty(),
            "Whitelist should suppress offense for find_by_name"
        );
    }
}
