use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct TernaryParentheses;

/// Check if a parenthesized node contains a safe assignment (=) in ternary context.
fn is_ternary_safe_assignment(paren: &ruby_prism::ParenthesesNode<'_>) -> bool {
    let body = match paren.body() {
        Some(b) => b,
        None => return false,
    };
    if let Some(stmts) = body.as_statements_node() {
        let stmts_body = stmts.body();
        if stmts_body.len() == 1 {
            let inner = &stmts_body.iter().next().unwrap();
            return inner.as_local_variable_write_node().is_some()
                || inner.as_instance_variable_write_node().is_some()
                || inner.as_class_variable_write_node().is_some()
                || inner.as_global_variable_write_node().is_some()
                || inner.as_constant_write_node().is_some();
        }
    }
    body.as_local_variable_write_node().is_some()
        || body.as_instance_variable_write_node().is_some()
        || body.as_class_variable_write_node().is_some()
        || body.as_global_variable_write_node().is_some()
        || body.as_constant_write_node().is_some()
}

/// Check if a condition is "complex" (not a simple variable/constant/method call).
fn is_complex_condition(node: &ruby_prism::Node<'_>) -> bool {
    // Simple: variables, constants, method calls
    if node.as_local_variable_read_node().is_some()
        || node.as_instance_variable_read_node().is_some()
        || node.as_class_variable_read_node().is_some()
        || node.as_global_variable_read_node().is_some()
        || node.as_constant_read_node().is_some()
        || node.as_constant_path_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_self_node().is_some()
    {
        return false;
    }
    // Method calls without operators are simple
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        // Operator methods (except []) are complex
        if !name[0].is_ascii_alphabetic() && name[0] != b'_' && name != b"[]" {
            return true;
        }
        return false;
    }
    // Everything else is complex (and, or, binary ops, etc.)
    true
}

impl Cop for TernaryParentheses {
    fn name(&self) -> &'static str {
        "Style/TernaryParentheses"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "require_no_parentheses");
        let allow_safe = config.get_bool("AllowSafeAssignment", true);
        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Ternary has no if_keyword_loc
        if if_node.if_keyword_loc().is_some() {
            return Vec::new();
        }

        let predicate = if_node.predicate();
        let is_parenthesized = predicate.as_parentheses_node().is_some();

        // AllowSafeAssignment: skip if condition is a parenthesized assignment
        if allow_safe && is_parenthesized {
            if let Some(paren) = predicate.as_parentheses_node() {
                if is_ternary_safe_assignment(&paren) {
                    return Vec::new();
                }
            }
        }

        match enforced_style {
            "require_parentheses" => {
                if !is_parenthesized {
                    let loc = predicate.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(source, line, column, "Use parentheses for ternary conditions.".to_string())];
                }
            }
            "require_parentheses_when_complex" => {
                let is_complex = is_complex_condition(&predicate);
                if is_complex && !is_parenthesized {
                    let loc = predicate.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(source, line, column, "Use parentheses for ternary expressions with complex conditions.".to_string())];
                } else if !is_complex && is_parenthesized {
                    let paren = predicate.as_parentheses_node().unwrap();
                    let open_loc = paren.opening_loc();
                    let (line, column) = source.offset_to_line_col(open_loc.start_offset());
                    return vec![self.diagnostic(source, line, column, "Only use parentheses for ternary expressions with complex conditions.".to_string())];
                }
            }
            _ => {
                // "require_no_parentheses" (default)
                if is_parenthesized {
                    let paren = predicate.as_parentheses_node().unwrap();
                    let open_loc = paren.opening_loc();
                    let (line, column) = source.offset_to_line_col(open_loc.start_offset());
                    return vec![self.diagnostic(source, line, column, "Ternary conditions should not be wrapped in parentheses.".to_string())];
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{run_cop_full_with_config, run_cop_full};

    crate::cop_fixture_tests!(TernaryParentheses, "cops/style/ternary_parentheses");

    #[test]
    fn require_parentheses_flags_missing() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("require_parentheses".into())),
            ]),
            ..CopConfig::default()
        };
        // No parens should be flagged
        let source = b"x = foo? ? 'a' : 'b'\n";
        let diags = run_cop_full_with_config(&TernaryParentheses, source, config.clone());
        assert_eq!(diags.len(), 1, "Should flag missing parens with require_parentheses");
        assert!(diags[0].message.contains("Use parentheses"));

        // With parens should be OK
        let source2 = b"x = (foo?) ? 'a' : 'b'\n";
        let diags2 = run_cop_full_with_config(&TernaryParentheses, source2, config);
        assert!(diags2.is_empty(), "Should allow parens with require_parentheses");
    }

    #[test]
    fn allow_safe_assignment_in_ternary() {
        // Default AllowSafeAssignment is true, so (x = y) ? a : b should be allowed
        let source = b"(x = y) ? 'a' : 'b'\n";
        let diags = run_cop_full(&TernaryParentheses, source);
        assert!(diags.is_empty(), "Should allow safe assignment parens");
    }

    #[test]
    fn disallow_safe_assignment() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("AllowSafeAssignment".into(), serde_yml::Value::Bool(false)),
            ]),
            ..CopConfig::default()
        };
        let source = b"(x = y) ? 'a' : 'b'\n";
        let diags = run_cop_full_with_config(&TernaryParentheses, source, config);
        assert_eq!(diags.len(), 1, "Should flag safe assignment parens when disallowed");
    }
}
