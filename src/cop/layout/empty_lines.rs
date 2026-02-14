use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct EmptyLines;

impl Cop for EmptyLines {
    fn name(&self) -> &'static str {
        "Layout/EmptyLines"
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig) -> Vec<Diagnostic> {
        let max = config
            .options
            .get("Max")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as usize;

        let mut diagnostics = Vec::new();
        let mut consecutive_blanks = 0;

        for (i, line) in source.lines().enumerate() {
            if line.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r') {
                consecutive_blanks += 1;
                if consecutive_blanks > max {
                    diagnostics.push(Diagnostic {
                        path: source.path_str().to_string(),
                        location: Location {
                            line: i + 1,
                            column: 0,
                        },
                        severity: Severity::Convention,
                        cop_name: self.name().to_string(),
                        message: "Extra blank line detected.".to_string(),
                    });
                }
            } else {
                consecutive_blanks = 0;
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
            &EmptyLines,
            include_bytes!("../../../testdata/cops/layout/empty_lines/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses(
            &EmptyLines,
            include_bytes!("../../../testdata/cops/layout/empty_lines/no_offense.rb"),
        );
    }

    #[test]
    fn config_max_2() {
        use std::collections::HashMap;
        use crate::testutil::{run_cop_with_config, run_cop};

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(2.into()))]),
            ..CopConfig::default()
        };
        // 3 consecutive blank lines should trigger with Max:2
        let source = b"x = 1\n\n\n\ny = 2\n";
        let diags = run_cop_with_config(&EmptyLines, source, config.clone());
        assert!(!diags.is_empty(), "Should fire with Max:2 on 3 consecutive blank lines");

        // 2 consecutive blank lines should NOT trigger with Max:2
        let source2 = b"x = 1\n\n\ny = 2\n";
        let diags2 = run_cop_with_config(&EmptyLines, source2, config);
        assert!(diags2.is_empty(), "Should not fire on 2 consecutive blank lines with Max:2");

        // 2 consecutive blank lines SHOULD trigger with default Max:1
        let diags3 = run_cop(&EmptyLines, source2);
        assert!(!diags3.is_empty(), "Should fire with default Max:1 on 2 consecutive blank lines");
    }
}
