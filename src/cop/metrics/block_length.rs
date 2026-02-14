use crate::cop::util::count_body_lines;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct BlockLength;

impl Cop for BlockLength {
    fn name(&self) -> &'static str {
        "Metrics/BlockLength"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let block_node = match node.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let max = config
            .options
            .get("Max")
            .and_then(|v| v.as_u64())
            .unwrap_or(25) as usize;

        let count_comments = config
            .options
            .get("CountComments")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let start_offset = block_node.opening_loc().start_offset();
        let end_offset = block_node.closing_loc().start_offset();
        let count = count_body_lines(source, start_offset, end_offset, count_comments);

        if count > max {
            let (line, column) = source.offset_to_line_col(start_offset);
            return vec![Diagnostic {
                path: source.path_str().to_string(),
                location: Location { line, column },
                severity: Severity::Convention,
                cop_name: self.name().to_string(),
                message: format!("Block has too many lines. [{count}/{max}]"),
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
            &BlockLength,
            include_bytes!("../../../testdata/cops/metrics/block_length/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &BlockLength,
            include_bytes!("../../../testdata/cops/metrics/block_length/no_offense.rb"),
        );
    }
}
