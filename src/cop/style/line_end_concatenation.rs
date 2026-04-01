use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Follow-up (2026-04-01): FN=2549/FP=20 was dominated by two mismatches with
/// RuboCop's token-based cop:
///
/// 1. The old line scanner only accepted a very narrow successor shape, so it
///    missed valid offenses when the continued string was followed by commas,
///    closing brackets, or appeared in chained `+`/`<<` expressions.
/// 2. It scanned raw string content and therefore flagged JavaScript-like
///    concatenation inside `%()` string literals, which RuboCop ignores.
///
/// Fix: inspect Prism `CallNode`s for `+`/`<<` directly, require the operator to
/// break across lines without an intervening comment, require the right-hand side
/// to be a quoted string node, and allow the left-hand side to be either a quoted
/// string or a concat chain that textually ends with one.
pub struct LineEndConcatenation;

impl Cop for LineEndConcatenation {
    fn name(&self) -> &'static str {
        "Style/LineEndConcatenation"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let Some(call) = node.as_call_node() else {
            return;
        };

        let operator = call.name().as_slice();
        if operator != b"+" && operator != b"<<" {
            return;
        }

        let Some(receiver) = call.receiver() else {
            return;
        };
        if !Self::ends_with_standard_string_literal(&receiver) {
            return;
        }

        let Some(arguments) = call.arguments() else {
            return;
        };
        let arg_list: Vec<_> = arguments.arguments().iter().collect();
        if arg_list.len() != 1 || !Self::is_standard_string_literal(&arg_list[0]) {
            return;
        }

        let Some(message_loc) = call.message_loc() else {
            return;
        };
        let argument = &arg_list[0];
        let operator_end = message_loc.end_offset();
        let argument_start = argument.location().start_offset();

        let (operator_line, _) = source.offset_to_line_col(message_loc.start_offset());
        let (argument_line, _) = source.offset_to_line_col(argument_start);
        if operator_line == argument_line {
            return;
        }

        if !Self::has_line_break_without_comment(source.as_bytes(), operator_end, argument_start) {
            return;
        }

        let (line, column) = source.offset_to_line_col(message_loc.start_offset());
        let operator = std::str::from_utf8(operator).unwrap_or("+");
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `\\` instead of `{}` to concatenate multiline strings.",
                operator
            ),
        ));
    }
}

impl LineEndConcatenation {
    fn is_standard_string_literal(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(string) = node.as_string_node() {
            return string.opening_loc().is_some_and(|loc| {
                let opening = loc.as_slice();
                opening == b"'" || opening == b"\""
            });
        }

        if let Some(string) = node.as_interpolated_string_node() {
            return string.opening_loc().is_some_and(|loc| {
                let opening = loc.as_slice();
                opening == b"'" || opening == b"\""
            });
        }

        false
    }

    fn ends_with_standard_string_literal(node: &ruby_prism::Node<'_>) -> bool {
        if Self::is_standard_string_literal(node) {
            return true;
        }

        let Some(call) = node.as_call_node() else {
            return false;
        };
        let operator = call.name().as_slice();
        if operator != b"+" && operator != b"<<" {
            return false;
        }

        let Some(arguments) = call.arguments() else {
            return false;
        };
        let mut arg_iter = arguments.arguments().iter();
        let Some(last_arg) = arg_iter.next() else {
            return false;
        };
        if arg_iter.next().is_some() {
            return false;
        }

        Self::ends_with_standard_string_literal(&last_arg)
    }

    fn has_line_break_without_comment(source: &[u8], start: usize, end: usize) -> bool {
        if start >= end || start >= source.len() {
            return false;
        }

        let between = &source[start..end.min(source.len())];
        between.contains(&b'\n') && !between.contains(&b'#')
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(LineEndConcatenation, "cops/style/line_end_concatenation");
}
