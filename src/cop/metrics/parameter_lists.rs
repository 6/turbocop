use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct ParameterLists;

impl Cop for ParameterLists {
    fn name(&self) -> &'static str {
        "Metrics/ParameterLists"
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

        let max = config
            .options
            .get("Max")
            .and_then(|v| v.as_u64())
            .unwrap_or(5) as usize;

        let count_keyword_args = config
            .options
            .get("CountKeywordArgs")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let params = match def_node.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let mut count = 0usize;
        count += params.requireds().len();
        count += params.optionals().len();
        count += params.posts().len();

        if params.rest().is_some() {
            count += 1;
        }

        if count_keyword_args {
            count += params.keywords().len();
            if params.keyword_rest().is_some() {
                count += 1;
            }
        }

        if count > max {
            let start_offset = def_node.def_keyword_loc().start_offset();
            let (line, column) = source.offset_to_line_col(start_offset);
            return vec![Diagnostic {
                path: source.path_str().to_string(),
                location: Location { line, column },
                severity: Severity::Convention,
                cop_name: self.name().to_string(),
                message: format!(
                    "Avoid parameter lists longer than {max} parameters. [{count}/{max}]"
                ),
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
            &ParameterLists,
            include_bytes!("../../../testdata/cops/metrics/parameter_lists/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &ParameterLists,
            include_bytes!("../../../testdata/cops/metrics/parameter_lists/no_offense.rb"),
        );
    }

    #[test]
    fn config_custom_max() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(2.into()))]),
            ..CopConfig::default()
        };
        // 3 params exceeds Max:2
        let source = b"def foo(a, b, c)\nend\n";
        let diags = run_cop_full_with_config(&ParameterLists, source, config);
        assert!(!diags.is_empty(), "Should fire with Max:2 on 3-param method");
        assert!(diags[0].message.contains("[3/2]"));
    }
}
