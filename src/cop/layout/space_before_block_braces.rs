use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct SpaceBeforeBlockBraces;

impl Cop for SpaceBeforeBlockBraces {
    fn name(&self) -> &'static str {
        "Layout/SpaceBeforeBlockBraces"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let block = match node.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let opening = block.opening_loc();

        // Only check { blocks, not do...end
        if opening.as_slice() != b"{" {
            return Vec::new();
        }

        let bytes = source.as_bytes();
        let before = opening.start_offset();
        if before > 0 && bytes[before - 1] != b' ' {
            let (line, column) = source.offset_to_line_col(before);
            return vec![Diagnostic {
                path: source.path_str().to_string(),
                location: Location { line, column },
                severity: Severity::Convention,
                cop_name: self.name().to_string(),
                message: "Space missing to the left of {.".to_string(),
            }];
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &SpaceBeforeBlockBraces,
            include_bytes!(
                "../../../testdata/cops/layout/space_before_block_braces/offense.rb"
            ),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &SpaceBeforeBlockBraces,
            include_bytes!(
                "../../../testdata/cops/layout/space_before_block_braces/no_offense.rb"
            ),
        );
    }
}
