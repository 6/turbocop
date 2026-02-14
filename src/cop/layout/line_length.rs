use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct LineLength;

impl Cop for LineLength {
    fn name(&self) -> &'static str {
        "Layout/LineLength"
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig) -> Vec<Diagnostic> {
        let max = config
            .options
            .get("Max")
            .and_then(|v| v.as_u64())
            .unwrap_or(120) as usize;

        let mut diagnostics = Vec::new();
        for (i, line) in source.lines().enumerate() {
            if line.len() > max {
                diagnostics.push(Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location {
                        line: i + 1,
                        column: max,
                    },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: format!("Line is too long. [{}/{}]", line.len(), max),
                });
            }
        }
        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses, assert_cop_offenses};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses(
            &LineLength,
            include_bytes!("../../../testdata/cops/layout/line_length/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses(
            &LineLength,
            include_bytes!("../../../testdata/cops/layout/line_length/no_offense.rb"),
        );
    }

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
