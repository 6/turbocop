use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Rails/FindEach — flags `Model.scope.each` chains that should use `find_each`.
///
/// ## Corpus investigation (2026-03-08)
///
/// **FP=277 root causes:**
/// 1. No-receiver scope calls (`all.each {}`) were flagged outside AR classes.
///    RuboCop only flags these inside classes inheriting `ApplicationRecord`,
///    `::ApplicationRecord`, `ActiveRecord::Base`, or `::ActiveRecord::Base`.
///    Switched from `check_node` to `check_source` with a visitor that tracks
///    class inheritance context.
/// 2. `model.errors.where(:title).each {}` was flagged — RuboCop skips when
///    the receiver of `where` is a call to `errors` (Active Model Errors).
/// 3. AllowedMethods/AllowedPatterns were only checked against the immediate
///    inner method, not the entire receiver chain. RuboCop walks all send nodes
///    in the chain (e.g., `User.order(:name).includes(:company).each` is
///    suppressed because `order` is in AllowedMethods).
/// 4. `select`, `limit`, `order` anywhere in the chain should suppress the
///    offense — these are in the default AllowedMethods (order, limit) or are
///    not AR scope methods (select), but when chained with AR scopes, the
///    entire expression should not be flagged if any link is in AllowedMethods.
///
/// **FN=198 root causes:**
/// No-receiver calls in AR classes (`where(name: name).each(&:touch)` inside
/// `class Model < ApplicationRecord`) were not flagged because the old
/// `check_node` approach had no class context.
///
/// **Fix:** Rewrote cop to use `check_source` with a `Visit` struct that
/// maintains a class inheritance stack. The visitor walks the AST, tracking
/// whether the current scope is inside an AR-inheriting class. For each
/// `.each` call, it walks the full receiver chain to collect all method names
/// and checks AllowedMethods/AllowedPatterns against the entire chain.
///
/// ## Follow-up investigation (2026-03-10)
///
/// **FP=4 root causes:**
/// 1. Hardcoded AllowedMethods default was `["order", "limit"]` but vendor
///    `config/default.yml` has `["order", "limit", "select", "lock"]`. When
///    config resolution fell back to hardcoded defaults, `select` and `lock`
///    in a chain before an AR scope method (e.g., `User.select(:name).where(active: true).each`)
///    were not suppressed.
/// 2. AllowedPatterns used substring `contains()` matching instead of regex.
///    RuboCop's `matches_allowed_pattern?` compiles patterns as `Regexp`, so
///    patterns like `^order$` would work as regex but fail with substring matching.
///
/// **Fix:** Updated hardcoded AllowedMethods default to include `select` and
/// `lock`. Changed AllowedPatterns matching from `contains()` to `regex::Regex`.
pub struct FindEach;

const AR_SCOPE_METHODS: &[&[u8]] = &[
    b"all",
    b"eager_load",
    b"includes",
    b"joins",
    b"left_joins",
    b"left_outer_joins",
    b"not",
    b"or",
    b"preload",
    b"references",
    b"unscoped",
    b"where",
];

/// Parent class names that indicate ActiveRecord inheritance.
const AR_BASE_CLASSES: &[&[u8]] = &[
    b"ApplicationRecord",
    b"::ApplicationRecord",
    b"ActiveRecord::Base",
    b"::ActiveRecord::Base",
];

impl Cop for FindEach {
    fn name(&self) -> &'static str {
        "Rails/FindEach"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allowed_methods = config
            .get_string_array("AllowedMethods")
            .unwrap_or_else(|| {
                vec![
                    "order".to_string(),
                    "limit".to_string(),
                    "select".to_string(),
                    "lock".to_string(),
                ]
            });
        let allowed_patterns = config
            .get_string_array("AllowedPatterns")
            .unwrap_or_default();

        let mut visitor = FindEachVisitor {
            cop: self,
            source,
            allowed_methods,
            allowed_patterns,
            diagnostics: Vec::new(),
            in_ar_class: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct FindEachVisitor<'a, 'src> {
    cop: &'a FindEach,
    source: &'src SourceFile,
    allowed_methods: Vec<String>,
    allowed_patterns: Vec<String>,
    diagnostics: Vec<Diagnostic>,
    /// Whether we are inside a class that inherits from ActiveRecord.
    in_ar_class: bool,
}

impl<'pr> FindEachVisitor<'_, '_> {
    fn check_call(&mut self, call: &ruby_prism::CallNode<'pr>) {
        // Must be an `each` call
        if call.name().as_slice() != b"each" {
            return;
        }

        // Skip safe navigation chains (&.each)
        if call
            .call_operator_loc()
            .is_some_and(|op| op.as_slice() == b"&.")
        {
            return;
        }

        // The receiver must be a CallNode (a method call, not a constant/variable)
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let inner_method = inner_call.name().as_slice();

        // Must be an AR scope method
        if !AR_SCOPE_METHODS.contains(&inner_method) {
            return;
        }

        // If the inner call has no receiver (e.g., `all.each` or `where(x).each`),
        // only flag if we're inside an AR-inheriting class
        if inner_call.receiver().is_none() && !self.in_ar_class {
            return;
        }

        // Check for Active Model Errors pattern: errors.where(:title).each
        if inner_method == b"where" {
            if let Some(inner_recv) = inner_call.receiver() {
                if let Some(inner_recv_call) = inner_recv.as_call_node() {
                    if inner_recv_call.name().as_slice() == b"errors" {
                        return;
                    }
                }
            }
        }

        // Walk the entire receiver chain and check AllowedMethods/AllowedPatterns
        // against ALL methods in the chain (matching RuboCop's behavior)
        if self.chain_has_allowed_method(call) {
            return;
        }

        // Report offense at the `.each` selector (matches RuboCop behavior)
        let msg_loc = match call.message_loc() {
            Some(loc) => loc,
            None => return,
        };
        let (line, column) = self.source.offset_to_line_col(msg_loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Use `find_each` instead of `each` for batch processing.".to_string(),
        ));
    }

    /// Walk the entire method chain from `each` down to the base receiver,
    /// collecting all method names. Return true if any method in the chain
    /// matches AllowedMethods or AllowedPatterns.
    fn chain_has_allowed_method(&self, node: &ruby_prism::CallNode<'pr>) -> bool {
        let mut current: Option<ruby_prism::Node<'pr>> = node.receiver();
        while let Some(ref recv) = current {
            if let Some(call) = recv.as_call_node() {
                let method_name = call.name().as_slice();
                let method_str = std::str::from_utf8(method_name).unwrap_or("");

                if self.allowed_methods.iter().any(|m| m == method_str) {
                    return true;
                }
                if self
                    .allowed_patterns
                    .iter()
                    .any(|p| regex::Regex::new(p).is_ok_and(|re| re.is_match(method_str)))
                {
                    return true;
                }

                current = call.receiver();
            } else {
                break;
            }
        }
        false
    }
}

impl<'pr> Visit<'pr> for FindEachVisitor<'_, '_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let prev_in_ar = self.in_ar_class;

        // Check if this class inherits from an AR base class
        if let Some(superclass) = node.superclass() {
            let loc = superclass.location();
            let parent_bytes = &self.source.as_bytes()[loc.start_offset()..loc.end_offset()];
            if AR_BASE_CLASSES.contains(&parent_bytes) {
                self.in_ar_class = true;
            }
        }

        ruby_prism::visit_class_node(self, node);
        self.in_ar_class = prev_in_ar;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        self.check_call(node);
        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FindEach, "cops/rails/find_each");

    #[test]
    fn allowed_patterns_suppresses_offense() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowedPatterns".to_string(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("order".to_string())]),
            )]),
            ..CopConfig::default()
        };
        let source = b"User.order(:name).each { |u| puts u }\n";
        let diags = run_cop_full_with_config(&FindEach, source, config);
        assert!(
            diags.is_empty(),
            "AllowedPatterns should suppress offense for matching method"
        );
    }

    #[test]
    fn allowed_patterns_uses_regex_matching() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // Regex pattern with anchors should match via regex, not substring
        let config = CopConfig {
            options: HashMap::from([(
                "AllowedPatterns".to_string(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("^order$".to_string())]),
            )]),
            ..CopConfig::default()
        };
        let source = b"User.order(:name).each { |u| puts u }\n";
        let diags = run_cop_full_with_config(&FindEach, source, config);
        assert!(
            diags.is_empty(),
            "AllowedPatterns should use regex matching (^order$ should match 'order')"
        );
    }
}
