use crate::cop::shared::node_type::{CALL_NODE, INTEGER_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Detects `sort.first`, `sort.last`, `sort_by {...}.first`, etc. and suggests
/// `min`/`max`/`min_by`/`max_by` instead.
///
/// ## Investigation notes
///
/// Historical FP/FN root causes (21 FP, 38 FN):
/// 1. **Offense location offset** -- nitrocop reported the offense at the outer
///    accessor call node (`.first`/`.last`/`[]`) start, which includes the entire
///    receiver chain. RuboCop reports starting at the `sort`/`sort_by` method name.
///    Fixed by using the sort call's `message_loc()` for the offense position.
/// 2. **`sort` with block not detected** -- `is_sort_call` required `block().is_none()`
///    for `sort`, rejecting `sort { |a, b| ... }.first`. RuboCop detects this pattern.
///    Fixed by allowing `sort` with a block (but still requiring no positional arguments).
/// 3. **`sort_by` without block or arguments** -- `sort_by` with no block returns an
///    Enumerator, not a sorted array. RuboCop does not flag `sort_by.first`.
///    Fixed by requiring `sort_by` to have a block or arguments.
/// 4. **Message format for `[]`** -- reported `sort...[]` instead of `sort...[0]` or
///    `sort...[-1]`. Fixed by including the index argument in the accessor display.
/// 5. **`sort` with block argument FP** -- `sort(&method(:cmp)).last` and `sort(&:foo).first`
///    were flagged because `&block_arg` is stored in Prism's `call.block()` (as a
///    `BlockArgumentNode`), not in `call.arguments()`, so the `arguments().is_none()` guard
///    passed. RuboCop only flags `sort` with no args or a real block, not block arguments.
///    Fixed by checking that `block()` is not a `BlockArgumentNode`.
pub struct RedundantSort;

impl RedundantSort {
    fn int_value(node: &ruby_prism::Node<'_>) -> Option<i64> {
        if let Some(int_node) = node.as_integer_node() {
            let src = int_node.location().as_slice();
            if let Ok(s) = std::str::from_utf8(src) {
                return s.parse::<i64>().ok();
            }
        }
        None
    }

    /// Check if a call is to sort or sort_by.
    /// - `sort`: requires no positional arguments (block is allowed for comparator)
    /// - `sort_by`: requires a block or arguments (bare `sort_by` returns Enumerator)
    fn is_sort_call(call: &ruby_prism::CallNode<'_>) -> Option<&'static str> {
        let name = call.name();
        let name_bytes = name.as_slice();
        if name_bytes == b"sort" && call.arguments().is_none() {
            // sort(&block_arg) passes the block argument via call.block() as a
            // BlockArgumentNode, not via call.arguments(). RuboCop does not flag
            // sort(&method(:cmp)).last or sort(&:foo).first — only plain sort or
            // sort with a real block { |a,b| ... } are redundant.
            if call
                .block()
                .is_some_and(|b| b.as_block_argument_node().is_some())
            {
                return None;
            }
            return Some("sort");
        }
        if name_bytes == b"sort_by" && (call.block().is_some() || call.arguments().is_some()) {
            return Some("sort_by");
        }
        None
    }
}

impl Cop for RedundantSort {
    fn name(&self) -> &'static str {
        "Style/RedundantSort"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, INTEGER_NODE]
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

        let method_name = call.name();
        let method_bytes = method_name.as_slice();

        // Must be .first, .last, .[], .at, or .slice
        if !matches!(method_bytes, b"first" | b"last" | b"[]" | b"at" | b"slice") {
            return;
        }

        // Determine if accessing first or last element, and build accessor display string
        let (is_first, accessor_display) = if method_bytes == b"first" {
            if call.arguments().is_some() {
                return;
            }
            (true, "first".to_string())
        } else if method_bytes == b"last" {
            if call.arguments().is_some() {
                return;
            }
            (false, "last".to_string())
        } else {
            // [], at, slice -- check argument
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() != 1 {
                    return;
                }
                let method_str = std::str::from_utf8(method_bytes).unwrap_or("[]");
                match Self::int_value(&arg_list[0]) {
                    Some(0) => {
                        let display = if method_bytes == b"[]" {
                            "[0]".to_string()
                        } else {
                            format!("{}(0)", method_str)
                        };
                        (true, display)
                    }
                    Some(-1) => {
                        let display = if method_bytes == b"[]" {
                            "[-1]".to_string()
                        } else {
                            format!("{}(-1)", method_str)
                        };
                        (false, display)
                    }
                    _ => return,
                }
            } else {
                return;
            }
        };

        // Receiver must be a call to .sort or .sort_by
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let sort_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let sorter = match Self::is_sort_call(&sort_call) {
            Some(s) => s,
            None => return,
        };

        let suggestion = if is_first {
            if sorter == "sort" { "min" } else { "min_by" }
        } else if sorter == "sort" {
            "max"
        } else {
            "max_by"
        };

        // Use the sort/sort_by call's message_loc for the offense position
        // (RuboCop highlights from the sort method name through the accessor)
        let loc = sort_call.message_loc().unwrap_or(sort_call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `{}` instead of `{}...{}`.",
                suggestion, sorter, accessor_display
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantSort, "cops/style/redundant_sort");
}
