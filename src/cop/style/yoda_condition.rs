use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, FALSE_NODE, FLOAT_NODE, INTEGER_NODE, NIL_NODE, STRING_NODE, SYMBOL_NODE, TRUE_NODE};

pub struct YodaCondition;

fn is_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
}

impl Cop for YodaCondition {
    fn name(&self) -> &'static str {
        "Style/YodaCondition"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, FALSE_NODE, FLOAT_NODE, INTEGER_NODE, NIL_NODE, STRING_NODE, SYMBOL_NODE, TRUE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "forbid_for_all_comparison_operators");
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let name = call.name().as_slice();

        let is_equality = name == b"==" || name == b"!=";
        let is_comparison = is_equality || name == b"<" || name == b">" || name == b"<=" || name == b">=";

        if !is_comparison {
            return Vec::new();
        }

        // For *_equality_operators_only styles, skip non-equality operators
        let equality_only = enforced_style == "forbid_for_equality_operators_only"
            || enforced_style == "require_for_equality_operators_only";
        if equality_only && !is_equality {
            return Vec::new();
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return Vec::new();
        }

        let require_yoda = enforced_style == "require_for_all_comparison_operators"
            || enforced_style == "require_for_equality_operators_only";

        if require_yoda {
            // Require Yoda: flag when literal is on the RIGHT (non-Yoda)
            if !is_literal(&receiver) && is_literal(&arg_list[0]) {
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(source, line, column, "Prefer Yoda conditions.".to_string())];
            }
        } else {
            // Forbid Yoda: flag when literal is on the LEFT (Yoda)
            if is_literal(&receiver) && !is_literal(&arg_list[0]) {
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(source, line, column, "Prefer non-Yoda conditions.".to_string())];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(YodaCondition, "cops/style/yoda_condition");

    #[test]
    fn both_literals_not_flagged() {
        let source = b"1 == 1\n";
        let diags = run_cop_full(&YodaCondition, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn nil_on_left_is_flagged() {
        let source = b"nil == x\n";
        let diags = run_cop_full(&YodaCondition, source);
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn require_yoda_style() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("require_for_all_comparison_operators".into())),
            ]),
            ..CopConfig::default()
        };
        // Non-Yoda should be flagged
        let source = b"x == 1\n";
        let diags = run_cop_full_with_config(&YodaCondition, source, config.clone());
        assert_eq!(diags.len(), 1, "Should flag non-Yoda with require style");
        assert!(diags[0].message.contains("Prefer Yoda"));

        // Yoda should be allowed
        let source2 = b"1 == x\n";
        let diags2 = run_cop_full_with_config(&YodaCondition, source2, config);
        assert!(diags2.is_empty(), "Should allow Yoda conditions with require style");
    }

    #[test]
    fn forbid_equality_only_skips_comparisons() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("forbid_for_equality_operators_only".into())),
            ]),
            ..CopConfig::default()
        };
        // `1 == x` should be flagged (equality Yoda)
        let source = b"1 == x\n";
        let diags = run_cop_full_with_config(&YodaCondition, source, config.clone());
        assert_eq!(diags.len(), 1, "Should flag equality Yoda");

        // `1 < x` should NOT be flagged (comparison, not equality)
        let source2 = b"1 < x\n";
        let diags2 = run_cop_full_with_config(&YodaCondition, source2, config);
        assert!(diags2.is_empty(), "Should skip non-equality comparison operators");
    }

    #[test]
    fn forbid_all_flags_comparison_operators() {
        // Default: forbid_for_all_comparison_operators
        // `1 < x` should be flagged (Yoda with comparison)
        let source = b"1 < x\n";
        let diags = run_cop_full(&YodaCondition, source);
        assert_eq!(diags.len(), 1, "Should flag comparison Yoda with default style");
    }
}
