use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct Tab;

impl Cop for Tab {
    fn name(&self) -> &'static str {
        "Style/Tab"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allow_named_tabs = config.get_bool("AllowNamedTabs", false);
        let _ = allow_named_tabs; // TODO: implement named tab handling

        let mut diagnostics = Vec::new();
        let mut byte_offset: usize = 0;

        for (i, line) in source.lines().enumerate() {
            let line_len = line.len() + 1; // +1 for newline
            if let Some(pos) = line.iter().position(|&b| b == b'\t') {
                let tab_offset = byte_offset + pos;
                // Skip tabs inside non-code regions (heredocs, strings, comments)
                if code_map.is_code(tab_offset) {
                    diagnostics.push(self.diagnostic(
                        source,
                        i + 1,
                        pos,
                        "Tab detected in indentation.".to_string(),
                    ));
                }
            }
            byte_offset += line_len;
        }
        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(Tab, "cops/style/tab");

    #[test]
    fn tab_at_start() {
        let diags = run_cop_full(&Tab, b"\tx = 1\n");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
        assert_eq!(diags[0].location.column, 0);
    }

    #[test]
    fn tab_in_middle() {
        let diags = run_cop_full(&Tab, b"x =\t1\n");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.column, 3);
    }

    #[test]
    fn no_tabs() {
        let diags = run_cop_full(&Tab, b"x = 1\n  y = 2\n");
        assert!(diags.is_empty());
    }

    #[test]
    fn multiple_lines_with_tabs() {
        let diags = run_cop_full(&Tab, b"\tx = 1\n\ty = 2\n");
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].location.line, 1);
        assert_eq!(diags[1].location.line, 2);
    }

    #[test]
    fn skip_tabs_in_heredoc() {
        let diags = run_cop_full(&Tab, b"x = <<~RUBY\n\thello\nRUBY\n");
        assert!(diags.is_empty(), "Should not fire on tabs inside heredoc");
    }

    #[test]
    fn skip_tabs_in_string() {
        let diags = run_cop_full(&Tab, b"x = \"hello\\tworld\"\n");
        assert!(diags.is_empty(), "Should not fire on tabs inside string");
    }
}
