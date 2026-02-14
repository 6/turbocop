use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct WhereRange;

const SQL_COMPARISON_OPS: &[&[u8]] = &[
    b">=", b"<=", b">", b"<", b"BETWEEN",
    b"between",
];

impl Cop for WhereRange {
    fn name(&self) -> &'static str {
        "Rails/WhereRange"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"where" {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        // First argument should be a string containing a comparison operator
        let string_node = match arg_list[0].as_string_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let content = string_node.unescaped();
        let has_comparison = SQL_COMPARISON_OPS.iter().any(|op| {
            content.windows(op.len()).any(|w| w == *op)
        });

        if !has_comparison {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use a range in `where` instead of manually constructing SQL conditions.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(WhereRange, "cops/rails/where_range");
}
