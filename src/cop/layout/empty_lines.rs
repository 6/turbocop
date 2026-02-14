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
}
