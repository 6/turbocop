use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::BEGIN_NODE;

pub struct BeginEndAlignment;

impl Cop for BeginEndAlignment {
    fn name(&self) -> &'static str {
        "Layout/BeginEndAlignment"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BEGIN_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyleAlignWith", "start_of_line");

        let begin_node = match node.as_begin_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let begin_kw_loc = match begin_node.begin_keyword_loc() {
            Some(loc) => loc,
            None => return Vec::new(), // Implicit begin (method body) — skip
        };

        let end_kw_loc = match begin_node.end_keyword_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let (begin_line, begin_col) = source.offset_to_line_col(begin_kw_loc.start_offset());
        let (end_line, end_col) = source.offset_to_line_col(end_kw_loc.start_offset());

        // Skip single-line begin..end
        if begin_line == end_line {
            return Vec::new();
        }

        let expected_col = match style {
            "begin" => begin_col,
            _ => {
                // "start_of_line" (default): align with the start of the line
                let bytes = source.as_bytes();
                let mut line_start = begin_kw_loc.start_offset();
                while line_start > 0 && bytes[line_start - 1] != b'\n' {
                    line_start -= 1;
                }
                let mut indent = 0;
                while line_start + indent < bytes.len() && bytes[line_start + indent] == b' ' {
                    indent += 1;
                }
                indent
            }
        };

        if end_col != expected_col {
            return vec![self.diagnostic(
                source,
                end_line,
                end_col,
                "`end` at 0, 0 is not aligned with `begin` at 0, 0.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(BeginEndAlignment, "cops/layout/begin_end_alignment");
}
