use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct TrailingEmptyLines;

impl Cop for TrailingEmptyLines {
    fn name(&self) -> &'static str {
        "Layout/TrailingEmptyLines"
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig) -> Vec<Diagnostic> {
        let bytes = source.as_bytes();
        if bytes.is_empty() {
            return Vec::new();
        }

        if *bytes.last().unwrap() != b'\n' {
            // Missing final newline
            let line_count = bytes.iter().filter(|&&b| b == b'\n').count() + 1;
            return vec![self.diagnostic(
                source,
                line_count,
                0,
                "Final newline missing.".to_string(),
            )];
        }

        // Check for trailing blank lines (content ends with \n\n)
        if bytes.len() >= 2 && bytes[bytes.len() - 2] == b'\n' {
            // Walk backwards to find the first extra blank line
            let mut end = bytes.len() - 1; // skip the final expected \n
            while end > 0 && bytes[end - 1] == b'\n' {
                end -= 1;
            }
            // end is at the position of the first byte of the trailing \n sequence
            // The line after the last content line is the first trailing blank line
            let line_num = bytes[..end].iter().filter(|&&b| b == b'\n').count() + 2;
            return vec![self.diagnostic(
                source,
                line_num,
                0,
                "Trailing blank line detected.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::source::SourceFile;

    crate::cop_scenario_fixture_tests!(
        TrailingEmptyLines, "cops/layout/trailing_empty_lines",
        missing_newline = "missing_newline.rb",
        trailing_blank = "trailing_blank.rb",
        multiple_trailing = "multiple_trailing.rb",
    );

    #[test]
    fn missing_final_newline() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1".to_vec());
        let diags = TrailingEmptyLines.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
        assert_eq!(diags[0].message, "Final newline missing.");
    }

    #[test]
    fn missing_final_newline_multiline() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\ny = 2".to_vec());
        let diags = TrailingEmptyLines.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 2);
        assert_eq!(diags[0].message, "Final newline missing.");
    }

    #[test]
    fn trailing_blank_line() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\n\n".to_vec());
        let diags = TrailingEmptyLines.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 2);
        assert_eq!(diags[0].message, "Trailing blank line detected.");
    }

    #[test]
    fn multiple_trailing_blank_lines() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\n\n\n\n".to_vec());
        let diags = TrailingEmptyLines.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 2);
        assert_eq!(diags[0].message, "Trailing blank line detected.");
    }

    #[test]
    fn proper_final_newline() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\n".to_vec());
        let diags = TrailingEmptyLines.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }

    #[test]
    fn multiline_proper() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\ny = 2\n".to_vec());
        let diags = TrailingEmptyLines.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }

    #[test]
    fn empty_file() {
        let source = SourceFile::from_bytes("test.rb", b"".to_vec());
        let diags = TrailingEmptyLines.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }
}
