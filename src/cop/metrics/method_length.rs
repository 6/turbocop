use crate::cop::util::count_body_lines;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct MethodLength;

impl Cop for MethodLength {
    fn name(&self) -> &'static str {
        "Metrics/MethodLength"
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

        let max = config
            .options
            .get("Max")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        let count_comments = config
            .options
            .get("CountComments")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let start_offset = def_node.def_keyword_loc().start_offset();
        let end_offset = end_loc.start_offset();
        let count = count_body_lines(source, start_offset, end_offset, count_comments);

        if count > max {
            let (line, column) = source.offset_to_line_col(start_offset);
            return vec![Diagnostic {
                path: source.path_str().to_string(),
                location: Location { line, column },
                severity: Severity::Convention,
                cop_name: self.name().to_string(),
                message: format!("Method has too many lines. [{count}/{max}]"),
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
            &MethodLength,
            include_bytes!("../../../testdata/cops/metrics/method_length/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &MethodLength,
            include_bytes!("../../../testdata/cops/metrics/method_length/no_offense.rb"),
        );
    }

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
