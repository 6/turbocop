use crate::cop::node_type::{
    AND_NODE, CALL_NODE, ELSE_NODE, FALSE_NODE, IF_NODE, OR_NODE, PARENTHESES_NODE, TRUE_NODE,
    UNLESS_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct IfWithBooleanLiteralBranches;

impl Cop for IfWithBooleanLiteralBranches {
    fn name(&self) -> &'static str {
        "Style/IfWithBooleanLiteralBranches"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            AND_NODE,
            CALL_NODE,
            ELSE_NODE,
            FALSE_NODE,
            IF_NODE,
            OR_NODE,
            PARENTHESES_NODE,
            TRUE_NODE,
            UNLESS_NODE,
        ]
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
        let allowed_methods = config.get_string_array("AllowedMethods");

        // Check `if` nodes (including ternary)
        if let Some(if_node) = node.as_if_node() {
            // Detect ternary: no if_keyword_loc means it's a ternary
            let is_ternary = if_node.if_keyword_loc().is_none();

            if !is_ternary {
                let kw_text = if_node.if_keyword_loc().unwrap().as_slice();
                // Must be `if`, not `elsif`
                if kw_text != b"if" {
                    return;
                }
            }

            // Need both branches (if body and else)
            let if_body = match if_node.statements() {
                Some(s) => s,
                None => return,
            };
            let else_clause = match if_node.subsequent() {
                Some(s) => s,
                None => return,
            };

            // Must be a simple else (not elsif)
            let else_node = match else_clause.as_else_node() {
                Some(e) => e,
                None => return, // it's an elsif
            };

            // Check if both branches are single boolean literals
            let if_bool = single_boolean_value(&if_body);
            let else_bool = single_boolean_value_from_else(&else_node);

            if let (Some(if_val), Some(else_val)) = (if_bool, else_bool) {
                // Both branches are boolean literals
                if (if_val && !else_val) || (!if_val && else_val) {
                    // Check if condition is known to return boolean.
                    // Applies to both ternary and if/unless forms (matching RuboCop).
                    if !condition_returns_boolean(&if_node.predicate(), &allowed_methods) {
                        return;
                    }

                    if is_ternary {
                        // For ternary, point at the `?`
                        // Find the ? position - it's after the predicate
                        let pred_end = if_node.predicate().location().start_offset()
                            + if_node.predicate().location().as_slice().len();
                        let src = source.as_bytes();
                        let mut q_offset = pred_end;
                        while q_offset < src.len() && src[q_offset] != b'?' {
                            q_offset += 1;
                        }
                        let (line, column) = source.offset_to_line_col(q_offset);
                        diagnostics.push(
                            self.diagnostic(
                                source,
                                line,
                                column,
                                "Remove redundant ternary operator with boolean literal branches."
                                    .to_string(),
                            ),
                        );
                        return;
                    }

                    let if_kw_loc = if_node.if_keyword_loc().unwrap();
                    let (line, column) = source.offset_to_line_col(if_kw_loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Remove redundant `if` with boolean literal branches.".to_string(),
                    ));
                }
            }

            return;
        }

        // Check `unless` nodes
        if let Some(unless_node) = node.as_unless_node() {
            let kw_loc = unless_node.keyword_loc();
            if kw_loc.as_slice() != b"unless" {
                return;
            }

            let unless_body = match unless_node.statements() {
                Some(s) => s,
                None => return,
            };
            let else_clause = match unless_node.else_clause() {
                Some(e) => e,
                None => return,
            };

            let unless_bool = single_boolean_value(&unless_body);
            let else_bool = single_boolean_value_from_else(&else_clause);

            if let (Some(unless_val), Some(else_val)) = (unless_bool, else_bool) {
                if (unless_val && !else_val) || (!unless_val && else_val) {
                    if !condition_returns_boolean(&unless_node.predicate(), &allowed_methods) {
                        return;
                    }

                    let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Remove redundant `unless` with boolean literal branches.".to_string(),
                    ));
                }
            }
        }
    }
}

/// Extract a single boolean literal value from a statements node.
fn single_boolean_value(stmts: &ruby_prism::StatementsNode<'_>) -> Option<bool> {
    let nodes: Vec<_> = stmts.body().into_iter().collect();
    if nodes.len() != 1 {
        return None;
    }
    if nodes[0].as_true_node().is_some() {
        Some(true)
    } else if nodes[0].as_false_node().is_some() {
        Some(false)
    } else {
        None
    }
}

/// Extract a single boolean literal value from an else node.
fn single_boolean_value_from_else(else_node: &ruby_prism::ElseNode<'_>) -> Option<bool> {
    let stmts = else_node.statements()?;
    single_boolean_value(&stmts)
}

/// Check if a condition expression is known to return a boolean value.
/// This includes comparison operators and predicate methods (ending with `?`).
fn condition_returns_boolean(
    node: &ruby_prism::Node<'_>,
    allowed_methods: &Option<Vec<String>>,
) -> bool {
    // Check for call node (comparison or predicate)
    if let Some(call) = node.as_call_node() {
        let method_name = call.name();
        let method_bytes = method_name.as_slice();

        // Check AllowedMethods
        if let Some(allowed) = allowed_methods {
            if let Ok(name_str) = std::str::from_utf8(method_bytes) {
                if allowed.iter().any(|m| m == name_str) {
                    return false; // Allowed methods are excluded from detection
                }
            }
        }

        // Comparison operators
        if method_bytes == b"=="
            || method_bytes == b"!="
            || method_bytes == b"<"
            || method_bytes == b">"
            || method_bytes == b"<="
            || method_bytes == b">="
            || method_bytes == b"==="
            || method_bytes == b"<=>"
            || method_bytes == b"=~"
            || method_bytes == b"!~"
        {
            return true;
        }

        // Predicate methods (ending with ?)
        if method_bytes.ends_with(b"?") {
            return true;
        }

        // Negation operator `!`
        if method_bytes == b"!" {
            return true;
        }
    }

    // Check for `and` / `or` / `&&` / `||`
    // For `&&` (and): only check the RIGHT operand (matches RuboCop behavior).
    // e.g., `foo? && bar && baz?` is flagged because RHS `baz?` is boolean.
    // For `||` (or): check BOTH operands.
    // e.g., `foo? || bar` is NOT flagged because `bar` is not boolean.
    if let Some(and_node) = node.as_and_node() {
        return condition_returns_boolean(&and_node.right(), allowed_methods);
    }
    if let Some(or_node) = node.as_or_node() {
        return condition_returns_boolean(&or_node.left(), allowed_methods)
            && condition_returns_boolean(&or_node.right(), allowed_methods);
    }

    // Parenthesized expression
    if let Some(parens) = node.as_parentheses_node() {
        if let Some(body) = parens.body() {
            return condition_returns_boolean(&body, allowed_methods);
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        IfWithBooleanLiteralBranches,
        "cops/style/if_with_boolean_literal_branches"
    );
}
