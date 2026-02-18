use crate::cop::Cop;
use crate::diagnostic::Severity;

/// Checks for syntax errors.
///
/// This is a stub cop. Prism handles syntax error reporting during parsing,
/// and the rblint pipeline reports parse errors separately. This cop exists
/// so the cop name is registered and can be referenced in configuration.
pub struct Syntax;

impl Cop for Syntax {
    fn name(&self) -> &'static str {
        "Lint/Syntax"
    }

    fn default_severity(&self) -> Severity {
        Severity::Fatal
    }

    // Syntax errors are reported by the parser (Prism), not by this cop.
    // This struct exists for configuration compatibility with RuboCop.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cop_name() {
        assert_eq!(Syntax.name(), "Lint/Syntax");
    }

    #[test]
    fn default_severity_is_fatal() {
        assert_eq!(Syntax.default_severity(), Severity::Fatal);
    }

    #[test]
    fn no_offenses_on_valid_source() {
        use crate::testutil::run_cop_full;
        let source = b"x = 1\ny = 2\n";
        let diags = run_cop_full(&Syntax, source);
        assert!(diags.is_empty());
    }
}
