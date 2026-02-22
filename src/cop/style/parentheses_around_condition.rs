use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CLASS_VARIABLE_WRITE_NODE, CONSTANT_WRITE_NODE, GLOBAL_VARIABLE_WRITE_NODE, IF_NODE, INSTANCE_VARIABLE_WRITE_NODE, LOCAL_VARIABLE_WRITE_NODE, MULTI_WRITE_NODE, PARENTHESES_NODE, STATEMENTS_NODE, UNLESS_NODE, UNTIL_NODE, WHILE_NODE};

pub struct ParenthesesAroundCondition;

/// Check if the content of a parenthesized node is a safe assignment (=).
/// RuboCop allows `if (a = b)` by default (AllowSafeAssignment: true).
fn is_safe_assignment(node: &ruby_prism::ParenthesesNode<'_>) -> bool {
    let body = match node.body() {
        Some(b) => b,
        None => return false,
    };
    // The body inside parens could be a statements node with a single assignment
    if let Some(stmts) = body.as_statements_node() {
        let stmts_body = stmts.body();
        if stmts_body.len() == 1 {
            let inner = &stmts_body.iter().next().unwrap();
            return is_assignment_node(inner);
        }
    }
    // Or it could be a direct assignment node
    is_assignment_node(&body)
}

fn is_multiline_paren(source: &SourceFile, paren: &ruby_prism::ParenthesesNode<'_>) -> bool {
    let open_loc = paren.opening_loc();
    let close_loc = paren.closing_loc();
    let (open_line, _) = source.offset_to_line_col(open_loc.start_offset());
    let (close_line, _) = source.offset_to_line_col(close_loc.start_offset());
    open_line != close_line
}

fn is_assignment_node(node: &ruby_prism::Node<'_>) -> bool {
    node.as_local_variable_write_node().is_some()
        || node.as_instance_variable_write_node().is_some()
        || node.as_class_variable_write_node().is_some()
        || node.as_global_variable_write_node().is_some()
        || node.as_constant_write_node().is_some()
        || node.as_multi_write_node().is_some()
        || is_setter_call(node)
}

/// Check if a node is a setter method call (e.g., `obj.attr = value`).
/// RuboCop treats these as safe assignments in parenthesized conditions.
fn is_setter_call(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name();
        let s = name.as_slice();
        // Setter methods end with `=` but are not ==, !=, <=, >=, <=>
        s.len() >= 2
            && s.last() == Some(&b'=')
            && s != b"=="
            && s != b"!="
            && s != b"<="
            && s != b">="
            && s != b"<=>"
    } else {
        false
    }
}

impl Cop for ParenthesesAroundCondition {
    fn name(&self) -> &'static str {
        "Style/ParenthesesAroundCondition"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_VARIABLE_WRITE_NODE, CONSTANT_WRITE_NODE, GLOBAL_VARIABLE_WRITE_NODE, IF_NODE, INSTANCE_VARIABLE_WRITE_NODE, LOCAL_VARIABLE_WRITE_NODE, MULTI_WRITE_NODE, PARENTHESES_NODE, STATEMENTS_NODE, UNLESS_NODE, UNTIL_NODE, WHILE_NODE]
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
        let allow_safe_assignment = config.get_bool("AllowSafeAssignment", true);
        let allow_multiline = config.get_bool("AllowInMultilineConditions", false);

        if let Some(if_node) = node.as_if_node() {
            // Must have `if` keyword (not ternary)
            let kw_loc = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return,
            };

            if let Some(paren) = if_node.predicate().as_parentheses_node() {
                if allow_safe_assignment && is_safe_assignment(&paren) {
                    return;
                }
                if allow_multiline && is_multiline_paren(source, &paren) {
                    return;
                }
                let keyword = if kw_loc.as_slice() == b"unless" {
                    "unless"
                } else {
                    "if"
                };
                let open_loc = paren.opening_loc();
                let (line, column) = source.offset_to_line_col(open_loc.start_offset());
                diagnostics.push(self.diagnostic(source, line, column, format!(
                    "Don't use parentheses around the condition of an `{keyword}`."
                )));
            }
        } else if let Some(unless_node) = node.as_unless_node() {
            if let Some(paren) = unless_node.predicate().as_parentheses_node() {
                if allow_safe_assignment && is_safe_assignment(&paren) {
                    return;
                }
                if allow_multiline && is_multiline_paren(source, &paren) {
                    return;
                }
                let open_loc = paren.opening_loc();
                let (line, column) = source.offset_to_line_col(open_loc.start_offset());
                diagnostics.push(self.diagnostic(source, line, column,
                    "Don't use parentheses around the condition of an `unless`.".to_string()));
            }
        } else if let Some(while_node) = node.as_while_node() {
            if let Some(paren) = while_node.predicate().as_parentheses_node() {
                if allow_safe_assignment && is_safe_assignment(&paren) {
                    return;
                }
                if allow_multiline && is_multiline_paren(source, &paren) {
                    return;
                }
                let open_loc = paren.opening_loc();
                let (line, column) = source.offset_to_line_col(open_loc.start_offset());
                diagnostics.push(self.diagnostic(source, line, column, "Don't use parentheses around the condition of a `while`.".to_string()));
            }
        } else if let Some(until_node) = node.as_until_node() {
            if let Some(paren) = until_node.predicate().as_parentheses_node() {
                if allow_safe_assignment && is_safe_assignment(&paren) {
                    return;
                }
                if allow_multiline && is_multiline_paren(source, &paren) {
                    return;
                }
                let open_loc = paren.opening_loc();
                let (line, column) = source.offset_to_line_col(open_loc.start_offset());
                diagnostics.push(self.diagnostic(source, line, column, "Don't use parentheses around the condition of an `until`.".to_string()));
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{run_cop_full_with_config, run_cop_full};

    crate::cop_fixture_tests!(ParenthesesAroundCondition, "cops/style/parentheses_around_condition");

    #[test]
    fn allow_multiline_conditions() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("AllowInMultilineConditions".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        // Multiline condition in parens should be allowed
        let source = b"if (x > 10 &&\n   y > 10)\n  puts 'hi'\nend\n";
        let diags = run_cop_full_with_config(&ParenthesesAroundCondition, source, config);
        assert!(diags.is_empty(), "Should allow multiline conditions in parens");
    }

    #[test]
    fn still_flags_single_line_with_allow_multiline() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("AllowInMultilineConditions".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        // Single-line parens should still be flagged
        let source = b"if (x > 10)\n  puts 'hi'\nend\n";
        let diags = run_cop_full_with_config(&ParenthesesAroundCondition, source, config);
        assert_eq!(diags.len(), 1, "Should still flag single-line parens");
    }

    #[test]
    fn flags_multiline_by_default() {
        // Multiline parens should be flagged with default config
        let source = b"if (x > 10 &&\n   y > 10)\n  puts 'hi'\nend\n";
        let diags = run_cop_full(&ParenthesesAroundCondition, source);
        assert_eq!(diags.len(), 1, "Should flag multiline parens by default");
    }
}
