use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DynamicFindBy;

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
        _parse_result: &ruby_prism::ParseResult<'_>,
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
        if call.receiver().is_none() {
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
            // Skip if any argument is a hash (keyword args) or splat
            if arg_list
                .iter()
                .any(|arg| arg.as_keyword_hash_node().is_some() || arg.as_splat_node().is_some())
            {
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
