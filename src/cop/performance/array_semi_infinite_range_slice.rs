use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ArraySemiInfiniteRangeSlice;

fn is_string_receiver(receiver: &ruby_prism::Node<'_>) -> bool {
    receiver.as_string_node().is_some()
        || receiver.as_interpolated_string_node().is_some()
        || receiver.as_x_string_node().is_some()
        || receiver.as_interpolated_x_string_node().is_some()
}

/// Check if a node is a positive integer literal.
fn is_positive_int(node: &ruby_prism::Node<'_>, source: &SourceFile) -> bool {
    if let Some(int_node) = node.as_integer_node() {
        let loc = int_node.location();
        let src = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
        if let Ok(s) = std::str::from_utf8(src) {
            if let Ok(v) = s.parse::<i64>() {
                return v > 0;
            }
        }
    }
    false
}

/// Check if a range node is a semi-infinite range with a positive integer literal endpoint.
/// Returns Some("drop") for endless ranges (N..) and Some("take") for beginless ranges (..N).
fn semi_infinite_range_direction(
    range: &ruby_prism::RangeNode<'_>,
    source: &SourceFile,
) -> Option<&'static str> {
    match (range.left(), range.right()) {
        // Endless range: N.. or N...
        (Some(left), None) => {
            if is_positive_int(&left, source) {
                Some("drop")
            } else {
                None
            }
        }
        // Beginless range: ..N or ...N
        (None, Some(right)) => {
            if is_positive_int(&right, source) {
                Some("take")
            } else {
                None
            }
        }
        _ => None,
    }
}

impl Cop for ArraySemiInfiniteRangeSlice {
    fn name(&self) -> &'static str {
        "Performance/ArraySemiInfiniteRangeSlice"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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

        let method_name = call.name();
        let method_bytes = method_name.as_slice();
        let is_bracket = method_bytes == b"[]";
        let is_slice = method_bytes == b"slice";

        if !is_bracket && !is_slice {
            return;
        }

        // Skip string literal receivers
        if let Some(receiver) = call.receiver() {
            if is_string_receiver(&receiver) {
                return;
            }
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args = arguments.arguments();
        if args.len() != 1 {
            return;
        }

        let first_arg = args.iter().next().unwrap();
        let range = match first_arg.as_range_node() {
            Some(r) => r,
            None => return,
        };

        let direction = match semi_infinite_range_direction(&range, source) {
            Some(d) => d,
            None => return,
        };

        let method_display = if is_bracket { "[]" } else { "slice" };

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{direction}` instead of `{method_display}` with a semi-infinite range."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ArraySemiInfiniteRangeSlice,
        "cops/performance/array_semi_infinite_range_slice"
    );
}
