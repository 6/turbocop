use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::DEF_NODE;

pub struct FirstParameterIndentation;

impl Cop for FirstParameterIndentation {
    fn name(&self) -> &'static str {
        "Layout/FirstParameterIndentation"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
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
        let style = config.get_str("EnforcedStyle", "consistent");
        let _indent_width = config.get_usize("IndentationWidth", 2);

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        let lparen_loc = match def_node.lparen_loc() {
            Some(loc) => loc,
            None => return,
        };
        let rparen_loc = match def_node.rparen_loc() {
            Some(loc) => loc,
            None => return,
        };

        let params = match def_node.parameters() {
            Some(p) => p,
            None => return,
        };

        let (open_line, open_col) = source.offset_to_line_col(lparen_loc.start_offset());
        let (close_line, _) = source.offset_to_line_col(rparen_loc.start_offset());

        // Only check multiline parameter lists
        if open_line == close_line {
            return;
        }

        // Find the first parameter
        let requireds: Vec<ruby_prism::Node<'_>> = params.requireds().iter().collect();
        let optionals: Vec<ruby_prism::Node<'_>> =
            params.optionals().iter().collect();

        let first_offset = if !requireds.is_empty() {
            requireds[0].location().start_offset()
        } else if !optionals.is_empty() {
            optionals[0].location().start_offset()
        } else if let Some(rest) = params.rest() {
            rest.location().start_offset()
        } else {
            return;
        };

        let (first_line, first_col) = source.offset_to_line_col(first_offset);

        // Skip if first param is on the same line as the parenthesis
        if first_line == open_line {
            return;
        }

        let def_kw_loc = def_node.def_keyword_loc();
        let def_line_indent = {
            let bytes = source.as_bytes();
            let mut line_start = def_kw_loc.start_offset();
            while line_start > 0 && bytes[line_start - 1] != b'\n' {
                line_start -= 1;
            }
            let mut indent = 0;
            while line_start + indent < bytes.len() && bytes[line_start + indent] == b' ' {
                indent += 1;
            }
            indent
        };

        let width = config.get_usize("IndentationWidth", 2);

        let expected = match style {
            "align_parentheses" => open_col + 1,
            _ => def_line_indent + width, // "consistent"
        };

        if first_col != expected {
            diagnostics.push(self.diagnostic(
                source,
                first_line,
                first_col,
                format!(
                    "Use {} (not {}) spaces for indentation.",
                    expected,
                    first_col
                ),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        FirstParameterIndentation,
        "cops/layout/first_parameter_indentation"
    );
}
