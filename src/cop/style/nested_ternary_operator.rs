use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ELSE_NODE, IF_NODE, PARENTHESES_NODE, STATEMENTS_NODE};

pub struct NestedTernaryOperator;

/// Check if an IfNode is a ternary operator (no if_keyword_loc in Prism)
fn is_ternary(if_node: &ruby_prism::IfNode<'_>) -> bool {
    if_node.if_keyword_loc().is_none()
}

/// Find ternary nodes within a node, recursing into parentheses
fn find_nested_ternary(node: &ruby_prism::Node<'_>, source: &SourceFile) -> Vec<(usize, usize)> {
    let mut results = Vec::new();
    if let Some(if_node) = node.as_if_node() {
        if is_ternary(&if_node) {
            let loc = if_node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            results.push((line, column));
        }
    }
    // Recurse into parentheses
    if let Some(paren) = node.as_parentheses_node() {
        if let Some(body) = paren.body() {
            if let Some(stmts) = body.as_statements_node() {
                for stmt in stmts.body().iter() {
                    results.extend(find_nested_ternary(&stmt, source));
                }
            }
        }
    }
    results
}

impl Cop for NestedTernaryOperator {
    fn name(&self) -> &'static str {
        "Style/NestedTernaryOperator"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ELSE_NODE, IF_NODE, PARENTHESES_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return,
        };

        // Must be a ternary
        if !is_ternary(&if_node) {
            return;
        }

        let mut offenses = Vec::new();

        // Check if_branch for nested ternaries
        if let Some(if_branch) = if_node.statements() {
            for stmt in if_branch.body().iter() {
                for (line, column) in find_nested_ternary(&stmt, source) {
                    offenses.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Ternary operators must not be nested. Prefer `if` or `else` constructs instead.".to_string(),
                    ));
                }
            }
        }

        // Check else_branch for nested ternaries
        if let Some(else_clause) = if_node.subsequent() {
            let else_node: ruby_prism::Node<'_> = else_clause.into();
            if let Some(else_n) = else_node.as_else_node() {
                if let Some(stmts) = else_n.statements() {
                    for stmt in stmts.body().iter() {
                        for (line, column) in find_nested_ternary(&stmt, source) {
                            offenses.push(self.diagnostic(
                                source,
                                line,
                                column,
                                "Ternary operators must not be nested. Prefer `if` or `else` constructs instead.".to_string(),
                            ));
                        }
                    }
                }
            }
            // Also check if the subsequent is itself an IfNode (ternary in else position without else keyword)
            if let Some(sub_if) = else_node.as_if_node() {
                if is_ternary(&sub_if) {
                    let loc = sub_if.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    offenses.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Ternary operators must not be nested. Prefer `if` or `else` constructs instead.".to_string(),
                    ));
                }
            }
        }

        diagnostics.extend(offenses);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NestedTernaryOperator, "cops/style/nested_ternary_operator");
}
