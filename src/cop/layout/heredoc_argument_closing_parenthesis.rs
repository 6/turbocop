use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, INTERPOLATED_STRING_NODE, STRING_NODE};

pub struct HeredocArgumentClosingParenthesis;

impl Cop for HeredocArgumentClosingParenthesis {
    fn name(&self) -> &'static str {
        "Layout/HeredocArgumentClosingParenthesis"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, INTERPOLATED_STRING_NODE, STRING_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must have parenthesized call
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

        // Check if any argument is a heredoc
        let mut has_heredoc = false;
        for arg in args.arguments().iter() {
            if is_heredoc_arg(source, &arg) {
                has_heredoc = true;
                break;
            }
        }

        if !has_heredoc {
            return;
        }

        // The closing paren should be right after the heredoc opener line
        let (open_line, _) = source.offset_to_line_col(open_loc.start_offset());
        let (close_line, close_col) = source.offset_to_line_col(close_loc.start_offset());

        // Check if closing paren is on the same line as the call/heredoc opener
        if close_line != open_line {
            // Find the last argument
            let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
            let last_arg = &arg_list[arg_list.len() - 1];
            // For heredoc, the opening tag is on the call line
            let (last_arg_line, _) = source.offset_to_line_col(last_arg.location().start_offset());
            if close_line != last_arg_line {
                diagnostics.push(self.diagnostic(
                    source,
                    close_line,
                    close_col,
                    "Put the closing parenthesis for a method call with a HEREDOC parameter on the same line as the HEREDOC opening.".to_string(),
                ));
            }
        }

    }
}

fn is_heredoc_arg(source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
    let bytes = source.as_bytes();
    // InterpolatedStringNode with heredoc opening
    if let Some(istr) = node.as_interpolated_string_node() {
        if let Some(opening) = istr.opening_loc() {
            let slice = &bytes[opening.start_offset()..opening.end_offset()];
            if slice.starts_with(b"<<") {
                return true;
            }
        }
    }
    // StringNode with heredoc opening
    if let Some(str_node) = node.as_string_node() {
        if let Some(opening) = str_node.opening_loc() {
            let slice = &bytes[opening.start_offset()..opening.end_offset()];
            if slice.starts_with(b"<<") {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        HeredocArgumentClosingParenthesis,
        "cops/layout/heredoc_argument_closing_parenthesis"
    );
}
