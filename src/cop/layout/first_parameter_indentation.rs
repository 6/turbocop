use crate::cop::node_type::DEF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

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

        // Find the first parameter by earliest start offset across all param types
        let mut first_offset: Option<usize> = None;
        let mut update_min = |offset: usize| {
            first_offset = Some(match first_offset {
                Some(cur) if cur <= offset => cur,
                _ => offset,
            });
        };

        if let Some(first) = params.requireds().iter().next() {
            update_min(first.location().start_offset());
        }
        if let Some(first) = params.optionals().iter().next() {
            update_min(first.location().start_offset());
        }
        if let Some(rest) = params.rest() {
            update_min(rest.location().start_offset());
        }
        if let Some(first) = params.posts().iter().next() {
            update_min(first.location().start_offset());
        }
        if let Some(first) = params.keywords().iter().next() {
            update_min(first.location().start_offset());
        }
        if let Some(kw_rest) = params.keyword_rest() {
            update_min(kw_rest.location().start_offset());
        }
        if let Some(block) = params.block() {
            update_min(block.location().start_offset());
        }

        let first_offset = match first_offset {
            Some(o) => o,
            None => return,
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
            "align_parentheses" => open_col + width,
            _ => def_line_indent + width, // "consistent"
        };

        if first_col != expected {
            diagnostics.push(self.diagnostic(
                source,
                first_line,
                first_col,
                format!(
                    "Use {} (not {}) spaces for indentation.",
                    expected, first_col
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
