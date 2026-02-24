use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RequireDependency;

impl Cop for RequireDependency {
    fn name(&self) -> &'static str {
        "Rails/RequireDependency"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // minimum_target_rails_version 6.0
        if !config.rails_version_at_least(6.0) {
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"require_dependency" {
            return;
        }

        // Must have at least one argument
        if call.arguments().is_none() {
            return;
        }

        // Receiverless call or Kernel.require_dependency
        let is_valid_receiver = match call.receiver() {
            None => true,
            Some(recv) => {
                if let Some(cr) = recv.as_constant_read_node() {
                    cr.name().as_slice() == b"Kernel"
                } else if let Some(cp) = recv.as_constant_path_node() {
                    if let Some(name) = cp.name() {
                        name.as_slice() == b"Kernel" && cp.parent().is_none()
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        };

        if !is_valid_receiver {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Do not use `require_dependency` with Zeitwerk mode.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_rails_fixture_tests!(RequireDependency, "cops/rails/require_dependency", 6.0);
}
