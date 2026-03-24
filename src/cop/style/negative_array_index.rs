use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/NegativeArrayIndex
///
/// Detects `arr[arr.length - N]` and suggests `arr[-N]`.
/// Also detects range patterns like `arr[(0..(arr.length - N))]` and suggests `arr[(0..-N)]`.
///
/// ## Investigation findings (2026-03-23)
///
/// **FP root cause (2 FPs):** nitrocop was using simple source-text comparison for receiver
/// matching without validating that receivers are "preserving methods." RuboCop only allows
/// the pattern when receivers are bare variables/constants or chains of preserving methods
/// (sort, reverse, shuffle, rotate). Method calls like `doc.pages` or indexing like
/// `assigns[:tags]` are NOT considered preserving, so RuboCop skips them. Fixed by
/// implementing the `preserving_method?` check from RuboCop.
///
/// **FN root cause (21 FNs):** nitrocop did not handle range-based indexing patterns like
/// `arr[0..arr.length-2]` or `arr[(1..(arr.size-2))]`. Also missing: `self[length - 1]`
/// implicit receiver, preserving method chains like `arr.sort[arr.reverse.length - 2]`.
/// Fixed by adding range pattern detection and the full receiver matching logic.
///
/// **RuboCop also skips:** assignment `arr[arr.length-2] = val`, subtraction by 0,
/// subtraction by a variable (non-integer), and receivers with non-preserving method chains.
pub struct NegativeArrayIndex;

const PRESERVING_METHODS: &[&[u8]] = &[b"sort", b"reverse", b"shuffle", b"rotate"];

/// Check if a node is a "preserving method" chain. That means:
/// - A bare variable/constant (receiver is None for calls, or it's not a call at all)
/// - A call to sort/reverse/shuffle/rotate on a preserving receiver
fn is_preserving(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => {
            // Any non-call node (variable read, constant, self, etc.) is a base case
            return true;
        }
    };

    let receiver = match call.receiver() {
        Some(r) => r,
        // No receiver means it's a bare identifier/method call like `arr` — this is
        // the base case (equivalent to RuboCop's `node.receiver.nil?` → true).
        // In Prism, `arr` without a local assignment is a CallNode with no receiver.
        None => return true,
    };

    let method_bytes = call.name().as_slice();

    if !PRESERVING_METHODS.contains(&method_bytes) {
        return false;
    }

    // Must not have arguments
    if call.arguments().is_some() {
        return false;
    }

    is_preserving(&receiver)
}

/// Check if a node is a CallNode with a receiver (i.e., has at least one method chain level).
/// RuboCop's `extract_base_receiver` returns non-nil only when the node has a receiver chain.
fn has_receiver_chain(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        return call.receiver().is_some();
    }
    false
}

fn node_src(node: &ruby_prism::Node<'_>) -> String {
    std::str::from_utf8(node.location().as_slice())
        .unwrap_or("")
        .to_string()
}

/// Info from a `something.length - N` pattern.
struct LengthSubtraction {
    /// Source text of the receiver of `.length`/`.size`/`.count`, or None for implicit receiver.
    length_receiver_src: Option<String>,
    /// Whether the length receiver is a preserving chain.
    length_receiver_preserving: bool,
    /// Source text of N (the subtracted integer).
    n_src: String,
}

/// Try to extract a `length_subtraction` pattern from a node.
fn extract_length_subtraction(node: &ruby_prism::Node<'_>) -> Option<LengthSubtraction> {
    let sub_call = node.as_call_node()?;

    if sub_call.name().as_slice() != b"-" {
        return None;
    }

    // Must have exactly one argument on the right side of `-`
    let sub_args = sub_call.arguments()?;
    let sub_arg_list: Vec<_> = sub_args.arguments().iter().collect();
    if sub_arg_list.len() != 1 {
        return None;
    }

    let sub_receiver = sub_call.receiver()?;

    let length_call = sub_receiver.as_call_node()?;
    let method_bytes = length_call.name().as_slice();
    if method_bytes != b"length" && method_bytes != b"size" && method_bytes != b"count" {
        return None;
    }

    // Length method must not have arguments
    if length_call.arguments().is_some() {
        return None;
    }

    // The subtracted value must be a positive integer literal
    let n_node = &sub_arg_list[0];
    n_node.as_integer_node()?;
    let n_src = node_src(n_node);
    // Filter out 0 and negative
    if n_src == "0" || n_src.starts_with('-') {
        return None;
    }

    let (length_receiver_src, length_receiver_preserving) = match length_call.receiver() {
        Some(r) => (Some(node_src(&r)), is_preserving(&r)),
        None => (None, true),
    };

    Some(LengthSubtraction {
        length_receiver_src,
        length_receiver_preserving,
        n_src,
    })
}

impl Cop for NegativeArrayIndex {
    fn name(&self) -> &'static str {
        "Style/NegativeArrayIndex"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be `[]` method (not `[]=`)
        if call.name().as_slice() != b"[]" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        let arg = &arg_list[0];

        // Check if arg is a ParenthesesNode wrapping a range
        if let Some(parens) = arg.as_parentheses_node() {
            if let Some(body) = parens.body() {
                if let Some(stmts) = body.as_statements_node() {
                    let inner_list: Vec<_> = stmts.body().iter().collect();
                    if inner_list.len() == 1 {
                        if let Some(range) = inner_list[0].as_range_node() {
                            self.check_range_pattern(source, &receiver, range, true, diagnostics);
                            return;
                        }
                    }
                }
            }
        }

        // Check if arg is a bare range (without parens)
        if let Some(range) = arg.as_range_node() {
            self.check_range_pattern(source, &receiver, range, false, diagnostics);
            return;
        }

        // Simple index pattern: arr[arr.length - N]
        self.check_simple_pattern(source, &receiver, arg, diagnostics);
    }
}

impl NegativeArrayIndex {
    fn check_simple_pattern(
        &self,
        source: &SourceFile,
        array_receiver: &ruby_prism::Node<'_>,
        arg: &ruby_prism::Node<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let info = match extract_length_subtraction(arg) {
            Some(v) => v,
            None => return,
        };

        let arr_src = node_src(array_receiver);

        match info.length_receiver_src {
            None => {
                // Implicit receiver on length: `self[length - 1]`
                if array_receiver.as_self_node().is_none() {
                    return;
                }
            }
            Some(ref len_recv_src) => {
                // Both must be preserving
                if !info.length_receiver_preserving || !is_preserving(array_receiver) {
                    return;
                }
                // If sources match, always ok
                if arr_src != *len_recv_src {
                    // When sources differ, the array_receiver must have a receiver chain
                    // (i.e., at least one method call like arr.sort). A bare variable
                    // like `arr` has no receiver, so arr[arr.sort.length-2] is NOT
                    // flagged, but arr.sort[arr.reverse.length-2] IS flagged.
                    // This matches RuboCop's extract_base_receiver check.
                    if !has_receiver_chain(array_receiver) {
                        return;
                    }
                }
            }
        }

        let arg_src = node_src(arg);
        let n_src = &info.n_src;
        let full_src = format!("{arr_src}[{arg_src}]");
        let loc = arg.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{arr_src}[-{n_src}]` instead of `{full_src}`."),
        ));
    }

    fn check_range_pattern(
        &self,
        source: &SourceFile,
        array_receiver: &ruby_prism::Node<'_>,
        range: ruby_prism::RangeNode<'_>,
        has_outer_parens: bool,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let range_start = match range.left() {
            Some(s) => s,
            None => return,
        };
        let range_end = match range.right() {
            Some(e) => e,
            None => return,
        };

        // Range start must be a "preserving" expression
        if !is_preserving(&range_start) {
            return;
        }

        // The range end might be wrapped in parens: (arr.length - N)
        let (info, end_has_parens) = if let Some(parens) = range_end.as_parentheses_node() {
            let extracted = (|| -> Option<LengthSubtraction> {
                let body = parens.body()?;
                let stmts = body.as_statements_node()?;
                let inner_list: Vec<_> = stmts.body().iter().collect();
                if inner_list.len() != 1 {
                    return None;
                }
                extract_length_subtraction(&inner_list[0])
            })();
            match extracted {
                Some(info) => (info, true),
                None => return,
            }
        } else {
            match extract_length_subtraction(&range_end) {
                Some(info) => (info, false),
                None => return,
            }
        };

        // For range patterns, use strict matching (source must match exactly)
        match &info.length_receiver_src {
            None => return, // No implicit receiver for range patterns
            Some(len_recv_src) => {
                if !is_preserving(array_receiver) {
                    return;
                }
                let arr_src = node_src(array_receiver);
                if arr_src != *len_recv_src {
                    return;
                }
            }
        }

        let arr_src = node_src(array_receiver);
        let range_op = if range.is_exclude_end() { "..." } else { ".." };
        let start_src = node_src(&range_start);
        let n_src = &info.n_src;

        let inner_end_src = node_src(&range_end);

        let (current_str, replacement_str) = if has_outer_parens {
            let current = format!("{arr_src}[({start_src}{range_op}{inner_end_src})]");
            let replacement = format!("{arr_src}[({start_src}{range_op}-{n_src})]");
            (current, replacement)
        } else {
            let end_display = if end_has_parens {
                // This would be like arr[0..(arr.length - 2)] without outer parens
                // but in practice range ends with parens usually have outer parens too
                inner_end_src.clone()
            } else {
                inner_end_src.clone()
            };
            let current = format!("{arr_src}[{start_src}{range_op}{end_display}]");
            let replacement = format!("{arr_src}[{start_src}{range_op}-{n_src}]");
            (current, replacement)
        };

        let loc = range_end.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{replacement_str}` instead of `{current_str}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NegativeArrayIndex, "cops/style/negative_array_index");
}
