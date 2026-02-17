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

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Check for unqualified constant (no parent scope, just `Foo` not `::Foo`)
        let const_node = match node.as_constant_read_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let name = std::str::from_utf8(const_node.name().as_slice()).unwrap_or("");

        // Check Only/Ignore config
        let only = config.get_string_array("Only").unwrap_or_default();
        let ignore = config.get_string_array("Ignore").unwrap_or_default();

        if !only.is_empty() && !only.contains(&name.to_string()) {
            return Vec::new();
        }
        if ignore.contains(&name.to_string()) {
            return Vec::new();
        }

        let loc = const_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Fully qualify this constant to avoid possibly ambiguous resolution.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ConstantResolution, "cops/lint/constant_resolution");
}
