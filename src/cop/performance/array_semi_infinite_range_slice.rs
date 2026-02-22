use crate::cop::node_type::{CALL_NODE, RANGE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ArraySemiInfiniteRangeSlice;

impl Cop for ArraySemiInfiniteRangeSlice {
    fn name(&self) -> &'static str {
        "Performance/ArraySemiInfiniteRangeSlice"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"[]" {
            return;
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

        // Semi-infinite range: has a left but no right (e.g., 2..)
        if range.left().is_none() || range.right().is_some() {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `drop` instead of `[]` with a semi-infinite range.".to_string(),
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
