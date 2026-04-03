use crate::cop::shared::node_type::{
    INTERPOLATED_REGULAR_EXPRESSION_NODE, REGULAR_EXPRESSION_NODE, STRING_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=1, FN=1.
///
/// FP=1: the old scanner treated `/x` comments as if they were part of the
/// regexp body, so a `]` inside a comment produced a false positive.
/// FN=1: we skipped interpolated regexps entirely, but RuboCop still flags
/// static `]` characters that appear in literal segments around interpolation.
/// The fix is a stateful raw-source scanner that preserves character-class
/// state across interpolated string segments while ignoring extended-mode
/// comments and whitespace outside character classes.
pub struct UnescapedBracketInRegexp;

impl Cop for UnescapedBracketInRegexp {
    fn name(&self) -> &'static str {
        "Lint/UnescapedBracketInRegexp"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            INTERPOLATED_REGULAR_EXPRESSION_NODE,
            REGULAR_EXPRESSION_NODE,
            STRING_NODE,
        ]
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
        if let Some(regexp) = node.as_regular_expression_node() {
            let content_loc = regexp.content_loc();
            let content = &source.as_bytes()[content_loc.start_offset()..content_loc.end_offset()];
            let mut state = RegexScanState::default();
            scan_regex_segment(
                self,
                source,
                content,
                content_loc.start_offset(),
                is_extended_regex(regexp.closing_loc().as_slice()),
                &mut state,
                diagnostics,
            );
            return;
        }

        if let Some(regexp) = node.as_interpolated_regular_expression_node() {
            let mut state = RegexScanState::default();
            let extended = is_extended_regex(regexp.closing_loc().as_slice());

            for part in regexp.parts().iter() {
                if let Some(string) = part.as_string_node() {
                    let content_loc = string.content_loc();
                    let content =
                        &source.as_bytes()[content_loc.start_offset()..content_loc.end_offset()];
                    scan_regex_segment(
                        self,
                        source,
                        content,
                        content_loc.start_offset(),
                        extended,
                        &mut state,
                        diagnostics,
                    );
                } else {
                    // Interpolation can change what an escaped character would bind to, but
                    // character-class and comment state still continue across boundaries.
                    state.escaped = false;
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CharClassState {
    Start,
    AfterCaret,
    Normal,
}

#[derive(Default)]
struct RegexScanState {
    escaped: bool,
    in_comment: bool,
    class_stack: Vec<CharClassState>,
    saw_token: bool,
}

fn is_extended_regex(closing_loc: &[u8]) -> bool {
    closing_loc.contains(&b'x')
}

fn scan_regex_segment(
    cop: &UnescapedBracketInRegexp,
    source: &SourceFile,
    content: &[u8],
    content_start: usize,
    extended: bool,
    state: &mut RegexScanState,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (i, byte) in content.iter().copied().enumerate() {
        if state.in_comment {
            if byte == b'\n' {
                state.in_comment = false;
            }
            continue;
        }

        if state.escaped {
            state.escaped = false;
            if let Some(top) = state.class_stack.last_mut() {
                *top = CharClassState::Normal;
            } else {
                state.saw_token = true;
            }
            continue;
        }

        if let Some(top) = state.class_stack.last_mut() {
            match byte {
                b'\\' => {
                    state.escaped = true;
                    *top = CharClassState::Normal;
                }
                b'[' => {
                    *top = CharClassState::Normal;
                    state.class_stack.push(CharClassState::Start);
                }
                b']' => match *top {
                    CharClassState::Start | CharClassState::AfterCaret => {
                        *top = CharClassState::Normal;
                    }
                    CharClassState::Normal => {
                        state.class_stack.pop();
                    }
                },
                b'^' if *top == CharClassState::Start => {
                    *top = CharClassState::AfterCaret;
                }
                _ => {
                    *top = CharClassState::Normal;
                }
            }
            continue;
        }

        if extended {
            match byte {
                b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c => continue,
                b'#' => {
                    state.in_comment = true;
                    continue;
                }
                _ => {}
            }
        }

        match byte {
            b'\\' => {
                state.escaped = true;
                state.saw_token = true;
            }
            b'[' => {
                state.class_stack.push(CharClassState::Start);
                state.saw_token = true;
            }
            b']' => {
                if state.saw_token {
                    let offset = content_start + i;
                    let (line, column) = source.offset_to_line_col(offset);
                    diagnostics.push(cop.diagnostic(
                        source,
                        line,
                        column,
                        "Regular expression has `]` without escape.".to_string(),
                    ));
                }
                state.saw_token = true;
            }
            _ => {
                state.saw_token = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        UnescapedBracketInRegexp,
        "cops/lint/unescaped_bracket_in_regexp"
    );
}
