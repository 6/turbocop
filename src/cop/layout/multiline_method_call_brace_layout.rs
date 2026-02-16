use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MultilineMethodCallBraceLayout;

impl Cop for MultilineMethodCallBraceLayout {
    fn name(&self) -> &'static str {
        "Layout/MultilineMethodCallBraceLayout"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "symmetrical");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must have explicit parentheses
        let opening = match call.opening_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };
        let closing = match call.closing_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        if opening.as_slice() != b"(" || closing.as_slice() != b")" {
            return Vec::new();
        }

        // Must have arguments
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        let (open_line, _) = source.offset_to_line_col(opening.start_offset());
        let (close_line, close_col) = source.offset_to_line_col(closing.start_offset());

        // Only check multiline calls (opening paren to closing paren)
        if open_line == close_line {
            return Vec::new();
        }

        let first_arg = &arg_list[0];
        let last_arg = arg_list.last().unwrap();

        let (first_arg_line, _) = source.offset_to_line_col(first_arg.location().start_offset());
        let (last_arg_line, _) = source.offset_to_line_col(
            last_arg.location().end_offset().saturating_sub(1),
        );

        let open_same_as_first = open_line == first_arg_line;
        let close_same_as_last = close_line == last_arg_line;

        match enforced_style {
            "symmetrical" => {
                if open_same_as_first && !close_same_as_last {
                    return vec![self.diagnostic(
                        source,
                        close_line,
                        close_col,
                        "Closing method call brace must be on the same line as the last argument when opening brace is on the same line as the first argument.".to_string(),
                    )];
                }
                if !open_same_as_first && close_same_as_last {
                    return vec![self.diagnostic(
                        source,
                        close_line,
                        close_col,
                        "Closing method call brace must be on the line after the last argument when opening brace is on a separate line from the first argument.".to_string(),
                    )];
                }
            }
            "new_line" => {
                if close_same_as_last {
                    return vec![self.diagnostic(
                        source,
                        close_line,
                        close_col,
                        "Closing method call brace must be on the line after the last argument."
                            .to_string(),
                    )];
                }
            }
            "same_line" => {
                if !close_same_as_last {
                    return vec![self.diagnostic(
                        source,
                        close_line,
                        close_col,
                        "Closing method call brace must be on the same line as the last argument."
                            .to_string(),
                    )];
                }
            }
            _ => {}
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        MultilineMethodCallBraceLayout,
        "cops/layout/multiline_method_call_brace_layout"
    );
}
