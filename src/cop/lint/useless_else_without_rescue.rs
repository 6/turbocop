use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct UselessElseWithoutRescue;

impl Cop for UselessElseWithoutRescue {
    fn name(&self) -> &'static str {
        "Lint/UselessElseWithoutRescue"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let begin_node = match node.as_begin_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Check if there's an else clause but no rescue clause
        if begin_node.else_clause().is_none() {
            return Vec::new();
        }

        if begin_node.rescue_clause().is_some() {
            return Vec::new(); // Has rescue, so else is fine
        }

        // This is an `else` without `rescue` in a begin..end
        // Note: This is a syntax error in Ruby 2.6+, but we still flag it
        let else_clause = begin_node.else_clause().unwrap();
        let loc = else_clause.else_keyword_loc();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "`else` without `rescue` is useless.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // This syntax is invalid in modern Ruby, so standard fixture tests won't work.
    // The cop is kept for completeness but won't trigger on valid Ruby code.
    // Test with basic assertions instead.

    #[test]
    fn no_offense_with_rescue() {
        let source = b"begin\n  x\nrescue\n  y\nelse\n  z\nend\n";
        let diags = crate::testutil::run_cop(&UselessElseWithoutRescue, source);
        assert!(diags.is_empty(), "Should not flag else with rescue");
    }

    #[test]
    fn no_offense_without_else() {
        let source = b"begin\n  x\nrescue\n  y\nend\n";
        let diags = crate::testutil::run_cop(&UselessElseWithoutRescue, source);
        assert!(diags.is_empty(), "Should not flag begin without else");
    }

    #[test]
    fn no_offense_regular_begin() {
        let source = b"begin\n  x\nend\n";
        let diags = crate::testutil::run_cop(&UselessElseWithoutRescue, source);
        assert!(diags.is_empty());
    }
}
