use crate::cop::node_type::{BLOCK_ARGUMENT_NODE, CALL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=0, FN=3.
///
/// FP=0: previous false positives in heredoc-heavy calls were fixed by
/// recursing into nested call arguments, keyword hashes, and assoc values when
/// checking whether the last argument contains a conflicting heredoc.
///
/// FN=3: this cop previously skipped brace-layout checks when *any* argument
/// contained a heredoc. RuboCop only skips when the *last* argument contains a
/// heredoc terminator that forces the closing parenthesis placement. Narrowing
/// the skip to the last argument fixes heredoc-first calls like
/// `foo(<<~EOS, arg ... ).call`.
///
/// ## Corpus investigation (2026-03-29)
///
/// FN=2: outer calls like `wrapper(Hash.from_xml(<<-XML ... XML ))` were still
/// skipped because the last argument contained a nested heredoc somewhere in
/// its subtree. RuboCop only skips when that descendant heredoc reaches the
/// last line of the last-argument node itself. Nested calls whose own closing
/// `)` lands after the heredoc terminator must still be checked.
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

        let last_arg = arg_list.last().unwrap();
        if last_line_heredoc(source, last_arg) {
            return;
        }

        let (open_line, _) = source.offset_to_line_col(opening.start_offset());
        let (close_line, close_col) = source.offset_to_line_col(closing.start_offset());

        // Only check multiline calls (opening paren to closing paren)
        if open_line == close_line {
            return;
        }

        let first_arg = &arg_list[0];

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

fn last_line_heredoc(source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
    use ruby_prism::Visit;

    struct LastLineHeredocDetector<'a> {
        source: &'a SourceFile,
        parent_last_line: usize,
        found: bool,
    }

    impl LastLineHeredocDetector<'_> {
        fn visit_heredoc<'pr>(
            &mut self,
            opening: Option<ruby_prism::Location<'pr>>,
            closing: Option<ruby_prism::Location<'pr>>,
        ) {
            let Some(opening) = opening else {
                return;
            };
            if self.found || !opening.as_slice().starts_with(b"<<") {
                return;
            }
            let Some(closing) = closing else {
                return;
            };

            let end_off = closing
                .end_offset()
                .saturating_sub(1)
                .max(closing.start_offset());
            let (closing_line, _) = self.source.offset_to_line_col(end_off);
            if closing_line >= self.parent_last_line {
                self.found = true;
            }
        }
    }

    impl<'pr> Visit<'pr> for LastLineHeredocDetector<'_> {
        fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
            self.visit_heredoc(node.opening_loc(), node.closing_loc());
            if !self.found {
                ruby_prism::visit_string_node(self, node);
            }
        }

        fn visit_interpolated_string_node(
            &mut self,
            node: &ruby_prism::InterpolatedStringNode<'pr>,
        ) {
            self.visit_heredoc(node.opening_loc(), node.closing_loc());
            if !self.found {
                ruby_prism::visit_interpolated_string_node(self, node);
            }
        }
    }

    let parent_last_line = node_last_line(source, node);
    let mut detector = LastLineHeredocDetector {
        source,
        parent_last_line,
        found: false,
    };
    detector.visit(node);
    detector.found
}

fn node_last_line(source: &SourceFile, node: &ruby_prism::Node<'_>) -> usize {
    let loc = node.location();
    let end_off = loc.end_offset().saturating_sub(1).max(loc.start_offset());
    source.offset_to_line_col(end_off).0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        MultilineMethodCallBraceLayout,
        "cops/layout/multiline_method_call_brace_layout"
    );

    #[test]
    fn heredoc_only_in_earlier_argument_still_checks_brace_layout() {
        let source = br#"foo(<<~EOS, arg
  text
EOS
).do_something
"#;
        let diagnostics = run_cop_full(&MultilineMethodCallBraceLayout, source);
        assert_eq!(
            diagnostics.len(),
            1,
            "Expected one offense: {diagnostics:?}"
        );
    }
}
