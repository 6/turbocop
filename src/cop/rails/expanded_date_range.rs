use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, RANGE_NODE};

pub struct ExpandedDateRange;

impl Cop for ExpandedDateRange {
    fn name(&self) -> &'static str {
        "Rails/ExpandedDateRange"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, RANGE_NODE]
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
        let range = match node.as_range_node() {
            Some(r) => r,
            None => return,
        };

        let left = match range.left() {
            Some(l) => l,
            None => return,
        };

        let right = match range.right() {
            Some(r) => r,
            None => return,
        };

        // Left should be a call to .beginning_of_day
        let left_call = match left.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if left_call.name().as_slice() != b"beginning_of_day" {
            return;
        }

        // Right should be a call to .end_of_day
        let right_call = match right.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if right_call.name().as_slice() != b"end_of_day" {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `all_day` instead of explicit date range expansion.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExpandedDateRange, "cops/rails/expanded_date_range");
}
