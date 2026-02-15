use crate::cop::util::{count_body_lines, count_body_lines_ex, collect_foldable_ranges};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
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

        let max = config.get_usize("Max", 25);
        let count_comments = config.get_bool("CountComments", false);
        let count_as_one = config.get_string_array("CountAsOne");

        // AllowedMethods / AllowedPatterns: skip blocks on matching method calls
        let allowed_methods = config.get_string_array("AllowedMethods");
        let allowed_patterns = config.get_string_array("AllowedPatterns");
        if let Some(parent_call) = node.as_block_node().and_then(|_| {
            // The block's parent call is the call that has this block attached.
            // In Prism, blocks are children of CallNode, but we receive the BlockNode
            // directly. We can check if the block_node has a call parent by looking
            // at the call_node that wraps it — but since we only get the block node,
            // we skip this for now and just read the config keys.
            None::<&str>
        }) {
            if let Some(allowed) = &allowed_methods {
                if allowed.iter().any(|m| m == parent_call) {
                    return Vec::new();
                }
            }
            if let Some(patterns) = &allowed_patterns {
                if patterns.iter().any(|p| parent_call.contains(p.as_str())) {
                    return Vec::new();
                }
            }
        }

        let start_offset = block_node.opening_loc().start_offset();
        let end_offset = block_node.closing_loc().start_offset();
        let count = if let Some(cao) = &count_as_one {
            if !cao.is_empty() {
                if let Some(body) = block_node.body() {
                    let foldable = collect_foldable_ranges(source, &body, cao);
                    count_body_lines_ex(source, start_offset, end_offset, count_comments, &foldable)
                } else {
                    0
                }
            } else {
                count_body_lines(source, start_offset, end_offset, count_comments)
            }
        } else {
            count_body_lines(source, start_offset, end_offset, count_comments)
        };

        if count > max {
            let (line, column) = source.offset_to_line_col(start_offset);
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!("Block has too many lines. [{count}/{max}]"),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BlockLength, "cops/metrics/block_length");

    #[test]
    fn config_custom_max() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(3.into()))]),
            ..CopConfig::default()
        };
        // 4 body lines exceeds Max:3
        let source = b"items.each do |x|\n  a = 1\n  b = 2\n  c = 3\n  d = 4\nend\n";
        let diags = run_cop_full_with_config(&BlockLength, source, config);
        assert!(!diags.is_empty(), "Should fire with Max:3 on 4-line block");
        assert!(diags[0].message.contains("[4/3]"));
    }

    #[test]
    fn config_count_as_one_array() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(3.into())),
                ("CountAsOne".into(), serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("array".into()),
                ])),
            ]),
            ..CopConfig::default()
        };
        // Body: a, b, [\n1,\n2\n] = 2 + 1 folded = 3 lines
        let source = b"items.each do |x|\n  a = 1\n  b = 2\n  arr = [\n    1,\n    2\n  ]\nend\n";
        let diags = run_cop_full_with_config(&BlockLength, source, config);
        assert!(diags.is_empty(), "Should not fire when array is folded (3/3)");
    }
}
