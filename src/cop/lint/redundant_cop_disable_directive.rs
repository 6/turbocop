use crate::cop::Cop;
use crate::diagnostic::Severity;

/// Checks for `# rubocop:disable` comments that can be removed.
///
/// This cop is special: it requires post-processing knowledge of which cops
/// actually fired offenses, so it cannot detect issues in a single-pass.
/// This struct exists so the cop name is registered and can be referenced
/// in configuration. The actual detection logic would be in the linter pipeline.
pub struct RedundantCopDisableDirective;

impl Cop for RedundantCopDisableDirective {
    fn name(&self) -> &'static str {
        "Lint/RedundantCopDisableDirective"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    // This cop is intentionally a no-op in check_lines/check_node/check_source.
    // A full implementation requires post-processing after all cops have run,
    // to determine which disable directives actually suppressed an offense.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cop_name() {
        assert_eq!(RedundantCopDisableDirective.name(), "Lint/RedundantCopDisableDirective");
    }

    #[test]
    fn default_severity_is_warning() {
        assert_eq!(RedundantCopDisableDirective.default_severity(), Severity::Warning);
    }

    #[test]
    fn no_offenses_on_clean_source() {
        use crate::testutil::run_cop_full;
        let source = b"x = 1\ny = 2\n";
        let diags = run_cop_full(&RedundantCopDisableDirective, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn no_offenses_with_disable_directive() {
        use crate::testutil::run_cop_full;
        let source = b"# rubocop:disable Layout/LineLength\nfoo\n# rubocop:enable Layout/LineLength\n";
        let diags = run_cop_full(&RedundantCopDisableDirective, source);
        // Stub cop produces no diagnostics
        assert!(diags.is_empty());
    }
}
