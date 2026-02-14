use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct StabbyLambdaParentheses;

impl Cop for StabbyLambdaParentheses {
    fn name(&self) -> &'static str {
        "Style/StabbyLambdaParentheses"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let lambda_node = match node.as_lambda_node() {
            Some(l) => l,
            None => return Vec::new(),
        };

        let enforced_style = config
            .options
            .get("EnforcedStyle")
            .and_then(|v| v.as_str())
            .unwrap_or("require_parentheses");

        // Only care if the lambda has parameters
        if lambda_node.parameters().is_none() {
            return Vec::new();
        }

        let operator_loc = lambda_node.operator_loc();
        let operator_end = operator_loc.end_offset();
        let opening_loc = lambda_node.opening_loc();
        let opening_start = opening_loc.start_offset();

        // Look at the source between `->` and the opening `{` or `do`
        // to see if there are parentheses
        let bytes = source.as_bytes();
        let search_end = opening_start.min(bytes.len());
        let between = if operator_end < search_end {
            &bytes[operator_end..search_end]
        } else {
            &[]
        };
        let has_paren = between.contains(&b'(');

        match enforced_style {
            "require_parentheses" => {
                if !has_paren {
                    let (line, column) = source.offset_to_line_col(operator_loc.start_offset());
                    return vec![Diagnostic {
                        path: source.path_str().to_string(),
                        location: Location { line, column },
                        severity: Severity::Convention,
                        cop_name: self.name().to_string(),
                        message: "Use parentheses for stabby lambda arguments.".to_string(),
                    }];
                }
            }
            "require_no_parentheses" => {
                if has_paren {
                    let (line, column) = source.offset_to_line_col(operator_loc.start_offset());
                    return vec![Diagnostic {
                        path: source.path_str().to_string(),
                        location: Location { line, column },
                        severity: Severity::Convention,
                        cop_name: self.name().to_string(),
                        message: "Do not use parentheses for stabby lambda arguments.".to_string(),
                    }];
                }
            }
            _ => {}
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{
        assert_cop_no_offenses_full, assert_cop_offenses_full, run_cop_full_with_config,
    };

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &StabbyLambdaParentheses,
            include_bytes!(
                "../../../testdata/cops/style/stabby_lambda_parentheses/offense.rb"
            ),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &StabbyLambdaParentheses,
            include_bytes!(
                "../../../testdata/cops/style/stabby_lambda_parentheses/no_offense.rb"
            ),
        );
    }

    #[test]
    fn config_require_no_parentheses() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("require_no_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"f = ->(x) { x }\n";
        let diags = run_cop_full_with_config(&StabbyLambdaParentheses, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("Do not use parentheses"));
    }
}
