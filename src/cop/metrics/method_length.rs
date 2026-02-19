use crate::cop::util::{count_body_lines_ex, collect_foldable_ranges, collect_heredoc_ranges};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::DEF_NODE;

pub struct MethodLength;

impl Cop for MethodLength {
    fn name(&self) -> &'static str {
        "Metrics/MethodLength"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        // Skip endless methods (no end keyword)
        let end_loc = match def_node.end_keyword_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let max = config.get_usize("Max", 10);
        let count_comments = config.get_bool("CountComments", false);
        let count_as_one = config.get_string_array("CountAsOne");

        // AllowedMethods / AllowedPatterns: skip methods matching these
        let allowed_methods = config.get_string_array("AllowedMethods");
        let allowed_patterns = config.get_string_array("AllowedPatterns");
        let method_name_str =
            std::str::from_utf8(def_node.name().as_slice()).unwrap_or("");
        if let Some(allowed) = &allowed_methods {
            if allowed.iter().any(|m| m == method_name_str) {
                return Vec::new();
            }
        }
        if let Some(patterns) = &allowed_patterns {
            if patterns.iter().any(|p| method_name_str.contains(p.as_str())) {
                return Vec::new();
            }
        }

        let start_offset = def_node.def_keyword_loc().start_offset();
        let end_offset = end_loc.start_offset();

        // Always fold heredoc lines to match RuboCop behavior. In RuboCop's
        // Parser AST, `body.source` for a heredoc returns only the opening
        // delimiter, so heredoc content is never counted toward method length.
        // Prism includes heredoc content in the node's byte range, so we must
        // explicitly fold those lines.
        let mut all_foldable: Vec<(usize, usize)> = if let Some(body) = def_node.body() {
            let mut ranges = collect_heredoc_ranges(source, &body);
            if let Some(cao) = &count_as_one {
                if !cao.is_empty() {
                    ranges.extend(collect_foldable_ranges(source, &body, cao));
                }
            }
            ranges
        } else {
            Vec::new()
        };
        // Deduplicate: heredoc ranges may already be in foldable ranges if
        // CountAsOne includes "heredoc"
        all_foldable.sort();
        all_foldable.dedup();

        let count = count_body_lines_ex(source, start_offset, end_offset, count_comments, &all_foldable);

        if count > max {
            let (line, column) = source.offset_to_line_col(start_offset);
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!("Method has too many lines. [{count}/{max}]"),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MethodLength, "cops/metrics/method_length");

    #[test]
    fn config_custom_max() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(5.into()))]),
            ..CopConfig::default()
        };
        // 6 body lines exceeds Max:5
        let source = b"def foo\n  a\n  b\n  c\n  d\n  e\n  f\nend\n";
        let diags = run_cop_full_with_config(&MethodLength, source, config);
        assert!(!diags.is_empty(), "Should fire with Max:5 on 6-line method");
        assert!(diags[0].message.contains("[6/5]"));
    }

    #[test]
    fn config_count_as_one_array() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        // With CountAsOne: ["array"], a multiline array counts as 1 line
        // Use Max:4 so it passes with folding but would fail without
        let config2 = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(4.into())),
                ("CountAsOne".into(), serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("array".into()),
                ])),
            ]),
            ..CopConfig::default()
        };
        // Body: a, b, c, arr = [\n1,\n2,\n3\n] = 3 + 4 = 7 lines without folding, 3 + 1 = 4 with folding
        let source = b"def foo\n  a = 1\n  b = 2\n  c = 3\n  arr = [\n    1,\n    2,\n    3\n  ]\nend\n";
        let diags = run_cop_full_with_config(&MethodLength, source, config2);
        assert!(diags.is_empty(), "Should not fire when array is folded to 1 line (4/4)");

        // Without CountAsOne, Max:4 should fire (7 lines > 4)
        let config3 = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(4.into())),
            ]),
            ..CopConfig::default()
        };
        let diags2 = run_cop_full_with_config(&MethodLength, source, config3);
        assert!(!diags2.is_empty(), "Should fire without CountAsOne (7 lines > 4)");
    }

    #[test]
    fn config_count_comments_true() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(3.into())),
                ("CountComments".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        // 4 lines including comments exceeds Max:3 when CountComments:true
        let source = b"def foo\n  # comment1\n  # comment2\n  a\n  b\nend\n";
        let diags = run_cop_full_with_config(&MethodLength, source, config);
        assert!(!diags.is_empty(), "Should fire with CountComments:true");
    }
}
