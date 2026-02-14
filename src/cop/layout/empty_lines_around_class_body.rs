use crate::cop::util::line_at;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct EmptyLinesAroundClassBody;

fn is_blank(line: &[u8]) -> bool {
    line.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r')
}

impl Cop for EmptyLinesAroundClassBody {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundClassBody"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let class_node = match node.as_class_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let class_loc = class_node.class_keyword_loc();
        let (class_line, _) = source.offset_to_line_col(class_loc.start_offset());

        let end_loc = class_node.end_keyword_loc();
        let (end_line, _) = source.offset_to_line_col(end_loc.start_offset());

        // Skip single-line classes
        if class_line == end_line {
            return Vec::new();
        }

        let mut diagnostics = Vec::new();

        // Check for blank line after class keyword
        let after_class_line = class_line + 1;
        if let Some(line) = line_at(source, after_class_line) {
            if is_blank(line) && after_class_line < end_line {
                diagnostics.push(Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location {
                        line: after_class_line,
                        column: 0,
                    },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Extra empty line detected at class body beginning.".to_string(),
                });
            }
        }

        // Check for blank line before end keyword
        if end_line > 1 {
            let before_end_line = end_line - 1;
            if before_end_line > class_line {
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
                            message: "Extra empty line detected at class body end.".to_string(),
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
            &EmptyLinesAroundClassBody,
            include_bytes!(
                "../../../testdata/cops/layout/empty_lines_around_class_body/offense.rb"
            ),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &EmptyLinesAroundClassBody,
            include_bytes!(
                "../../../testdata/cops/layout/empty_lines_around_class_body/no_offense.rb"
            ),
        );
    }

    #[test]
    fn single_line_class_no_offense() {
        let src = b"class Foo; end\n";
        let diags = run_cop_full(&EmptyLinesAroundClassBody, src);
        assert!(diags.is_empty(), "Single-line class should not trigger");
    }

    #[test]
    fn blank_line_at_both_ends() {
        let src = b"class Foo\n\n  def bar; end\n\nend\n";
        let diags = run_cop_full(&EmptyLinesAroundClassBody, src);
        assert_eq!(diags.len(), 2, "Should flag both beginning and end blank lines");
    }
}
