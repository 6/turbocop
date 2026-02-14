use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct BlockAlignment;

impl Cop for BlockAlignment {
    fn name(&self) -> &'static str {
        "Layout/BlockAlignment"
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

        let closing_loc = block_node.closing_loc();

        // Only check do...end blocks, not brace blocks
        if closing_loc.as_slice() != b"end" {
            return Vec::new();
        }

        // Get the opening line's indentation (column of first non-whitespace on
        // the line that contains the block opener / call)
        let opening_loc = block_node.opening_loc();
        let (opening_line, _) = source.offset_to_line_col(opening_loc.start_offset());

        // Find the indentation of the line containing the block opener.
        // We walk back from the opening_loc to find the start-of-line indentation.
        let bytes = source.as_bytes();
        let mut line_start = opening_loc.start_offset();
        while line_start > 0 && bytes[line_start - 1] != b'\n' {
            line_start -= 1;
        }
        let mut indent = 0;
        while line_start + indent < bytes.len() && bytes[line_start + indent] == b' ' {
            indent += 1;
        }

        let (end_line, end_col) = source.offset_to_line_col(closing_loc.start_offset());

        // Only flag if end is on a different line and misaligned
        if end_line != opening_line && end_col != indent {
            return vec![Diagnostic {
                path: source.path_str().to_string(),
                location: Location {
                    line: end_line,
                    column: end_col,
                },
                severity: Severity::Convention,
                cop_name: self.name().to_string(),
                message: "Align `end` with the start of the line where the block is defined."
                    .to_string(),
            }];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full, run_cop_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &BlockAlignment,
            include_bytes!("../../../testdata/cops/layout/block_alignment/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &BlockAlignment,
            include_bytes!("../../../testdata/cops/layout/block_alignment/no_offense.rb"),
        );
    }

    #[test]
    fn brace_block_no_offense() {
        let source = b"items.each { |x|\n  puts x\n}\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(diags.is_empty());
    }
}
