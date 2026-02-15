use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
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
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyleAlignWith", "either");
        let block_node = match node.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let closing_loc = block_node.closing_loc();

        // Only check do...end blocks, not brace blocks
        if closing_loc.as_slice() != b"end" {
            return Vec::new();
        }

        let opening_loc = block_node.opening_loc();
        let (opening_line, _) = source.offset_to_line_col(opening_loc.start_offset());

        // Find the indentation of the line containing the block opener.
        let bytes = source.as_bytes();
        let mut line_start = opening_loc.start_offset();
        while line_start > 0 && bytes[line_start - 1] != b'\n' {
            line_start -= 1;
        }
        let mut start_of_line_indent = 0;
        while line_start + start_of_line_indent < bytes.len()
            && bytes[line_start + start_of_line_indent] == b' '
        {
            start_of_line_indent += 1;
        }

        // Get the column of `do` keyword itself
        let (_, do_col) = source.offset_to_line_col(opening_loc.start_offset());

        let (end_line, end_col) = source.offset_to_line_col(closing_loc.start_offset());

        // Only flag if end is on a different line
        if end_line == opening_line {
            return Vec::new();
        }

        match style {
            "start_of_block" => {
                // `end` must align with `do`
                if end_col != do_col {
                    return vec![self.diagnostic(
                        source,
                        end_line,
                        end_col,
                        "Align `end` with `do`.".to_string(),
                    )];
                }
            }
            "start_of_line" => {
                // `end` must align with start of the line containing `do`
                if end_col != start_of_line_indent {
                    return vec![self.diagnostic(
                        source,
                        end_line,
                        end_col,
                        "Align `end` with the start of the line where the block is defined."
                            .to_string(),
                    )];
                }
            }
            _ => {
                // "either" (default): accept either alignment
                if end_col != start_of_line_indent && end_col != do_col {
                    return vec![self.diagnostic(
                        source,
                        end_line,
                        end_col,
                        "Align `end` with the start of the line where the block is defined."
                            .to_string(),
                    )];
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(BlockAlignment, "cops/layout/block_alignment");

    #[test]
    fn brace_block_no_offense() {
        let source = b"items.each { |x|\n  puts x\n}\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn start_of_block_style() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyleAlignWith".into(), serde_yml::Value::String("start_of_block".into())),
            ]),
            ..CopConfig::default()
        };
        // `end` aligned with start of line (col 0), not with `do` (col 11)
        let src = b"items.each do |x|\n  puts x\nend\n";
        let diags = run_cop_full_with_config(&BlockAlignment, src, config);
        assert_eq!(diags.len(), 1, "start_of_block should flag end not aligned with do");
        assert!(diags[0].message.contains("do"));
    }
}
