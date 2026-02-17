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

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Check for ConstantReadNode: `HashWithIndifferentAccess`
        if let Some(cr) = node.as_constant_read_node() {
            if cr.name().as_slice() == b"HashWithIndifferentAccess" {
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Avoid top-level `HashWithIndifferentAccess`.".to_string(),
                )];
            }
        }

        // Check for ConstantPathNode: `::HashWithIndifferentAccess`
        if let Some(cp) = node.as_constant_path_node() {
            if cp.parent().is_none() {
                if let Some(name) = cp.name() {
                    if name.as_slice() == b"HashWithIndifferentAccess" {
                        let loc = node.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        return vec![self.diagnostic(
                            source,
                            line,
                            column,
                            "Avoid top-level `HashWithIndifferentAccess`.".to_string(),
                        )];
                    }
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TopLevelHashWithIndifferentAccess, "cops/rails/top_level_hash_with_indifferent_access");
}
