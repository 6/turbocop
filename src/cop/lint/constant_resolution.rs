use crate::cop::node_type::{CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks that certain constants are fully qualified.
/// Disabled by default; useful for gems to avoid conflicts.
pub struct ConstantResolution;

impl Cop for ConstantResolution {
    fn name(&self) -> &'static str {
        "Lint/ConstantResolution"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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
        // Check for unqualified constant (no parent scope, just `Foo` not `::Foo`)
        // ConstantPathNode (qualified like Foo::Bar or ::Foo) is already resolved,
        // so we only flag simple ConstantReadNode references.
        if node.as_constant_path_node().is_some() {
            return;
        }

        let const_node = match node.as_constant_read_node() {
            Some(n) => n,
            None => return,
        };

        let name = std::str::from_utf8(const_node.name().as_slice()).unwrap_or("");

        // Check Only/Ignore config.
        // When Only is explicitly empty (`Only: []`, the RuboCop default), nothing
        // should be checked — return early.  When Only is absent (not configured),
        // flag all unqualified constants (the cop was explicitly enabled without
        // restricting which constants to check).
        let only = config.get_string_array("Only");
        let ignore = config.get_string_array("Ignore").unwrap_or_default();

        match &only {
            Some(list) if list.is_empty() => return, // Explicit empty Only = nothing to check
            Some(list) => {
                if !list.contains(&name.to_string()) {
                    return;
                }
            }
            None => {} // Only not configured; flag all unqualified constants
        }
        if ignore.contains(&name.to_string()) {
            return;
        }

        let loc = const_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Fully qualify this constant to avoid possibly ambiguous resolution.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ConstantResolution, "cops/lint/constant_resolution");
}
