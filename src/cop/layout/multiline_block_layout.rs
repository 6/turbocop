use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MultilineBlockLayout;

impl Cop for MultilineBlockLayout {
    fn name(&self) -> &'static str {
        "Layout/MultilineBlockLayout"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let block_node = match node.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let opening_loc = block_node.opening_loc();
        let closing_loc = block_node.closing_loc();

        let (open_line, _) = source.offset_to_line_col(opening_loc.start_offset());
        let (close_line, _) = source.offset_to_line_col(closing_loc.start_offset());

        // Single line block — no offense
        if open_line == close_line {
            return Vec::new();
        }

        let mut diagnostics = Vec::new();

        // Check 1: Block arguments should be on the same line as block start
        if let Some(params) = block_node.parameters() {
            let params_loc = params.location();
            let (params_end_line, _) = source.offset_to_line_col(params_loc.end_offset().saturating_sub(1));
            if params_end_line != open_line {
                // Block params NOT on the same line as `do` or `{`
                let (params_line, params_col) = source.offset_to_line_col(params_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    params_line,
                    params_col,
                    "Block argument expression is not on the same line as the block start.".to_string(),
                ));
            }
        }

        // Check 2: Block body should NOT be on the same line as block start
        if let Some(body) = block_node.body() {
            // When the block contains rescue/ensure, Prism wraps the body in a
            // BeginNode whose location spans from the `do`/`{` keyword — not from
            // the first actual statement.  Unwrap to find the real first expression.
            let first_expr_offset = if let Some(begin_node) = body.as_begin_node() {
                if let Some(stmts) = begin_node.statements() {
                    let children: Vec<ruby_prism::Node<'_>> = stmts.body().iter().collect();
                    children.first().map(|n| n.location().start_offset())
                } else {
                    // No statements before rescue/ensure — use rescue clause location
                    begin_node.rescue_clause().map(|r| r.location().start_offset())
                }
            } else {
                Some(body.location().start_offset())
            };

            if let Some(offset) = first_expr_offset {
                let (body_line, body_col) = source.offset_to_line_col(offset);
                if body_line == open_line {
                    diagnostics.push(self.diagnostic(
                        source,
                        body_line,
                        body_col,
                        "Block body expression is on the same line as the block start.".to_string(),
                    ));
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(MultilineBlockLayout, "cops/layout/multiline_block_layout");
}
