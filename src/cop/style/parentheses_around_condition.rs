use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

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

fn is_assignment_node(node: &ruby_prism::Node<'_>) -> bool {
    node.as_local_variable_write_node().is_some()
        || node.as_instance_variable_write_node().is_some()
        || node.as_class_variable_write_node().is_some()
        || node.as_global_variable_write_node().is_some()
        || node.as_constant_write_node().is_some()
        || node.as_multi_write_node().is_some()
}

impl Cop for ParenthesesAroundCondition {
    fn name(&self) -> &'static str {
        "Style/ParenthesesAroundCondition"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allow_safe_assignment = config.get_bool("AllowSafeAssignment", true);

        if let Some(if_node) = node.as_if_node() {
            // Must have `if` keyword (not ternary)
            let kw_loc = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return Vec::new(),
            };

            if let Some(paren) = if_node.predicate().as_parentheses_node() {
                if allow_safe_assignment && is_safe_assignment(&paren) {
                    return Vec::new();
                }
                let keyword = if kw_loc.as_slice() == b"unless" {
                    "unless"
                } else {
                    "if"
                };
                let open_loc = paren.opening_loc();
                let (line, column) = source.offset_to_line_col(open_loc.start_offset());
                return vec![self.diagnostic(source, line, column, format!(
                    "Don't use parentheses around the condition of an `{keyword}`."
                ))];
            }
        } else if let Some(while_node) = node.as_while_node() {
            if let Some(paren) = while_node.predicate().as_parentheses_node() {
                if allow_safe_assignment && is_safe_assignment(&paren) {
                    return Vec::new();
                }
                let open_loc = paren.opening_loc();
                let (line, column) = source.offset_to_line_col(open_loc.start_offset());
                return vec![self.diagnostic(source, line, column, "Don't use parentheses around the condition of a `while`.".to_string())];
            }
        } else if let Some(until_node) = node.as_until_node() {
            if let Some(paren) = until_node.predicate().as_parentheses_node() {
                if allow_safe_assignment && is_safe_assignment(&paren) {
                    return Vec::new();
                }
                let open_loc = paren.opening_loc();
                let (line, column) = source.offset_to_line_col(open_loc.start_offset());
                return vec![self.diagnostic(source, line, column, "Don't use parentheses around the condition of an `until`.".to_string())];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(ParenthesesAroundCondition, "cops/style/parentheses_around_condition");
}
