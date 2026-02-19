use crate::cop::util::{is_rspec_example_group, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, PROGRAM_NODE};

/// RSpec/MultipleDescribes: Flag multiple top-level example groups in a single file.
pub struct MultipleDescribes;

impl Cop for MultipleDescribes {
    fn name(&self) -> &'static str {
        "RSpec/MultipleDescribes"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, PROGRAM_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Only check ProgramNode (root)
        let program = match node.as_program_node() {
            Some(p) => p,
            None => return,
        };

        let stmts = program.statements();
        let body = stmts.body();

        // Collect top-level example group calls (describe, context, etc.)
        // but exclude shared_examples, shared_examples_for, shared_context
        let mut example_groups: Vec<(usize, usize)> = Vec::new();

        for stmt in body.iter() {
            if let Some(call) = stmt.as_call_node() {
                let name = call.name().as_slice();
                if is_top_level_example_group(call.receiver().as_ref(), name) {
                    let loc = call.location();
                    let (line, col) = source.offset_to_line_col(loc.start_offset());
                    example_groups.push((line, col));
                }
            }
        }

        if example_groups.len() <= 1 {
            return;
        }

        // Flag all except the last one (RuboCop flags all except the last)
        for &(line, col) in &example_groups[..example_groups.len() - 1] {
            diagnostics.push(self.diagnostic(
                source,
                line,
                col,
                "Do not use multiple top-level example groups - try to nest them.".to_string(),
            ));
        }

    }
}

fn is_top_level_example_group(receiver: Option<&ruby_prism::Node<'_>>, name: &[u8]) -> bool {
    // Shared examples/contexts are excluded
    if name == b"shared_examples"
        || name == b"shared_examples_for"
        || name == b"shared_context"
    {
        return false;
    }

    if is_rspec_example_group(name) {
        // Must be receiverless or RSpec.describe / ::RSpec.describe
        match receiver {
            None => return true,
            Some(recv) => {
                if let Some(cr) = recv.as_constant_read_node() {
                    if cr.name().as_slice() == b"RSpec" {
                        return true;
                    }
                }
                if let Some(cp) = recv.as_constant_path_node() {
                    if let Some(n) = cp.name() {
                        if n.as_slice() == b"RSpec" && cp.parent().is_none() {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_scenario_fixture_tests!(
        MultipleDescribes, "cops/rspec/multiple_describes",
        scenario_class_and_method = "class_and_method.rb",
        scenario_class_only = "class_only.rb",
        scenario_string_args = "string_args.rb",
    );
}
