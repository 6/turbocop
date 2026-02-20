use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{DEF_NODE, RETURN_NODE, STATEMENTS_NODE};

pub struct RedundantReturn;

impl Cop for RedundantReturn {
    fn name(&self) -> &'static str {
        "Style/RedundantReturn"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE, RETURN_NODE, STATEMENTS_NODE]
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
        let allow_multiple = config.get_bool("AllowMultipleReturnValues", false);
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        let body = match def_node.body() {
            Some(b) => b,
            None => return,
        };

        let statements = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let last = match statements.body().last() {
            Some(n) => n,
            None => return,
        };

        if let Some(ret_node) = last.as_return_node() {
            // AllowMultipleReturnValues: skip `return x, y` when enabled
            if allow_multiple {
                let arg_count = ret_node.arguments().map_or(0, |a| a.arguments().len());
                if arg_count > 1 {
                    return;
                }
            }
            let loc = last.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(source, line, column, "Redundant `return` detected.".to_string()));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{run_cop_full_with_config, run_cop_full};

    crate::cop_fixture_tests!(RedundantReturn, "cops/style/redundant_return");

    #[test]
    fn allow_multiple_return_values() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("AllowMultipleReturnValues".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        // `return x, y` should be allowed when AllowMultipleReturnValues is true
        let source = b"def foo\n  return x, y\nend\n";
        let diags = run_cop_full_with_config(&RedundantReturn, source, config);
        assert!(diags.is_empty(), "Should allow multiple return values when configured");
    }

    #[test]
    fn disallow_multiple_return_values_by_default() {
        // `return x, y` should be flagged by default
        let source = b"def foo\n  return x, y\nend\n";
        let diags = run_cop_full(&RedundantReturn, source);
        assert_eq!(diags.len(), 1, "Should flag multiple return values by default");
    }

    #[test]
    fn allow_multiple_still_flags_single_return() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("AllowMultipleReturnValues".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        // `return x` should still be flagged even with AllowMultipleReturnValues
        let source = b"def foo\n  return x\nend\n";
        let diags = run_cop_full_with_config(&RedundantReturn, source, config);
        assert_eq!(diags.len(), 1, "Single return should still be flagged");
    }
}
