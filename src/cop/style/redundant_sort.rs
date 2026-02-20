use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, INTEGER_NODE};

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

    /// Check if a call is to sort or sort_by (with no args for sort, with/without args for sort_by)
    fn is_sort_call(call: &ruby_prism::CallNode<'_>) -> Option<&'static str> {
        let name = call.name();
        let name_bytes = name.as_slice();
        if name_bytes == b"sort" && call.arguments().is_none() && call.block().is_none() {
            return Some("sort");
        }
        if name_bytes == b"sort_by" {
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

        // Determine if accessing first or last element
        let is_first = if method_bytes == b"first" {
            if call.arguments().is_some() { return; }
            true
        } else if method_bytes == b"last" {
            if call.arguments().is_some() { return; }
            false
        } else {
            // [], at, slice -- check argument
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() != 1 {
                    return;
                }
                match Self::int_value(&arg_list[0]) {
                    Some(0) => true,
                    Some(-1) => false,
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

        let sorter = if let Some(sort_call) = receiver.as_call_node() {
            match Self::is_sort_call(&sort_call) {
                Some(s) => s,
                None => return,
            }
        } else {
            return;
        };

        let suggestion = if is_first {
            if sorter == "sort" { "min" } else { "min_by" }
        } else {
            if sorter == "sort" { "max" } else { "max_by" }
        };

        let accessor_src = std::str::from_utf8(method_bytes).unwrap_or("");
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{}` instead of `{}...{}`.", suggestion, sorter, accessor_src),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantSort, "cops/style/redundant_sort");
}
