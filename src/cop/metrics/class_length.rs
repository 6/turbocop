use crate::cop::util::{count_body_lines_full, collect_foldable_ranges};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CLASS_NODE, MODULE_NODE, STATEMENTS_NODE};

pub struct ClassLength;

/// Collect line ranges of inner class/module definitions within a body node.
/// Returns (start_line, end_line) pairs (1-indexed) for each inner class/module.
fn inner_classlike_ranges(source: &SourceFile, body: &ruby_prism::Node<'_>) -> Vec<(usize, usize)> {
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return Vec::new(),
    };
    let mut ranges = Vec::new();
    for node in stmts.body().iter() {
        if let Some(cls) = node.as_class_node() {
            let loc = cls.location();
            let (start, _) = source.offset_to_line_col(loc.start_offset());
            let end_off = loc.end_offset().saturating_sub(1).max(loc.start_offset());
            let (end, _) = source.offset_to_line_col(end_off);
            ranges.push((start, end));
        } else if let Some(m) = node.as_module_node() {
            let loc = m.location();
            let (start, _) = source.offset_to_line_col(loc.start_offset());
            let end_off = loc.end_offset().saturating_sub(1).max(loc.start_offset());
            let (end, _) = source.offset_to_line_col(end_off);
            ranges.push((start, end));
        }
    }
    ranges
}

impl Cop for ClassLength {
    fn name(&self) -> &'static str {
        "Metrics/ClassLength"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_NODE, MODULE_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let class_node = match node.as_class_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let max = config.get_usize("Max", 100);
        let count_comments = config.get_bool("CountComments", false);
        let count_as_one = config.get_string_array("CountAsOne");

        let start_offset = class_node.class_keyword_loc().start_offset();
        let end_offset = class_node.end_keyword_loc().start_offset();

        // Collect foldable ranges from CountAsOne config
        let mut foldable_ranges = Vec::new();
        if let Some(cao) = &count_as_one {
            if !cao.is_empty() {
                if let Some(body) = class_node.body() {
                    foldable_ranges.extend(collect_foldable_ranges(source, &body, cao));
                }
            }
        }

        // Collect inner class/module line ranges to fully exclude from the count
        let mut inner_ranges = Vec::new();
        if let Some(body) = class_node.body() {
            inner_ranges = inner_classlike_ranges(source, &body);
        }

        let count = count_body_lines_full(
            source, start_offset, end_offset, count_comments,
            &foldable_ranges, &inner_ranges,
        );

        if count > max {
            let (line, column) = source.offset_to_line_col(start_offset);
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!("Class has too many lines. [{count}/{max}]"),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ClassLength, "cops/metrics/class_length");

    #[test]
    fn config_custom_max() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(3.into()))]),
            ..CopConfig::default()
        };
        // 4 body lines exceeds Max:3
        let source = b"class Foo\n  a = 1\n  b = 2\n  c = 3\n  d = 4\nend\n";
        let diags = run_cop_full_with_config(&ClassLength, source, config);
        assert!(!diags.is_empty(), "Should fire with Max:3 on 4-line class");
        assert!(diags[0].message.contains("[4/3]"));
    }

    #[test]
    fn config_count_as_one_hash() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        // With CountAsOne: ["hash"], a multiline hash counts as 1 line
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(3.into())),
                ("CountAsOne".into(), serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("hash".into()),
                ])),
            ]),
            ..CopConfig::default()
        };
        // Body: a, b, { k: v, \n k2: v2 \n } = 2 + 1 folded = 3 lines
        let source = b"class Foo\n  a = 1\n  b = 2\n  HASH = {\n    k: 1,\n    k2: 2\n  }\nend\n";
        let diags = run_cop_full_with_config(&ClassLength, source, config);
        assert!(diags.is_empty(), "Should not fire when hash is folded (3/3)");
    }
}
