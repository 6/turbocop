use crate::cop::node_type::{BLOCK_ARGUMENT_NODE, CALL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MultilineMethodCallBraceLayout;

impl Cop for MultilineMethodCallBraceLayout {
    fn name(&self) -> &'static str {
        "Layout/MultilineMethodCallBraceLayout"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_ARGUMENT_NODE, CALL_NODE]
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
        let enforced_style = config.get_str("EnforcedStyle", "symmetrical");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must have explicit parentheses
        let opening = match call.opening_loc() {
            Some(loc) => loc,
            None => return,
        };
        let closing = match call.closing_loc() {
            Some(loc) => loc,
            None => return,
        };

        if opening.as_slice() != b"(" || closing.as_slice() != b")" {
            return;
        }

        // Must have arguments
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // RuboCop skips the brace layout check when any argument contains a
        // heredoc, because heredoc terminators force unusual line placement for
        // the closing paren. Detect heredoc arguments by checking if the
        // opening location starts with `<<`.
        for arg in &arg_list {
            if is_heredoc_node(arg) {
                return;
            }
        }

        let (open_line, _) = source.offset_to_line_col(opening.start_offset());
        let (close_line, close_col) = source.offset_to_line_col(closing.start_offset());

        // Only check multiline calls (opening paren to closing paren)
        if open_line == close_line {
            return;
        }

        let first_arg = &arg_list[0];
        let last_arg = arg_list.last().unwrap();

        let (first_arg_line, _) = source.offset_to_line_col(first_arg.location().start_offset());

        // Compute the effective end of the last argument. In Prism, `&block`
        // arguments are stored in the CallNode's `block` field, not in the
        // arguments list. For `define_method(method, &lambda do...end)`, the
        // BlockArgumentNode's end offset includes the block's `end`, so use
        // it when present to correctly determine the last arg's line.
        let last_arg_end = if let Some(block) = call.block() {
            if block.as_block_argument_node().is_some() {
                // &block_arg — its span includes the block content
                block.location().end_offset().saturating_sub(1)
            } else {
                // Regular do...end block — `)` comes before the block, not after
                last_arg.location().end_offset().saturating_sub(1)
            }
        } else {
            last_arg.location().end_offset().saturating_sub(1)
        };
        let (last_arg_line, _) = source.offset_to_line_col(last_arg_end);

        let open_same_as_first = open_line == first_arg_line;
        let close_same_as_last = close_line == last_arg_line;

        match enforced_style {
            "symmetrical" => {
                if open_same_as_first && !close_same_as_last {
                    diagnostics.push(self.diagnostic(
                        source,
                        close_line,
                        close_col,
                        "Closing method call brace must be on the same line as the last argument when opening brace is on the same line as the first argument.".to_string(),
                    ));
                }
                if !open_same_as_first && close_same_as_last {
                    diagnostics.push(self.diagnostic(
                        source,
                        close_line,
                        close_col,
                        "Closing method call brace must be on the line after the last argument when opening brace is on a separate line from the first argument.".to_string(),
                    ));
                }
            }
            "new_line" => {
                if close_same_as_last {
                    diagnostics.push(self.diagnostic(
                        source,
                        close_line,
                        close_col,
                        "Closing method call brace must be on the line after the last argument."
                            .to_string(),
                    ));
                }
            }
            "same_line" => {
                if !close_same_as_last {
                    diagnostics.push(self.diagnostic(
                        source,
                        close_line,
                        close_col,
                        "Closing method call brace must be on the same line as the last argument."
                            .to_string(),
                    ));
                }
            }
            _ => {}
        }
    }
}

/// Check if a node is or contains a heredoc string (opening starts with `<<`).
/// Also walks into method call receivers to detect `<<~SQL.tr(...)` patterns
/// where the heredoc is wrapped in a method call.
fn is_heredoc_node(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(s) = node.as_interpolated_string_node() {
        if let Some(open) = s.opening_loc() {
            return open.as_slice().starts_with(b"<<");
        }
    }
    if let Some(s) = node.as_string_node() {
        if let Some(open) = s.opening_loc() {
            return open.as_slice().starts_with(b"<<");
        }
    }
    // Check if this is a method call on a heredoc (e.g., <<~SQL.tr("\n", ""))
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            return is_heredoc_node(&recv);
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        MultilineMethodCallBraceLayout,
        "cops/layout/multiline_method_call_brace_layout"
    );
}
