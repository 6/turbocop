use crate::cop::node_type::{CALL_NODE, HASH_NODE};
use crate::cop::util::{self, RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EmptyMetadata;

impl Cop for EmptyMetadata {
    fn name(&self) -> &'static str {
        "RSpec/EmptyMetadata"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, HASH_NODE]
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
        // Detect empty metadata hash `{}` in example groups/examples
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Check if this is an RSpec method (example group or example, including ::RSpec.describe)
        let is_rspec = if call.receiver().is_none() {
            is_rspec_example_group(method_name) || is_rspec_example(method_name)
        } else if let Some(recv) = call.receiver() {
            util::constant_name(&recv).map_or(false, |n| n == b"RSpec")
                && method_name == b"describe"
        } else {
            false
        };

        if !is_rspec {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        // Note: keyword_hash_node (keyword args) intentionally not handled —
        // empty metadata is specifically the `{}` hash literal form, not keyword args.
        for arg in args.arguments().iter() {
            if let Some(hash) = arg.as_hash_node() {
                if hash.elements().iter().count() == 0 {
                    let loc = hash.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Avoid empty metadata hash.".to_string(),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyMetadata, "cops/rspec/empty_metadata");
}
