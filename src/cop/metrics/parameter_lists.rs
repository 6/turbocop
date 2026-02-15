use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
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

        let max = config.get_usize("Max", 5);
        let count_keyword_args = config.get_bool("CountKeywordArgs", true);
        let max_optional = config.get_usize("MaxOptionalParameters", 3);

        let params = match def_node.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let mut diagnostics = Vec::new();

        // Check total parameter count
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
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!(
                    "Avoid parameter lists longer than {max} parameters. [{count}/{max}]"
                ),
            ));
        }

        // Check optional parameter count
        let optional_count = params.optionals().len();
        if optional_count > max_optional {
            let start_offset = def_node.def_keyword_loc().start_offset();
            let (line, column) = source.offset_to_line_col(start_offset);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!(
                    "Method has too many optional parameters. [{optional_count}/{max_optional}]"
                ),
            ));
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ParameterLists, "cops/metrics/parameter_lists");

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

    #[test]
    fn config_max_optional_parameters() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        // 3 optional params with MaxOptionalParameters:2 should fire
        let config = CopConfig {
            options: HashMap::from([
                ("MaxOptionalParameters".into(), serde_yml::Value::Number(2.into())),
            ]),
            ..CopConfig::default()
        };
        let source = b"def foo(a = 1, b = 2, c = 3)\nend\n";
        let diags = run_cop_full_with_config(&ParameterLists, source, config);
        assert!(
            diags.iter().any(|d| d.message.contains("too many optional parameters")),
            "Should fire for too many optional parameters"
        );
    }

    #[test]
    fn config_max_optional_parameters_ok() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        // 2 optional params with MaxOptionalParameters:3 should not fire
        let config = CopConfig {
            options: HashMap::from([
                ("MaxOptionalParameters".into(), serde_yml::Value::Number(3.into())),
            ]),
            ..CopConfig::default()
        };
        let source = b"def foo(a = 1, b = 2)\nend\n";
        let diags = run_cop_full_with_config(&ParameterLists, source, config);
        assert!(
            !diags.iter().any(|d| d.message.contains("optional parameters")),
            "Should not fire for optional parameters under limit"
        );
    }
}
