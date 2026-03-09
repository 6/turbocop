use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// No-op: `YAML.load` is safe since Ruby 3.1 (Psych 4 default), making this cop obsolete.
/// Retained for configuration compatibility only.
pub struct YamlLoad;

impl Cop for YamlLoad {
    fn name(&self) -> &'static str {
        "Security/YAMLLoad"
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
        let source = b"YAML.load(data)\nPsych.load(data)\n::YAML.load(x)\n";
        let diags = crate::testutil::run_cop(&YamlLoad, source);
        assert!(
            diags.is_empty(),
            "No-op cop should never produce diagnostics"
        );
    }
}
