use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct LeadingEmptyLines;

impl Cop for LeadingEmptyLines {
    fn name(&self) -> &'static str {
        "Layout/LeadingEmptyLines"
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig, diagnostics: &mut Vec<Diagnostic>) {
        let bytes = source.as_bytes();
        if bytes.is_empty() {
            return;
        }

        if bytes[0] == b'\n' {
            diagnostics.push(self.diagnostic(
                source,
                1,
                0,
                "Unnecessary blank line at the beginning of the source.".to_string(),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_scenario_fixture_tests!(
        LeadingEmptyLines, "cops/layout/leading_empty_lines",
        single_blank = "single_blank.rb",
        two_blanks = "two_blanks.rb",
        three_blanks = "three_blanks.rb",
    );

    #[test]
    fn empty_file() {
        let source = SourceFile::from_bytes("test.rb", b"".to_vec());
        let mut diags = Vec::new();
        LeadingEmptyLines.check_lines(&source, &CopConfig::default(), &mut diags);
        assert!(diags.is_empty());
    }
}
