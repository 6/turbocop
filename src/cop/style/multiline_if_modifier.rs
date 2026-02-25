use crate::cop::node_type::{IF_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MultilineIfModifier;

/// Check if all physical lines from `from_line` to `to_line - 1` (1-indexed)
/// end with a backslash continuation character. If so, the expression is a
/// single logical line and should not be considered multiline.
fn lines_joined_by_backslash(source: &SourceFile, from_line: usize, to_line: usize) -> bool {
    if from_line >= to_line {
        return false;
    }
    let lines: Vec<&[u8]> = source.lines().collect();
    for line_num in from_line..to_line {
        // line_num is 1-indexed, lines vec is 0-indexed
        let idx = line_num - 1;
        if idx >= lines.len() {
            return false;
        }
        let line = lines[idx];
        let trimmed = line
            .iter()
            .rposition(|&b| b != b' ' && b != b'\t' && b != b'\r');
        match trimmed {
            Some(pos) if line[pos] == b'\\' => {}
            _ => return false,
        }
    }
    true
}

impl Cop for MultilineIfModifier {
    fn name(&self) -> &'static str {
        "Style/MultilineIfModifier"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, UNLESS_NODE]
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
        // Check `if` modifier form
        if let Some(if_node) = node.as_if_node() {
            let if_kw_loc = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return,
            };

            if if_kw_loc.as_slice() != b"if" {
                return;
            }

            // Must be modifier form (no end keyword)
            if if_node.end_keyword_loc().is_some() {
                return;
            }

            // Check if the body spans multiple lines
            if let Some(stmts) = if_node.statements() {
                let body_nodes: Vec<_> = stmts.body().into_iter().collect();
                if body_nodes.is_empty() {
                    return;
                }

                let body_start_line = source
                    .offset_to_line_col(body_nodes[0].location().start_offset())
                    .0;
                let if_kw_line = source.offset_to_line_col(if_kw_loc.start_offset()).0;

                if body_start_line < if_kw_line {
                    // Skip if lines are joined by backslash continuation (single logical line)
                    if lines_joined_by_backslash(source, body_start_line, if_kw_line) {
                        return;
                    }
                    // Body starts before the `if` keyword - it's multiline
                    let body_start = body_nodes[0].location().start_offset();
                    let (line, column) = source.offset_to_line_col(body_start);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Favor a normal if-statement over a modifier clause in a multiline statement.".to_string(),
                    ));
                }
            }

            return;
        }

        // Check `unless` modifier form
        if let Some(unless_node) = node.as_unless_node() {
            let kw_loc = unless_node.keyword_loc();

            if kw_loc.as_slice() != b"unless" {
                return;
            }

            // Must be modifier form (no end keyword)
            if unless_node.end_keyword_loc().is_some() {
                return;
            }

            // Check if the body spans multiple lines
            if let Some(stmts) = unless_node.statements() {
                let body_nodes: Vec<_> = stmts.body().into_iter().collect();
                if body_nodes.is_empty() {
                    return;
                }

                let body_start_line = source
                    .offset_to_line_col(body_nodes[0].location().start_offset())
                    .0;
                let kw_line = source.offset_to_line_col(kw_loc.start_offset()).0;

                if body_start_line < kw_line {
                    // Skip if lines are joined by backslash continuation (single logical line)
                    if lines_joined_by_backslash(source, body_start_line, kw_line) {
                        return;
                    }
                    let body_start = body_nodes[0].location().start_offset();
                    let (line, column) = source.offset_to_line_col(body_start);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Favor a normal unless-statement over a modifier clause in a multiline statement.".to_string(),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultilineIfModifier, "cops/style/multiline_if_modifier");
}
