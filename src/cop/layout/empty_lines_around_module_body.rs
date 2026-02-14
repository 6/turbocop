use crate::cop::util::line_at;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct EmptyLinesAroundModuleBody;

fn is_blank(line: &[u8]) -> bool {
    line.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r')
}

impl Cop for EmptyLinesAroundModuleBody {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundModuleBody"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let module_node = match node.as_module_node() {
            Some(m) => m,
            None => return Vec::new(),
        };

        let module_loc = module_node.module_keyword_loc();
        let (module_line, _) = source.offset_to_line_col(module_loc.start_offset());

        let end_loc = module_node.end_keyword_loc();
        let (end_line, _) = source.offset_to_line_col(end_loc.start_offset());

        // Skip single-line modules
        if module_line == end_line {
            return Vec::new();
        }

        let mut diagnostics = Vec::new();

        // Check for blank line after module keyword
        let after_module_line = module_line + 1;
        if let Some(line) = line_at(source, after_module_line) {
            if is_blank(line) && after_module_line < end_line {
                diagnostics.push(Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location {
                        line: after_module_line,
                        column: 0,
                    },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Extra empty line detected at module body beginning.".to_string(),
                });
            }
        }

        // Check for blank line before end keyword
        if end_line > 1 {
            let before_end_line = end_line - 1;
            if before_end_line > module_line {
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
                            message: "Extra empty line detected at module body end.".to_string(),
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
            &EmptyLinesAroundModuleBody,
            include_bytes!(
                "../../../testdata/cops/layout/empty_lines_around_module_body/offense.rb"
            ),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &EmptyLinesAroundModuleBody,
            include_bytes!(
                "../../../testdata/cops/layout/empty_lines_around_module_body/no_offense.rb"
            ),
        );
    }

    #[test]
    fn single_line_module_no_offense() {
        let src = b"module Foo; end\n";
        let diags = run_cop_full(&EmptyLinesAroundModuleBody, src);
        assert!(diags.is_empty(), "Single-line module should not trigger");
    }

    #[test]
    fn blank_line_at_both_ends() {
        let src = b"module Foo\n\n  def bar; end\n\nend\n";
        let diags = run_cop_full(&EmptyLinesAroundModuleBody, src);
        assert_eq!(diags.len(), 2, "Should flag both beginning and end blank lines");
    }
}
