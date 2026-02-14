use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ExpectOutput;

impl Cop for ExpectOutput {
    fn name(&self) -> &'static str {
        "RSpec/ExpectOutput"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Look for $stdout = ... or $stderr = ...
        let gvw = match node.as_global_variable_write_node() {
            Some(g) => g,
            None => return Vec::new(),
        };

        let name = gvw.name().as_slice();
        let stream = if name == b"$stdout" {
            "stdout"
        } else if name == b"$stderr" {
            "stderr"
        } else {
            return Vec::new();
        };

        let loc = gvw.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `expect {{ ... }}.to output(...).to_{stream}` instead of mutating ${stream}."
            ),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExpectOutput, "cops/rspec/expect_output");
}
