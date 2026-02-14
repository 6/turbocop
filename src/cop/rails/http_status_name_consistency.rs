use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct HttpStatusNameConsistency;

impl Cop for HttpStatusNameConsistency {
    fn name(&self) -> &'static str {
        "Rails/HttpStatusNameConsistency"
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
        if call.name().as_slice() != b"head" {
            return Vec::new();
        }
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        // Check if first positional argument is a numeric literal
        let first = match args.arguments().iter().next() {
            Some(a) => a,
            None => return Vec::new(),
        };
        if first.as_integer_node().is_some() {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Use symbolic status code instead of numeric.".to_string(),
            )];
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        HttpStatusNameConsistency,
        "cops/rails/http_status_name_consistency"
    );
}
