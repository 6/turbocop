use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct MultilineMethodArgumentLineBreaks;

impl Cop for MultilineMethodArgumentLineBreaks {
    fn name(&self) -> &'static str {
        "Layout/MultilineMethodArgumentLineBreaks"
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
    ) -> Vec<Diagnostic> {
        let _allow_multiline_final = config.get_bool("AllowMultilineFinalElement", false);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let open_loc = match call.opening_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };
        let close_loc = match call.closing_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        if open_loc.as_slice() != b"(" || close_loc.as_slice() != b")" {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let (open_line, _) = source.offset_to_line_col(open_loc.start_offset());
        let (close_line, _) = source.offset_to_line_col(close_loc.start_offset());

        // Only check multiline calls
        if open_line == close_line {
            return Vec::new();
        }

        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
        if arg_list.len() < 2 {
            return Vec::new();
        }

        let mut diagnostics = Vec::new();

        for i in 1..arg_list.len() {
            let prev = &arg_list[i - 1];
            let curr = &arg_list[i];

            let (prev_line, _) = source.offset_to_line_col(
                prev.location().end_offset().saturating_sub(1),
            );
            let (curr_line, curr_col) = source.offset_to_line_col(curr.location().start_offset());

            if prev_line == curr_line {
                diagnostics.push(self.diagnostic(
                    source,
                    curr_line,
                    curr_col,
                    "Each argument in a multi-line method call must start on a separate line."
                        .to_string(),
                ));
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        MultilineMethodArgumentLineBreaks,
        "cops/layout/multiline_method_argument_line_breaks"
    );
}
