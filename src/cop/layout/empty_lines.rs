use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct EmptyLines;

impl Cop for EmptyLines {
    fn name(&self) -> &'static str {
        "Layout/EmptyLines"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let max = config.get_usize("Max", 1);

        let mut consecutive_blanks = 0;
        let mut byte_offset: usize = 0;

        for (i, line) in source.lines().enumerate() {
            let line_len = line.len() + 1; // +1 for newline
            if line.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r') {
                // Skip blank lines inside non-code regions (heredocs, strings)
                if !code_map.is_code(byte_offset) {
                    byte_offset += line_len;
                    consecutive_blanks = 0;
                    continue;
                }
                consecutive_blanks += 1;
                if consecutive_blanks > max {
                    diagnostics.push(self.diagnostic(
                        source,
                        i + 1,
                        0,
                        "Extra blank line detected.".to_string(),
                    ));
                }
            } else {
                consecutive_blanks = 0;
            }
            byte_offset += line_len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{run_cop_full, run_cop_full_with_config};

    crate::cop_fixture_tests!(EmptyLines, "cops/layout/empty_lines");

    #[test]
    fn config_max_2() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(2.into()))]),
            ..CopConfig::default()
        };
        // 3 consecutive blank lines should trigger with Max:2
        let source = b"x = 1\n\n\n\ny = 2\n";
        let diags = run_cop_full_with_config(&EmptyLines, source, config.clone());
        assert!(!diags.is_empty(), "Should fire with Max:2 on 3 consecutive blank lines");

        // 2 consecutive blank lines should NOT trigger with Max:2
        let source2 = b"x = 1\n\n\ny = 2\n";
        let diags2 = run_cop_full_with_config(&EmptyLines, source2, config);
        assert!(diags2.is_empty(), "Should not fire on 2 consecutive blank lines with Max:2");

        // 2 consecutive blank lines SHOULD trigger with default Max:1
        let diags3 = run_cop_full(&EmptyLines, source2);
        assert!(!diags3.is_empty(), "Should fire with default Max:1 on 2 consecutive blank lines");
    }

    #[test]
    fn skip_blanks_in_heredoc() {
        let source = b"x = <<~RUBY\n  foo\n\n\n  bar\nRUBY\n";
        let diags = run_cop_full(&EmptyLines, source);
        assert!(diags.is_empty(), "Should not fire on blank lines inside heredoc");
    }
}
