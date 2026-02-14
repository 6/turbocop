use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct SpaceInsideParens;

impl Cop for SpaceInsideParens {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsideParens"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let parens = match node.as_parentheses_node() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let bytes = source.as_bytes();
        let open_end = parens.opening_loc().end_offset();
        let close_start = parens.closing_loc().start_offset();

        // Skip empty parens ()
        if close_start == open_end {
            return Vec::new();
        }

        let mut diagnostics = Vec::new();

        // Check space after (
        if let Some(&b' ') = bytes.get(open_end) {
            // Skip if the next non-space is a newline (multiline)
            if bytes.get(open_end) != Some(&b'\n') && bytes.get(open_end) != Some(&b'\r') {
                let (line, column) = source.offset_to_line_col(open_end);
                diagnostics.push(Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location { line, column },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Space inside parentheses detected.".to_string(),
                });
            }
        }

        // Check space before )
        if close_start > 0 {
            let before = bytes.get(close_start - 1).copied();
            if before == Some(b' ') {
                // Skip if the char before the space is a newline (multiline)
                let before_space = if close_start >= 2 {
                    bytes.get(close_start - 2).copied()
                } else {
                    None
                };
                if before_space != Some(b'\n') && before_space != Some(b'\r') {
                    let (line, column) = source.offset_to_line_col(close_start - 1);
                    diagnostics.push(Diagnostic {
                        path: source.path_str().to_string(),
                        location: Location { line, column },
                        severity: Severity::Convention,
                        cop_name: self.name().to_string(),
                        message: "Space inside parentheses detected.".to_string(),
                    });
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &SpaceInsideParens,
            include_bytes!("../../../testdata/cops/layout/space_inside_parens/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &SpaceInsideParens,
            include_bytes!("../../../testdata/cops/layout/space_inside_parens/no_offense.rb"),
        );
    }
}
