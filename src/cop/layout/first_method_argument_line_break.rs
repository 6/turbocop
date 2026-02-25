use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FirstMethodArgumentLineBreak;

impl Cop for FirstMethodArgumentLineBreak {
    fn name(&self) -> &'static str {
        "Layout/FirstMethodArgumentLineBreak"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let _allow_multiline_final = config.get_bool("AllowMultilineFinalElement", false);
        let _allowed_methods = config.get_string_array("AllowedMethods");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must have parenthesized arguments
        let open_loc = match call.opening_loc() {
            Some(loc) => loc,
            None => return,
        };
        let close_loc = match call.closing_loc() {
            Some(loc) => loc,
            None => return,
        };

        if open_loc.as_slice() != b"(" || close_loc.as_slice() != b")" {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        let (open_line, _) = source.offset_to_line_col(open_loc.start_offset());
        let (close_line, _) = source.offset_to_line_col(close_loc.start_offset());

        // Only check multiline calls
        if open_line == close_line {
            return;
        }

        let first = &arg_list[0];
        let (first_line, first_col) = source.offset_to_line_col(first.location().start_offset());

        if first_line == open_line {
            diagnostics.push(
                self.diagnostic(
                    source,
                    first_line,
                    first_col,
                    "Add a line break before the first argument of a multi-line method call."
                        .to_string(),
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        FirstMethodArgumentLineBreak,
        "cops/layout/first_method_argument_line_break"
    );
}
