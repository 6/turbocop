use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct MultipleComparison;

impl Cop for MultipleComparison {
    fn name(&self) -> &'static str {
        "Lint/MultipleComparison"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Pattern: (send (send _ COMP _) COMP _)
        // i.e., x < y < z
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let outer_method = outer_call.name().as_slice();
        if !is_comparison(outer_method) {
            return Vec::new();
        }

        // The receiver of the outer call should itself be a comparison call
        let receiver = match outer_call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let inner_method = inner_call.name().as_slice();
        if !is_comparison(inner_method) {
            return Vec::new();
        }

        let loc = outer_call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use the `&&` operator to compare multiple values.".to_string(),
        )]
    }
}

fn is_comparison(method: &[u8]) -> bool {
    matches!(method, b"<" | b">" | b"<=" | b">=")
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultipleComparison, "cops/lint/multiple_comparison");
}
