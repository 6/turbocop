use crate::cop::node_type::{CALL_NODE, INTEGER_NODE, RANGE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SlicingWithRange;

impl SlicingWithRange {
    fn int_value(node: &ruby_prism::Node<'_>) -> Option<i64> {
        if let Some(int_node) = node.as_integer_node() {
            let src = int_node.location().as_slice();
            if let Ok(s) = std::str::from_utf8(src) {
                return s.parse::<i64>().ok();
            }
        }
        None
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

        // Check for inclusive range (0..-1) or (0..nil)
        if let Some(irange) = range_node.as_range_node() {
            // Check operator is .. (inclusive)
            let op = irange.operator_loc();
            let is_inclusive = op.as_slice() == b"..";
            let is_exclusive = op.as_slice() == b"...";

            if let Some(left) = irange.left() {
                if Self::int_value(&left) == Some(0) {
                    // 0..-1 (inclusive) — redundant, remove the slice
                    if is_inclusive {
                        if let Some(right) = irange.right() {
                            if Self::int_value(&right) == Some(-1) {
                                let (line, column) = source.offset_to_line_col(bracket_offset);
                                let src =
                                    std::str::from_utf8(node.location().as_slice()).unwrap_or("");
                                diagnostics.push(self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    format!(
                                            "Prefer `{}` over `{}`.",
                                            std::str::from_utf8(
                                                call.receiver().unwrap().location().as_slice()
                                            )
                                            .unwrap_or("ary"),
                                            src
                                        ),
                                ));
                            }
                        }
                        // 0..nil — also redundant
                        if irange.right().is_none() {
                            let (line, column) = source.offset_to_line_col(bracket_offset);
                            let src = std::str::from_utf8(node.location().as_slice()).unwrap_or("");
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                format!(
                                        "Prefer `{}` over `{}`.",
                                        std::str::from_utf8(
                                            call.receiver().unwrap().location().as_slice()
                                        )
                                        .unwrap_or("ary"),
                                        src
                                    ),
                            ));
                        }
                    }
                    // 0...nil — also redundant
                    if is_exclusive && irange.right().is_none() {
                        let (line, column) = source.offset_to_line_col(bracket_offset);
                        let src = std::str::from_utf8(node.location().as_slice()).unwrap_or("");
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            format!(
                                    "Prefer `{}` over `{}`.",
                                    std::str::from_utf8(
                                        call.receiver().unwrap().location().as_slice()
                                    )
                                    .unwrap_or("ary"),
                                    src
                                ),
                        ));
                    }
                }

                // x..-1 where x != 0 — suggest endless range
                if is_inclusive {
                    if let Some(right) = irange.right() {
                        if Self::int_value(&right) == Some(-1) && Self::int_value(&left) != Some(0)
                        {
                            let left_src =
                                std::str::from_utf8(left.location().as_slice()).unwrap_or("1");
                            let (line, column) = source.offset_to_line_col(bracket_offset);
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                format!("Prefer `[{}..]` over `[{}..-1]`.", left_src, left_src),
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
