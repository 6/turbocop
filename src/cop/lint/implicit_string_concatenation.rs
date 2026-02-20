use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{INTERPOLATED_STRING_NODE, STRING_NODE};

pub struct ImplicitStringConcatenation;

impl Cop for ImplicitStringConcatenation {
    fn name(&self) -> &'static str {
        "Lint/ImplicitStringConcatenation"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[INTERPOLATED_STRING_NODE, STRING_NODE]
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
        let interp = match node.as_interpolated_string_node() {
            Some(n) => n,
            None => return,
        };

        // Skip if this has an opening_loc (heredoc or actual interpolated string literal)
        if interp.opening_loc().is_some() {
            return;
        }

        let parts = interp.parts();
        if parts.len() < 2 {
            return;
        }


        // Check consecutive string parts that are on the same line
        let mut prev: Option<ruby_prism::Node<'_>> = None;
        for part in parts.iter() {
            if let Some(ref prev_node) = prev {
                // Both must be string-like (StringNode or InterpolatedStringNode)
                let prev_is_str = prev_node.as_string_node().is_some()
                    || prev_node.as_interpolated_string_node().is_some();
                let curr_is_str = part.as_string_node().is_some()
                    || part.as_interpolated_string_node().is_some();

                if prev_is_str && curr_is_str {
                    let prev_loc = prev_node.location();
                    let curr_loc = part.location();
                    let (prev_line, _) = source.offset_to_line_col(prev_loc.end_offset().saturating_sub(1));
                    let (curr_line, _) = source.offset_to_line_col(curr_loc.start_offset());

                    // Only flag if on the same line
                    if prev_line == curr_line {
                        // Check that the previous string actually ends with a closing delimiter
                        // (not an embedded newline in one string)
                        let prev_src = &source.as_bytes()[prev_loc.start_offset()..prev_loc.end_offset()];
                        let last_byte = prev_src.last().copied().unwrap_or(0);
                        if last_byte == b'\'' || last_byte == b'"' {
                            let lhs_display = display_str(source, &prev_loc);
                            let rhs_display = display_str(source, &curr_loc);

                            let loc = interp.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                format!(
                                    "Combine {} and {} into a single string literal, rather than using implicit string concatenation.",
                                    lhs_display, rhs_display
                                ),
                            ));
                            break; // One offense per dstr node
                        }
                    }
                }
            }
            prev = Some(part);
        }

    }
}

fn display_str(source: &SourceFile, loc: &ruby_prism::Location<'_>) -> String {
    let bytes = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
    std::str::from_utf8(bytes).unwrap_or("?").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ImplicitStringConcatenation, "cops/lint/implicit_string_concatenation");
}
