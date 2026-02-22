use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct InitialIndentation;

impl Cop for InitialIndentation {
    fn name(&self) -> &'static str {
        "Layout/InitialIndentation"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Find the first non-empty line
        for (i, line) in source.lines().enumerate() {
            if line.is_empty() {
                continue;
            }
            if line[0] == b' ' || line[0] == b'\t' {
                let ws_len = line
                    .iter()
                    .take_while(|&&b| b == b' ' || b == b'\t')
                    .count();
                let mut diag = self.diagnostic(
                    source,
                    i + 1,
                    0,
                    "Indentation of first line detected.".to_string(),
                );
                if let Some(ref mut corr) = corrections {
                    if let Some(start) = source.line_col_to_offset(i + 1, 0) {
                        corr.push(crate::correction::Correction {
                            start,
                            end: start + ws_len,
                            replacement: String::new(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                }
                diagnostics.push(diag);
            }
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::source::SourceFile;

    crate::cop_scenario_fixture_tests!(
        InitialIndentation,
        "cops/layout/initial_indentation",
        space_indent = "space_indent.rb",
        tab_indent = "tab_indent.rb",
        deep_indent = "deep_indent.rb",
    );

    #[test]
    fn leading_blank_then_indented() {
        let source = SourceFile::from_bytes("test.rb", b"\n  x = 1\n".to_vec());
        let mut diags = Vec::new();
        InitialIndentation.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 2);
    }

    #[test]
    fn leading_blank_then_unindented() {
        let source = SourceFile::from_bytes("test.rb", b"\nx = 1\n".to_vec());
        let mut diags = Vec::new();
        InitialIndentation.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(diags.is_empty());
    }

    #[test]
    fn autocorrect_remove_spaces() {
        let input = b"  x = 1\n";
        let (_diags, corrections) =
            crate::testutil::run_cop_autocorrect(&InitialIndentation, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1\n");
    }

    #[test]
    fn autocorrect_remove_tabs() {
        let input = b"\tx = 1\n";
        let (_diags, corrections) =
            crate::testutil::run_cop_autocorrect(&InitialIndentation, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1\n");
    }

    #[test]
    fn empty_file() {
        let source = SourceFile::from_bytes("test.rb", b"".to_vec());
        let mut diags = Vec::new();
        InitialIndentation.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(diags.is_empty());
    }
}
