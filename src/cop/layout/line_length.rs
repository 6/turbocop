use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct LineLength;

impl Cop for LineLength {
    fn name(&self) -> &'static str {
        "Layout/LineLength"
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig) -> Vec<Diagnostic> {
        let max = config.get_usize("Max", 120);

        let mut diagnostics = Vec::new();
        for (i, line) in source.lines().enumerate() {
            if line.len() > max {
                diagnostics.push(self.diagnostic(
                    source,
                    i + 1,
                    max,
                    format!("Line is too long. [{}/{}]", line.len(), max),
                ));
            }
        }
        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(LineLength, "cops/layout/line_length");

    #[test]
    fn custom_max() {
        use std::collections::HashMap;
        let mut options = HashMap::new();
        options.insert("Max".to_string(), serde_yml::Value::Number(10.into()));
        let config = CopConfig {
            options,
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes("test.rb", b"short\nthis line is longer than ten\n".to_vec());
        let diags = LineLength.check_lines(&source, &config);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 2);
        assert_eq!(diags[0].location.column, 10);
        assert_eq!(diags[0].message, "Line is too long. [28/10]");
    }

    #[test]
    fn exact_max_no_offense() {
        use std::collections::HashMap;
        let mut options = HashMap::new();
        options.insert("Max".to_string(), serde_yml::Value::Number(5.into()));
        let config = CopConfig {
            options,
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes("test.rb", b"12345\n".to_vec());
        let diags = LineLength.check_lines(&source, &config);
        assert!(diags.is_empty());
    }
}
