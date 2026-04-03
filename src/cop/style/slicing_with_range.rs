use crate::cop::shared::node_type::{CALL_NODE, INTEGER_NODE, RANGE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03-18):
/// - FN: `x[idx..nil]` and `x[idx...nil]` — Prism parses explicit `nil` as a NilNode
///   child (not absent). The cop was only checking for absent right (`is_none()`) and
///   integer `-1`, missing the NilNode case. Fixed by treating NilNode right child
///   the same as absent (endless) for both `0..nil` (redundant) and `x..nil` (suggest
///   endless range) patterns, matching RuboCop's behavior.
/// - FN: `x[idx .. - 1]` with spaces around unary minus — Prism parses the right side
///   as `CallNode(name: :-@, receiver: IntegerNode(1))` instead of a direct
///   `IntegerNode(-1)`. Fixed by teaching `int_value` to fold unary `-@`/`+@` calls on
///   integer literals so the existing `..-1` logic also covers `.. - 1`.
/// - FP fix: 4 corpus FPs on `x[0..]` / `x[0...]` patterns. RuboCop's NodePattern
///   `nil` matches a NilNode AST type, NOT an absent child. So `{(int -1) nil}` matches
///   `0..-1` and `0..nil` but NOT `0..` (endless range where right child is absent).
///   Fixed by only flagging explicit-nil right (`0..nil`, `0...nil`), not endless
///   ranges (`0..`, `0...`). The `right_is_nil_like` helper was removed; Pattern 1b
///   now checks for `right.as_nil_node()` specifically.
pub struct SlicingWithRange;

impl SlicingWithRange {
    fn int_value(node: &ruby_prism::Node<'_>) -> Option<i64> {
        if let Some(int_node) = node.as_integer_node() {
            let src = int_node.location().as_slice();
            if let Ok(s) = std::str::from_utf8(src) {
                return s.parse::<i64>().ok();
            }
        }

        // Prism parses spaced unary numeric literals like `- 1` as a `-@` call
        // on `1`, while compact `-1` is a direct IntegerNode.
        if let Some(call) = node.as_call_node() {
            let name = call.name().as_slice();
            if (name == b"-@" || name == b"+@")
                && call.arguments().is_none()
                && call.call_operator_loc().is_none()
            {
                let value = Self::int_value(&call.receiver()?)?;
                return Some(if name == b"-@" { -value } else { value });
            }
        }

        None
    }

    /// Check if the right side of a range is an explicit `nil` keyword (NilNode).
    /// This does NOT match an absent right child (endless range like `x..`).
    fn right_is_explicit_nil(range: &ruby_prism::RangeNode<'_>) -> bool {
        match range.right() {
            None => false,
            Some(right) => right.as_nil_node().is_some(),
        }
    }

    /// Match RuboCop's "current" message text:
    /// - bracket form: `[start .. - 1]` / `[start..nil]`
    /// - dot/safe-nav form: `start .. - 1` / `start..nil`
    fn current_range_source(
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        range: &ruby_prism::RangeNode<'_>,
    ) -> String {
        if call.call_operator_loc().is_some() {
            std::str::from_utf8(range.location().as_slice())
                .unwrap_or("")
                .to_string()
        } else {
            match (call.opening_loc(), call.closing_loc()) {
                (Some(open), Some(close)) => source
                    .byte_slice(open.start_offset(), close.end_offset(), "")
                    .to_string(),
                _ => std::str::from_utf8(range.location().as_slice())
                    .map(|text| format!("[{text}]"))
                    .unwrap_or_default(),
            }
        }
    }
}

impl Cop for SlicingWithRange {
    fn name(&self) -> &'static str {
        "Style/SlicingWithRange"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, INTEGER_NODE, RANGE_NODE]
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

        // Must be a [] call with exactly one argument
        if call.name().as_slice() != b"[]" {
            return;
        }
        if call.receiver().is_none() {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        let range_node = &arg_list[0];

        // Use opening_loc (the `[`) as the diagnostic position to match RuboCop
        let bracket_offset = call
            .opening_loc()
            .map(|l| l.start_offset())
            .unwrap_or(node.location().start_offset());

        if let Some(irange) = range_node.as_range_node() {
            let op = irange.operator_loc();
            let is_inclusive = op.as_slice() == b"..";
            let is_exclusive = op.as_slice() == b"...";
            let op_str = if is_inclusive { ".." } else { "..." };

            if let Some(left) = irange.left() {
                let left_is_zero = Self::int_value(&left) == Some(0);

                if left_is_zero {
                    // Pattern 1: 0..-1 (inclusive) — redundant, remove the slice
                    if is_inclusive {
                        if let Some(right) = irange.right() {
                            if Self::int_value(&right) == Some(-1) {
                                let (line, column) = source.offset_to_line_col(bracket_offset);
                                let src =
                                    std::str::from_utf8(node.location().as_slice()).unwrap_or("");
                                let recv = std::str::from_utf8(
                                    call.receiver().unwrap().location().as_slice(),
                                )
                                .unwrap_or("ary");
                                diagnostics.push(self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    format!("Prefer `{recv}` over `{src}`."),
                                ));
                                return;
                            }
                        }
                    }

                    // Pattern 1b: 0..nil (inclusive), 0...nil (exclusive) — redundant
                    // Note: 0.. and 0... (endless ranges) are NOT flagged — RuboCop's
                    // NodePattern `nil` matches NilNode, not absent children.
                    if (is_inclusive || is_exclusive) && Self::right_is_explicit_nil(&irange) {
                        let (line, column) = source.offset_to_line_col(bracket_offset);
                        let src = std::str::from_utf8(node.location().as_slice()).unwrap_or("");
                        let recv =
                            std::str::from_utf8(call.receiver().unwrap().location().as_slice())
                                .unwrap_or("ary");
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            format!("Prefer `{recv}` over `{src}`."),
                        ));
                        return;
                    }
                }

                // Pattern 2: x..-1 where x != 0 — suggest endless range
                if is_inclusive && !left_is_zero {
                    if let Some(right) = irange.right() {
                        if Self::int_value(&right) == Some(-1) {
                            let left_src =
                                std::str::from_utf8(left.location().as_slice()).unwrap_or("1");
                            let current_src = Self::current_range_source(source, &call, &irange);
                            let (line, column) = source.offset_to_line_col(bracket_offset);
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                format!("Prefer `[{left_src}..]` over `{current_src}`."),
                            ));
                            return;
                        }
                    }
                }

                // Pattern 2b: x..nil or x...nil where x != 0 — suggest endless range
                if !left_is_zero {
                    if let Some(right) = irange.right() {
                        if right.as_nil_node().is_some() {
                            let left_src =
                                std::str::from_utf8(left.location().as_slice()).unwrap_or("1");
                            let current_src = Self::current_range_source(source, &call, &irange);
                            let (line, column) = source.offset_to_line_col(bracket_offset);
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                format!("Prefer `[{left_src}{op_str}]` over `{current_src}`."),
                            ));
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SlicingWithRange, "cops/style/slicing_with_range");
}
