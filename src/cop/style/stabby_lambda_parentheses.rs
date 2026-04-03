use crate::cop::shared::node_type::LAMBDA_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/StabbyLambdaParentheses - checks parentheses around stabby lambda arguments.
///
/// ## FP fix (93 FP, 2 FN in corpus)
///
/// Root cause: nitrocop used `lambda_node.parameters().is_some()` to detect lambdas
/// with arguments. In Prism, `parameters()` returns a `Node` that can be:
/// - `BlockParametersNode` — explicit arguments like `->(x) {}` or `-> x {}`
/// - `NumberedParametersNode` — implicit `_1`, `_2` usage like `-> { _1 + _2 }`
/// - `ItParametersNode` — implicit `it` usage like `-> { it + 1 }` (Ruby 3.4+)
///
/// RuboCop's `arguments?` only returns true for explicit argument declarations.
/// Numbered/it parameters are implicit and should not trigger this cop.
///
/// Additionally, `->() {}` (empty explicit parens, no actual arguments) has
/// `arguments? = false` in RuboCop, so we must also check that the
/// `BlockParametersNode` contains actual parameters.
///
/// ## FN fix: method call parens in default values
///
/// The paren detection used `between.contains('(')` which matched method call
/// parentheses in default values like `-> a=a() { }`. Fixed to check if the
/// first non-whitespace character after `->` is `(`, which correctly
/// distinguishes parameter parens from default value method calls.
pub struct StabbyLambdaParentheses;

impl Cop for StabbyLambdaParentheses {
    fn name(&self) -> &'static str {
        "Style/StabbyLambdaParentheses"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[LAMBDA_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let lambda_node = match node.as_lambda_node() {
            Some(l) => l,
            None => return,
        };

        let enforced_style = config.get_str("EnforcedStyle", "require_parentheses");

        // Only care if the lambda has explicit arguments (BlockParametersNode).
        // Skip implicit parameters (NumberedParametersNode for _1/_2,
        // ItParametersNode for `it`) and lambdas with no parameters at all.
        let params = match lambda_node.parameters() {
            Some(p) => p,
            None => return,
        };
        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return, // NumberedParametersNode or ItParametersNode — skip
        };

        // Also skip empty parameter lists like ->() {} — RuboCop's arguments?
        // returns false when there are no actual arguments, just empty parens.
        let has_actual_params = if let Some(inner) = block_params.parameters() {
            !inner.requireds().is_empty()
                || !inner.optionals().is_empty()
                || inner.rest().is_some()
                || !inner.posts().is_empty()
                || !inner.keywords().is_empty()
                || inner.keyword_rest().is_some()
                || inner.block().is_some()
        } else {
            false
        };
        if !has_actual_params {
            return;
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
        // Check if the first non-whitespace character after `->` is `(`.
        // This distinguishes parameter parentheses `->(x)` from method call
        // parentheses in default values `-> a=a() { }`.
        let has_paren = between
            .iter()
            .find(|&&b| b != b' ' && b != b'\t' && b != b'\n' && b != b'\r')
            .is_some_and(|&b| b == b'(');

        match enforced_style {
            "require_parentheses" => {
                if !has_paren {
                    let (line, column) = source.offset_to_line_col(operator_loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Use parentheses for stabby lambda arguments.".to_string(),
                    ));
                }
            }
            "require_no_parentheses" => {
                if has_paren {
                    let (line, column) = source.offset_to_line_col(operator_loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Do not use parentheses for stabby lambda arguments.".to_string(),
                    ));
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full_with_config;

    crate::cop_fixture_tests!(
        StabbyLambdaParentheses,
        "cops/style/stabby_lambda_parentheses"
    );

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

    #[test]
    fn no_offense_numbered_params() {
        let source = b"f = -> { _1 + _2 }\n";
        let diags = crate::testutil::run_cop_full(&StabbyLambdaParentheses, source);
        assert_eq!(diags.len(), 0, "numbered params should not trigger offense");
    }

    #[test]
    fn no_offense_it_param() {
        let source = b"f = -> { it + 1 }\n";
        let diags = crate::testutil::run_cop_full(&StabbyLambdaParentheses, source);
        assert_eq!(diags.len(), 0, "it param should not trigger offense");
    }

    #[test]
    fn no_offense_empty_parens() {
        let source = b"f = ->() { true }\n";
        let diags = crate::testutil::run_cop_full(&StabbyLambdaParentheses, source);
        assert_eq!(diags.len(), 0, "empty parens should not trigger offense");
    }

    #[test]
    fn no_offense_numbered_params_require_no_parens() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("require_no_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"f = -> { _1 + _2 }\n";
        let diags = run_cop_full_with_config(&StabbyLambdaParentheses, source, config);
        assert_eq!(
            diags.len(),
            0,
            "numbered params should not trigger offense under require_no_parentheses"
        );
    }
}
