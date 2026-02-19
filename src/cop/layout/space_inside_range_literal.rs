use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::RANGE_NODE;

pub struct SpaceInsideRangeLiteral;

impl Cop for SpaceInsideRangeLiteral {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsideRangeLiteral"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[RANGE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Check both inclusive (..) and exclusive (...) ranges
        let (left, right, op_loc) = if let Some(range) = node.as_range_node() {
            (range.left(), range.right(), range.operator_loc())
        } else {
            return;
        };

        let bytes = source.as_bytes();
        let op_start = op_loc.start_offset();
        let op_end = op_loc.end_offset();

        let mut has_space = false;

        // Check space before operator
        if let Some(left_node) = left {
            let left_end = left_node.location().end_offset();
            if op_start > left_end {
                let between = &bytes[left_end..op_start];
                if between.iter().any(|&b| b == b' ' || b == b'\t') {
                    has_space = true;
                }
            }
        }

        // Check space after operator
        if let Some(right_node) = right {
            let right_start = right_node.location().start_offset();
            if right_start > op_end {
                let between = &bytes[op_end..right_start];
                if between.iter().any(|&b| b == b' ' || b == b'\t') {
                    has_space = true;
                }
            }
        }

        if has_space {
            let (line, col) = source.offset_to_line_col(node.location().start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                col,
                "Space inside range literal.".to_string(),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceInsideRangeLiteral, "cops/layout/space_inside_range_literal");
}
