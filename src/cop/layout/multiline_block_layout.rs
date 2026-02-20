use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BEGIN_NODE, BLOCK_NODE};

pub struct MultilineBlockLayout;

impl Cop for MultilineBlockLayout {
    fn name(&self) -> &'static str {
        "Layout/MultilineBlockLayout"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BEGIN_NODE, BLOCK_NODE]
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
        let block_node = match node.as_block_node() {
            Some(b) => b,
            None => return,
        };

        let opening_loc = block_node.opening_loc();
        let closing_loc = block_node.closing_loc();

        let (open_line, _) = source.offset_to_line_col(opening_loc.start_offset());
        let (close_line, _) = source.offset_to_line_col(closing_loc.start_offset());

        // Single line block — no offense
        if open_line == close_line {
            return;
        }

        // Check 1: Block arguments should be on the same line as block start
        if let Some(params) = block_node.parameters() {
            let params_loc = params.location();
            let (params_end_line, _) = source.offset_to_line_col(params_loc.end_offset().saturating_sub(1));
            if params_end_line != open_line {
                // Block params NOT on the same line as `do` or `{`.
                // But if fitting all args on one line would exceed max line length,
                // the line break is necessary and acceptable (RuboCop's
                // line_break_necessary_in_args? check).
                let max_len = get_max_line_length(config);

                let line_break_necessary = if let Some(max_len) = max_len {
                    let bytes = source.as_bytes();
                    // Find start of the line containing the block opening
                    let mut line_start = opening_loc.start_offset();
                    while line_start > 0 && bytes[line_start - 1] != b'\n' {
                        line_start -= 1;
                    }
                    // Get the first line content (before params)
                    let first_line_len = opening_loc.end_offset() - line_start;
                    // Get params source and flatten to single line
                    let params_source = &bytes[params_loc.start_offset()..params_loc.end_offset()];
                    let flat_params = flatten_to_single_line(params_source);
                    // Total: first_line + space + | + flat_params + |
                    let needed = first_line_len + 1 + 1 + flat_params.len() + 1;
                    needed > max_len
                } else {
                    false
                };

                if !line_break_necessary {
                    let (params_line, params_col) = source.offset_to_line_col(params_loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        params_line,
                        params_col,
                        "Block argument expression is not on the same line as the block start.".to_string(),
                    ));
                }
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

    }
}

/// Get the max line length from config. Checks for a cross-cop injected
/// MaxLineLength key, falling back to a default of 120.
fn get_max_line_length(config: &CopConfig) -> Option<usize> {
    // Check for explicitly configured MaxLineLength on this cop
    if let Some(val) = config.options.get("MaxLineLength") {
        return val.as_u64().map(|v| v as usize);
    }
    // Default: use 120 (RuboCop's default Layout/LineLength Max)
    Some(120)
}

/// Flatten multiline params to a single line by replacing newlines and
/// collapsing whitespace sequences.
fn flatten_to_single_line(source: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(source.len());
    let mut prev_was_whitespace = false;
    for &b in source {
        if b == b'\n' || b == b'\r' || b == b' ' || b == b'\t' {
            if !prev_was_whitespace && !result.is_empty() {
                result.push(b' ');
            }
            prev_was_whitespace = true;
        } else {
            result.push(b);
            prev_was_whitespace = false;
        }
    }
    // Trim trailing whitespace
    while result.last() == Some(&b' ') {
        result.pop();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(MultilineBlockLayout, "cops/layout/multiline_block_layout");
}
