use crate::cop::util::{is_rspec_example_group, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SpecFilePathSuffix;

impl Cop for SpecFilePathSuffix {
    fn name(&self) -> &'static str {
        "RSpec/SpecFilePathSuffix"
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
        // Only check ProgramNode (root)
        let program = match node.as_program_node() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let stmts = program.statements();
        let body = stmts.body();

        // Check if file contains any example group (not just shared examples)
        let has_example_group = body.iter().any(|stmt| {
            if let Some(call) = stmt.as_call_node() {
                let name = call.name().as_slice();
                if is_rspec_example_group(name)
                    && name != b"shared_examples"
                    && name != b"shared_examples_for"
                    && name != b"shared_context"
                {
                    return true;
                }
                // Also handle feature
                if name == b"feature" {
                    return true;
                }
            }
            false
        });

        if !has_example_group {
            return Vec::new();
        }

        let path = source.path_str();
        if path.ends_with("_spec.rb") {
            return Vec::new();
        }

        // File-level offense — report at line 1, column 0
        vec![self.diagnostic(
            source,
            1,
            0,
            "Spec path should end with `_spec.rb`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_scenario_fixture_tests!(
        SpecFilePathSuffix, "cops/rspec/spec_file_path_suffix",
        scenario_repeated_rb = "repeated_rb.rb",
        scenario_missing_spec = "missing_spec.rb",
        scenario_wrong_ext = "wrong_ext.rb",
    );
}
