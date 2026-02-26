use crate::cop::node_type::INTERPOLATED_STRING_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct TripleQuotes;

impl Cop for TripleQuotes {
    fn name(&self) -> &'static str {
        "Lint/TripleQuotes"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[INTERPOLATED_STRING_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let interp = match node.as_interpolated_string_node() {
            Some(n) => n,
            None => return,
        };

        // Check if any child is an empty StringNode (indicating implicit concatenation
        // with empty strings, which is what triple quotes produce).
        let has_empty_str = interp.parts().iter().any(|part| {
            if let Some(s) = part.as_string_node() {
                s.unescaped().is_empty()
            } else {
                false
            }
        });

        if !has_empty_str {
            return;
        }

        // Check if the source starts with 3+ quote characters
        let loc = interp.location();
        let src = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
        let quote_count = src.iter().take_while(|&&b| b == b'"' || b == b'\'').count();

        if quote_count < 3 {
            return;
        }

        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Triple quotes found. Did you mean to use a heredoc?".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TripleQuotes, "cops/lint/triple_quotes");

    #[test]
    fn skip_in_heredoc() {
        let source = b"x = <<~RUBY\n  \"\"\"\n  foo\n  \"\"\"\nRUBY\n";
        let diags = crate::testutil::run_cop_full(&TripleQuotes, source);
        assert!(
            diags.is_empty(),
            "Should not fire on triple quotes inside heredoc"
        );
    }
}
