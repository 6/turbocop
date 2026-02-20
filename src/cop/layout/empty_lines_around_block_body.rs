use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::BLOCK_NODE;

pub struct EmptyLinesAroundBlockBody;

impl Cop for EmptyLinesAroundBlockBody {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundBlockBody"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "no_empty_lines");
        let block_node = match node.as_block_node() {
            Some(b) => b,
            None => return,
        };

        match style {
            "empty_lines" => {
                // Require empty lines at beginning and end of block body
                diagnostics.extend(util::check_missing_empty_lines_around_body_with_corrections(
                    self.name(),
                    source,
                    block_node.opening_loc().start_offset(),
                    block_node.closing_loc().start_offset(),
                    "block",
                    corrections,
                ));
            }
            _ => {
                // "no_empty_lines" (default): flag extra empty lines
                diagnostics.extend(util::check_empty_lines_around_body_with_corrections(
                    self.name(),
                    source,
                    block_node.opening_loc().start_offset(),
                    block_node.closing_loc().start_offset(),
                    "block",
                    corrections,
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        EmptyLinesAroundBlockBody,
        "cops/layout/empty_lines_around_block_body"
    );
    crate::cop_autocorrect_fixture_tests!(
        EmptyLinesAroundBlockBody,
        "cops/layout/empty_lines_around_block_body"
    );

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

    #[test]
    fn empty_lines_style_requires_blank_lines() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("empty_lines".into())),
            ]),
            ..CopConfig::default()
        };
        // Block WITHOUT blank lines at beginning/end
        let src = b"items.each do |x|\n  puts x\nend\n";
        let diags = run_cop_full_with_config(&EmptyLinesAroundBlockBody, src, config);
        assert_eq!(diags.len(), 2, "empty_lines style should require blank lines at both ends");
    }

    #[test]
    fn empty_lines_style_accepts_blank_lines() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("empty_lines".into())),
            ]),
            ..CopConfig::default()
        };
        // Block WITH blank lines at beginning/end
        let src = b"items.each do |x|\n\n  puts x\n\nend\n";
        let diags = run_cop_full_with_config(&EmptyLinesAroundBlockBody, src, config);
        assert!(diags.is_empty(), "empty_lines style should accept blank lines");
    }
}
