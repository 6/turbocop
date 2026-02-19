use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct InitialIndentation;

impl Cop for InitialIndentation {
    fn name(&self) -> &'static str {
        "Layout/InitialIndentation"
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig, diagnostics: &mut Vec<Diagnostic>) {
        // Find the first non-empty line
        for (i, line) in source.lines().enumerate() {
            if line.is_empty() {
                continue;
            }
            if line[0] == b' ' || line[0] == b'\t' {
                diagnostics.push(self.diagnostic(
                    source,
                    i + 1,
                    0,
                    "Indentation of first line detected.".to_string(),
                ));
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
        InitialIndentation, "cops/layout/initial_indentation",
        space_indent = "space_indent.rb",
        tab_indent = "tab_indent.rb",
        deep_indent = "deep_indent.rb",
    );

    #[test]
    fn leading_blank_then_indented() {
        let source = SourceFile::from_bytes("test.rb", b"\n  x = 1\n".to_vec());
        let mut diags = Vec::new();
        InitialIndentation.check_lines(&source, &CopConfig::default(), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 2);
    }

    #[test]
    fn leading_blank_then_unindented() {
        let source = SourceFile::from_bytes("test.rb", b"\nx = 1\n".to_vec());
        let mut diags = Vec::new();
        InitialIndentation.check_lines(&source, &CopConfig::default(), &mut diags);
        assert!(diags.is_empty());
    }

    #[test]
    fn empty_file() {
        let source = SourceFile::from_bytes("test.rb", b"".to_vec());
        let mut diags = Vec::new();
        InitialIndentation.check_lines(&source, &CopConfig::default(), &mut diags);
        assert!(diags.is_empty());
    }
}
