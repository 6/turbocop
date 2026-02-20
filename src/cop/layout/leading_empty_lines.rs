use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct LeadingEmptyLines;

impl Cop for LeadingEmptyLines {
    fn name(&self) -> &'static str {
        "Layout/LeadingEmptyLines"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig, diagnostics: &mut Vec<Diagnostic>, mut corrections: Option<&mut Vec<crate::correction::Correction>>) {
        let bytes = source.as_bytes();
        if bytes.is_empty() {
            return;
        }

        if bytes[0] == b'\n' {
            let mut diag = self.diagnostic(
                source,
                1,
                0,
                "Unnecessary blank line at the beginning of the source.".to_string(),
            );
            if let Some(ref mut corr) = corrections {
                let end = bytes.iter().position(|&b| b != b'\n').unwrap_or(bytes.len());
                corr.push(crate::correction::Correction {
                    start: 0,
                    end,
                    replacement: String::new(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
            diagnostics.push(diag);
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
        LeadingEmptyLines.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(diags.is_empty());
    }

    #[test]
    fn autocorrect_single_blank() {
        let input = b"\nx = 1\n";
        let (diags, corrections) = crate::testutil::run_cop_autocorrect(&LeadingEmptyLines, input);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1\n");
    }

    #[test]
    fn autocorrect_multiple_blanks() {
        let input = b"\n\n\nx = 1\n";
        let (diags, corrections) = crate::testutil::run_cop_autocorrect(&LeadingEmptyLines, input);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1\n");
    }
}
