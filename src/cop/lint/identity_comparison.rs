use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct IdentityComparison;

impl Cop for IdentityComparison {
    fn name(&self) -> &'static str {
        "Lint/IdentityComparison"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"equal?" {
            return Vec::new();
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list = args.arguments();
        if arg_list.len() != 1 {
            return Vec::new();
        }

        let first_arg = match arg_list.iter().next() {
            Some(a) => a,
            None => return Vec::new(),
        };

        // Compare source text of receiver and argument
        let recv_text = receiver.location().as_slice();
        let arg_text = first_arg.location().as_slice();

        if recv_text == arg_text {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Comparing an object to itself with `equal?`.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IdentityComparison, "cops/lint/identity_comparison");
}
