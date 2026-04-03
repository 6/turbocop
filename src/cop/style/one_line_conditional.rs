use crate::cop::shared::node_type::{IF_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Detects single-line `if/then/else/end` and `unless/then/else/end` constructs.
///
/// Fixed: was requiring `then` keyword, missing semicolon-delimited forms like
/// `if foo; bar else baz end`. Removed the `then_keyword_loc` check to match
/// RuboCop which only requires single-line + else branch, not `then`.
///
/// Also: RuboCop skips empty else bodies (`if x; y; else; end`) because
/// `node.else_branch` is nil. Added corresponding check on ElseNode#statements.
///
/// Also fixed false positives where Prism represents RuboCop-exempt then-bodies
/// as either multiple statements (`if x then a; b else c end`) or a single
/// parenthesized expression (`if x then (a) else b end`).
pub struct OneLineConditional;

impl Cop for OneLineConditional {
    fn name(&self) -> &'static str {
        "Style/OneLineConditional"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, UNLESS_NODE]
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
        // AlwaysCorrectToMultiline only affects auto-correction (ternary vs multiline),
        // not detection. Read it to satisfy config completeness.
        let _always_multiline = config.get_bool("AlwaysCorrectToMultiline", false);
        // Check `if ... then ... else ... end` on one line
        if let Some(if_node) = node.as_if_node() {
            let kw_loc = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return, // ternary
            };

            let kw_bytes = kw_loc.as_slice();
            if kw_bytes != b"if" {
                return;
            }

            // Must not be modifier form
            if if_node.end_keyword_loc().is_none() {
                return;
            }

            // Must have an else branch with content (RuboCop's `node.else_branch`
            // returns nil for empty else bodies like `if x; y; else; end`)
            match if_node.subsequent() {
                None => return,
                Some(sub) => {
                    if let Some(else_node) = sub.as_else_node() {
                        if !branch_has_content(else_node.statements()) {
                            return;
                        }
                    }
                }
            }

            if exempt_then_branch(if_node.statements()) {
                return;
            }

            // Must be single-line
            let loc = if_node.location();
            let (start_line, _) = source.offset_to_line_col(loc.start_offset());
            let (end_line, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));
            if start_line != end_line {
                return;
            }

            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Favor the ternary operator (`?:`) over single-line `if/then/else/end` constructs.".to_string(),
            ));
        }

        // Check `unless ... then ... else ... end` on one line
        if let Some(unless_node) = node.as_unless_node() {
            let kw_loc = unless_node.keyword_loc();
            if kw_loc.as_slice() != b"unless" {
                return;
            }

            // Must not be modifier form
            if unless_node.end_keyword_loc().is_none() {
                return;
            }

            // Must have an else branch with content
            let Some(else_clause) = unless_node.else_clause() else {
                return;
            };
            if !branch_has_content(else_clause.statements()) {
                return;
            }

            if exempt_then_branch(unless_node.statements()) {
                return;
            }

            // Must be single-line
            let loc = unless_node.location();
            let (start_line, _) = source.offset_to_line_col(loc.start_offset());
            let (end_line, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));
            if start_line != end_line {
                return;
            }

            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Favor the ternary operator (`?:`) over single-line `unless/then/else/end` constructs.".to_string(),
            ));
        }
    }
}

fn branch_has_content(statements: Option<ruby_prism::StatementsNode<'_>>) -> bool {
    statements.is_some_and(|statements| !statements.body().is_empty())
}

fn exempt_then_branch(statements: Option<ruby_prism::StatementsNode<'_>>) -> bool {
    let Some(statements) = statements else {
        return false;
    };

    let body = statements.body();
    if body.len() > 1 {
        return true;
    }

    body.first()
        .is_some_and(|statement| statement.as_parentheses_node().is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OneLineConditional, "cops/style/one_line_conditional");
}
