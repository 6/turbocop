use crate::cop::util::line_at;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct EmptyLinesAroundMethodBody;

fn is_blank(line: &[u8]) -> bool {
    line.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r')
}

impl Cop for EmptyLinesAroundMethodBody {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundMethodBody"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        // Skip endless methods (no end keyword)
        let end_loc = match def_node.end_keyword_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let def_loc = def_node.def_keyword_loc();
        let (def_line, _) = source.offset_to_line_col(def_loc.start_offset());
        let (end_line, _) = source.offset_to_line_col(end_loc.start_offset());

        // Skip single-line methods
        if def_line == end_line {
            return Vec::new();
        }

        let mut diagnostics = Vec::new();

        // Check for blank line after def keyword
        let after_def_line = def_line + 1;
        if let Some(line) = line_at(source, after_def_line) {
            if is_blank(line) && after_def_line < end_line {
                diagnostics.push(Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location {
                        line: after_def_line,
                        column: 0,
                    },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Extra empty line detected at method body beginning.".to_string(),
                });
            }
        }

        // Check for blank line before end keyword
        if end_line > 1 {
            let before_end_line = end_line - 1;
            if before_end_line > def_line {
                if let Some(line) = line_at(source, before_end_line) {
                    if is_blank(line) {
                        diagnostics.push(Diagnostic {
                            path: source.path_str().to_string(),
                            location: Location {
                                line: before_end_line,
                                column: 0,
                            },
                            severity: Severity::Convention,
                            cop_name: self.name().to_string(),
                            message: "Extra empty line detected at method body end.".to_string(),
                        });
                    }
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full, run_cop_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &EmptyLinesAroundMethodBody,
            include_bytes!(
                "../../../testdata/cops/layout/empty_lines_around_method_body/offense.rb"
            ),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &EmptyLinesAroundMethodBody,
            include_bytes!(
                "../../../testdata/cops/layout/empty_lines_around_method_body/no_offense.rb"
            ),
        );
    }

    #[test]
    fn single_line_def_no_offense() {
        let src = b"def foo; 42; end\n";
        let diags = run_cop_full(&EmptyLinesAroundMethodBody, src);
        assert!(diags.is_empty(), "Single-line def should not trigger");
    }

    #[test]
    fn endless_method_no_offense() {
        let src = b"def foo = 42\n";
        let diags = run_cop_full(&EmptyLinesAroundMethodBody, src);
        assert!(diags.is_empty(), "Endless method should not trigger");
    }
}
