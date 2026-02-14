use crate::cop::util::line_at;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct EmptyLinesAroundBlockBody;

fn is_blank(line: &[u8]) -> bool {
    line.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r')
}

impl Cop for EmptyLinesAroundBlockBody {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundBlockBody"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let block_node = match node.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let opening_loc = block_node.opening_loc();
        let closing_loc = block_node.closing_loc();

        let (opening_line, _) = source.offset_to_line_col(opening_loc.start_offset());
        let (closing_line, _) = source.offset_to_line_col(closing_loc.start_offset());

        // Skip single-line blocks
        if opening_line == closing_line {
            return Vec::new();
        }

        let mut diagnostics = Vec::new();

        // Check for blank line after opening (do / {)
        let after_opening_line = opening_line + 1;
        if let Some(line) = line_at(source, after_opening_line) {
            if is_blank(line) && after_opening_line < closing_line {
                diagnostics.push(Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location {
                        line: after_opening_line,
                        column: 0,
                    },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Extra empty line detected at block body beginning.".to_string(),
                });
            }
        }

        // Check for blank line before closing (end / })
        if closing_line > 1 {
            let before_closing_line = closing_line - 1;
            if before_closing_line > opening_line {
                if let Some(line) = line_at(source, before_closing_line) {
                    if is_blank(line) {
                        diagnostics.push(Diagnostic {
                            path: source.path_str().to_string(),
                            location: Location {
                                line: before_closing_line,
                                column: 0,
                            },
                            severity: Severity::Convention,
                            cop_name: self.name().to_string(),
                            message: "Extra empty line detected at block body end.".to_string(),
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
            &EmptyLinesAroundBlockBody,
            include_bytes!(
                "../../../testdata/cops/layout/empty_lines_around_block_body/offense.rb"
            ),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &EmptyLinesAroundBlockBody,
            include_bytes!(
                "../../../testdata/cops/layout/empty_lines_around_block_body/no_offense.rb"
            ),
        );
    }

    #[test]
    fn single_line_block_no_offense() {
        let src = b"[1, 2, 3].each { |x| puts x }\n";
        let diags = run_cop_full(&EmptyLinesAroundBlockBody, src);
        assert!(diags.is_empty(), "Single-line block should not trigger");
    }

    #[test]
    fn do_end_block_with_blank_lines() {
        let src = b"items.each do |x|\n\n  puts x\n\nend\n";
        let diags = run_cop_full(&EmptyLinesAroundBlockBody, src);
        assert_eq!(diags.len(), 2, "Should flag both beginning and end blank lines");
    }
}
