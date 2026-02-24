use crate::cop::node_type::{CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct TopLevelHashWithIndifferentAccess;

impl Cop for TopLevelHashWithIndifferentAccess {
    fn name(&self) -> &'static str {
        "Rails/TopLevelHashWithIndifferentAccess"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CONSTANT_PATH_NODE, CONSTANT_READ_NODE]
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
        // minimum_target_rails_version 5.1
        if !config.rails_version_at_least(5.1) {
            return;
        }

        // Check for ConstantReadNode: `HashWithIndifferentAccess`
        if let Some(cr) = node.as_constant_read_node() {
            if cr.name().as_slice() == b"HashWithIndifferentAccess" {
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Avoid top-level `HashWithIndifferentAccess`.".to_string(),
                ));
            }
        }

        // Check for ConstantPathNode: `::HashWithIndifferentAccess`
        if let Some(cp) = node.as_constant_path_node() {
            if cp.parent().is_none() {
                if let Some(name) = cp.name() {
                    if name.as_slice() == b"HashWithIndifferentAccess" {
                        let loc = node.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Avoid top-level `HashWithIndifferentAccess`.".to_string(),
                        ));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_rails_fixture_tests!(
        TopLevelHashWithIndifferentAccess,
        "cops/rails/top_level_hash_with_indifferent_access",
        5.1
    );
}
