use crate::cop::util::{count_body_lines, count_body_lines_ex, collect_foldable_ranges};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ModuleLength;

impl Cop for ModuleLength {
    fn name(&self) -> &'static str {
        "Metrics/ModuleLength"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let module_node = match node.as_module_node() {
            Some(m) => m,
            None => return Vec::new(),
        };

        let max = config.get_usize("Max", 100);
        let count_comments = config.get_bool("CountComments", false);
        let count_as_one = config.get_string_array("CountAsOne");

        let start_offset = module_node.module_keyword_loc().start_offset();
        let end_offset = module_node.end_keyword_loc().start_offset();
        let count = if let Some(cao) = &count_as_one {
            if !cao.is_empty() {
                if let Some(body) = module_node.body() {
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
                format!("Module has too many lines. [{count}/{max}]"),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ModuleLength, "cops/metrics/module_length");

    #[test]
    fn config_custom_max() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(3.into()))]),
            ..CopConfig::default()
        };
        // 4 body lines exceeds Max:3
        let source = b"module Foo\n  a = 1\n  b = 2\n  c = 3\n  d = 4\nend\n";
        let diags = run_cop_full_with_config(&ModuleLength, source, config);
        assert!(!diags.is_empty(), "Should fire with Max:3 on 4-line module");
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
        let source = b"module Foo\n  a = 1\n  b = 2\n  ARR = [\n    1,\n    2\n  ]\nend\n";
        let diags = run_cop_full_with_config(&ModuleLength, source, config);
        assert!(diags.is_empty(), "Should not fire when array is folded (3/3)");
    }
}
