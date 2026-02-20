use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{IF_NODE, UNLESS_NODE};

pub struct MultilineIfThen;

impl Cop for MultilineIfThen {
    fn name(&self) -> &'static str {
        "Style/MultilineIfThen"
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
        // Handle `if ... then` (multi-line)
        if let Some(if_node) = node.as_if_node() {
            // Must have an `if` keyword (not ternary)
            let if_kw_loc = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return,
            };

            let kw_text = if_kw_loc.as_slice();

            // Must be `if` or `elsif`, not `unless`
            if kw_text != b"if" && kw_text != b"elsif" {
                return;
            }

            // Check for `then` keyword
            let then_loc = match if_node.then_keyword_loc() {
                Some(loc) => loc,
                None => return,
            };

            if then_loc.as_slice() != b"then" {
                return;
            }

            // Check if this is a multiline if (then and body/end are on different lines)
            let then_line = source.offset_to_line_col(then_loc.start_offset()).0;

            // For "table style" if/then/elsif/end, all on one line - allow it
            // Check if the body is on the same line as `then`
            if let Some(stmts) = if_node.statements() {
                let body_nodes: Vec<_> = stmts.body().into_iter().collect();
                if !body_nodes.is_empty() {
                    let first_body_line = source.offset_to_line_col(body_nodes[0].location().start_offset()).0;
                    if first_body_line == then_line {
                        // Table style: `if cond then body` all on same line
                        return;
                    }
                }
            } else {
                // No body statements. Check if end is on the same line as then.
                if let Some(end_loc) = if_node.end_keyword_loc() {
                    let end_line = source.offset_to_line_col(end_loc.start_offset()).0;
                    if end_line == then_line {
                        return;
                    }
                }
                // If there's a subsequent (elsif/else) on same line, it's table style
                if let Some(sub) = if_node.subsequent() {
                    let sub_line = source.offset_to_line_col(sub.location().start_offset()).0;
                    if sub_line == then_line {
                        return;
                    }
                }
            }

            let keyword_name = if kw_text == b"elsif" { "elsif" } else { "if" };
            let (line, column) = source.offset_to_line_col(then_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Do not use `then` for multi-line `{}`.", keyword_name),
            ));
        }

        // Handle `unless ... then` (multi-line)
        if let Some(unless_node) = node.as_unless_node() {
            let then_loc = match unless_node.then_keyword_loc() {
                Some(loc) => loc,
                None => return,
            };

            if then_loc.as_slice() != b"then" {
                return;
            }

            let then_line = source.offset_to_line_col(then_loc.start_offset()).0;

            // Check for table style (body on same line as then)
            if let Some(stmts) = unless_node.statements() {
                let body_nodes: Vec<_> = stmts.body().into_iter().collect();
                if !body_nodes.is_empty() {
                    let first_body_line = source.offset_to_line_col(body_nodes[0].location().start_offset()).0;
                    if first_body_line == then_line {
                        return;
                    }
                }
            } else {
                if let Some(end_loc) = unless_node.end_keyword_loc() {
                    let end_line = source.offset_to_line_col(end_loc.start_offset()).0;
                    if end_line == then_line {
                        return;
                    }
                }
            }

            let (line, column) = source.offset_to_line_col(then_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Do not use `then` for multi-line `unless`.".to_string(),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultilineIfThen, "cops/style/multiline_if_then");
}
