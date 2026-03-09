use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// No-op: `else` without `rescue` is a syntax error in Ruby 3.4+, making this cop obsolete.
/// Retained for configuration compatibility only.
pub struct UselessElseWithoutRescue;

impl Cop for UselessElseWithoutRescue {
    fn name(&self) -> &'static str {
        "Lint/UselessElseWithoutRescue"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[]
    }

    fn check_node(
        &self,
        _source: &SourceFile,
        _node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        _diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_fires() {
        let source = b"begin\n  x\nrescue\n  y\nelse\n  z\nend\n";
        let diags = crate::testutil::run_cop(&UselessElseWithoutRescue, source);
        assert!(
            diags.is_empty(),
            "No-op cop should never produce diagnostics"
        );
    }
}
