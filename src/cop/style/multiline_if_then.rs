use crate::cop::shared::node_type::{IF_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

fn body_starts_on_same_line(
    source: &SourceFile,
    statements: Option<ruby_prism::StatementsNode<'_>>,
    then_line: usize,
) -> bool {
    statements
        .and_then(|stmts| stmts.body().into_iter().next())
        .map(|first_body| {
            source
                .offset_to_line_col(first_body.location().start_offset())
                .0
                == then_line
        })
        .unwrap_or(false)
}

/// Prism exposes `if true then ; end` / `unless cond then ; end` as empty-body
/// conditionals whose `then` and `end` share a line. RuboCop still flags those,
/// so only real same-line bodies are exempt from this cop.
pub struct MultilineIfThen;

impl Cop for MultilineIfThen {
    fn name(&self) -> &'static str {
        "Style/MultilineIfThen"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, UNLESS_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
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

            // Table style is only allowed when a real body starts on the same line.
            if body_starts_on_same_line(source, if_node.statements(), then_line) {
                return;
            }

            let keyword_name = if kw_text == b"elsif" { "elsif" } else { "if" };
            let (line, column) = source.offset_to_line_col(then_loc.start_offset());
            let mut diag = self.diagnostic(
                source,
                line,
                column,
                format!("Do not use `then` for multi-line `{}`.", keyword_name),
            );
            // Autocorrect: remove ` then` (including preceding whitespace)
            if let Some(ref mut corr) = corrections {
                let src = source.as_bytes();
                let mut remove_start = then_loc.start_offset();
                while remove_start > 0 && src[remove_start - 1] == b' ' {
                    remove_start -= 1;
                }
                corr.push(crate::correction::Correction {
                    start: remove_start,
                    end: then_loc.end_offset(),
                    replacement: String::new(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
            diagnostics.push(diag);
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

            if body_starts_on_same_line(source, unless_node.statements(), then_line) {
                return;
            }

            let (line, column) = source.offset_to_line_col(then_loc.start_offset());
            let mut diag = self.diagnostic(
                source,
                line,
                column,
                "Do not use `then` for multi-line `unless`.".to_string(),
            );
            if let Some(ref mut corr) = corrections {
                let src = source.as_bytes();
                let mut remove_start = then_loc.start_offset();
                while remove_start > 0 && src[remove_start - 1] == b' ' {
                    remove_start -= 1;
                }
                corr.push(crate::correction::Correction {
                    start: remove_start,
                    end: then_loc.end_offset(),
                    replacement: String::new(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
            diagnostics.push(diag);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultilineIfThen, "cops/style/multiline_if_then");
    crate::cop_autocorrect_fixture_tests!(MultilineIfThen, "cops/style/multiline_if_then");
}
