use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct SpaceInsideHashLiteralBraces;

impl Cop for SpaceInsideHashLiteralBraces {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsideHashLiteralBraces"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let hash = match node.as_hash_node() {
            Some(h) => h,
            None => return Vec::new(),
        };

        let opening = hash.opening_loc();
        let closing = hash.closing_loc();

        // Only check hash literals with { }
        if opening.as_slice() != b"{" || closing.as_slice() != b"}" {
            return Vec::new();
        }

        let bytes = source.as_bytes();
        let open_end = opening.end_offset();
        let close_start = closing.start_offset();

        // Skip empty hashes {}
        if close_start == open_end {
            return Vec::new();
        }

        // Skip multiline hashes (opening and closing on different lines)
        let (open_line, _) = source.offset_to_line_col(opening.start_offset());
        let (close_line, _) = source.offset_to_line_col(closing.start_offset());
        if open_line != close_line {
            return Vec::new();
        }

        let enforced = config
            .options
            .get("EnforcedStyle")
            .and_then(|v| v.as_str())
            .unwrap_or("space");

        let mut diagnostics = Vec::new();

        let space_after_open = bytes.get(open_end) == Some(&b' ');
        let space_before_close = close_start > 0 && bytes.get(close_start - 1) == Some(&b' ');

        match enforced {
            "space" => {
                if !space_after_open {
                    let (line, column) = source.offset_to_line_col(opening.start_offset());
                    diagnostics.push(Diagnostic {
                        path: source.path_str().to_string(),
                        location: Location { line, column },
                        severity: Severity::Convention,
                        cop_name: self.name().to_string(),
                        message: "Space inside { missing.".to_string(),
                    });
                }
                if !space_before_close {
                    let (line, column) = source.offset_to_line_col(closing.start_offset());
                    diagnostics.push(Diagnostic {
                        path: source.path_str().to_string(),
                        location: Location { line, column },
                        severity: Severity::Convention,
                        cop_name: self.name().to_string(),
                        message: "Space inside } missing.".to_string(),
                    });
                }
            }
            "no_space" => {
                if space_after_open {
                    let (line, column) = source.offset_to_line_col(open_end);
                    diagnostics.push(Diagnostic {
                        path: source.path_str().to_string(),
                        location: Location { line, column },
                        severity: Severity::Convention,
                        cop_name: self.name().to_string(),
                        message: "Space inside { detected.".to_string(),
                    });
                }
                if space_before_close {
                    let (line, column) = source.offset_to_line_col(close_start - 1);
                    diagnostics.push(Diagnostic {
                        path: source.path_str().to_string(),
                        location: Location { line, column },
                        severity: Severity::Convention,
                        cop_name: self.name().to_string(),
                        message: "Space inside } detected.".to_string(),
                    });
                }
            }
            _ => {}
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
            &SpaceInsideHashLiteralBraces,
            include_bytes!(
                "../../../testdata/cops/layout/space_inside_hash_literal_braces/offense.rb"
            ),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &SpaceInsideHashLiteralBraces,
            include_bytes!(
                "../../../testdata/cops/layout/space_inside_hash_literal_braces/no_offense.rb"
            ),
        );
    }
}
