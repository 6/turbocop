use crate::cop::node_type::{CALL_NODE, ELSE_NODE, FALSE_NODE, IF_NODE, TRUE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantConditional;

impl RedundantConditional {
    /// Check if a node is a comparison operator call
    fn is_comparison(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            let name = call.name();
            let name_bytes = name.as_slice();
            return matches!(
                name_bytes,
                b"==" | b"!=" | b"<" | b">" | b"<=" | b">=" | b"===" | b"<=>" | b"=~" | b"!~"
            );
        }
        false
    }

    fn is_true_literal(node: &ruby_prism::Node<'_>) -> bool {
        node.as_true_node().is_some()
    }

    fn is_false_literal(node: &ruby_prism::Node<'_>) -> bool {
        node.as_false_node().is_some()
    }

    fn single_stmt_is_true(stmts: &ruby_prism::StatementsNode<'_>) -> bool {
        let body: Vec<_> = stmts.body().into_iter().collect();
        body.len() == 1 && Self::is_true_literal(&body[0])
    }

    fn single_stmt_is_false(stmts: &ruby_prism::StatementsNode<'_>) -> bool {
        let body: Vec<_> = stmts.body().into_iter().collect();
        body.len() == 1 && Self::is_false_literal(&body[0])
    }
}

impl Cop for RedundantConditional {
    fn name(&self) -> &'static str {
        "Style/RedundantConditional"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, ELSE_NODE, FALSE_NODE, IF_NODE, TRUE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return,
        };

        let predicate = if_node.predicate();

        // Must be a comparison operator
        if !Self::is_comparison(&predicate) {
            return;
        }

        // Get the then branch statements
        let then_stmts = match if_node.statements() {
            Some(s) => s,
            None => return,
        };

        // Get the else branch
        let else_branch = match if_node.subsequent() {
            Some(n) => n,
            None => return,
        };

        // Else branch must be an ElseNode
        let else_node = match else_branch.as_else_node() {
            Some(e) => e,
            None => return,
        };

        let else_stmts = match else_node.statements() {
            Some(s) => s,
            None => return,
        };

        // Check for `if cond; true; else; false; end` or `if cond; false; else; true; end`
        let then_true_else_false =
            Self::single_stmt_is_true(&then_stmts) && Self::single_stmt_is_false(&else_stmts);
        let then_false_else_true =
            Self::single_stmt_is_false(&then_stmts) && Self::single_stmt_is_true(&else_stmts);

        if !then_true_else_false && !then_false_else_true {
            return;
        }

        let condition_src =
            std::str::from_utf8(predicate.location().as_slice()).unwrap_or("condition");
        let replacement = if then_true_else_false {
            condition_src.to_string()
        } else {
            format!("!({})", condition_src)
        };

        let loc = if_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "This conditional expression can just be replaced by `{}`.",
                replacement
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantConditional, "cops/style/redundant_conditional");
}
