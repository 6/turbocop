use crate::cop::node_type::{INTERPOLATED_STRING_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyHeredoc;

impl Cop for EmptyHeredoc {
    fn name(&self) -> &'static str {
        "Style/EmptyHeredoc"
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
        // Check for heredoc string nodes with empty content
        if let Some(string_node) = node.as_string_node() {
            if let Some(opening) = string_node.opening_loc() {
                let opening_str = std::str::from_utf8(opening.as_slice()).unwrap_or("");
                if opening_str.starts_with("<<") {
                    // It's a heredoc - check if content is empty
                    let content = string_node.content_loc();
                    if content.as_slice().is_empty() {
                        let loc = opening;
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Use an empty string literal instead of heredoc.".to_string(),
                        ));
                    }
                }
            }
        }

        // Also check InterpolatedStringNode for heredocs
        if let Some(interp_node) = node.as_interpolated_string_node() {
            if let Some(opening) = interp_node.opening_loc() {
                let opening_str = std::str::from_utf8(opening.as_slice()).unwrap_or("");
                if opening_str.starts_with("<<") {
                    // It's a heredoc - check if content is empty (no parts)
                    if interp_node.parts().is_empty() {
                        let loc = opening;
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Use an empty string literal instead of heredoc.".to_string(),
                        ));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyHeredoc, "cops/style/empty_heredoc");
}
