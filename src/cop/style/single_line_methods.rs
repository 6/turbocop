use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{DEF_NODE, STATEMENTS_NODE};

pub struct SingleLineMethods;

impl Cop for SingleLineMethods {
    fn name(&self) -> &'static str {
        "Style/SingleLineMethods"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allow_empty = config.get_bool("AllowIfMethodIsEmpty", true);
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        // Skip endless methods (no end keyword)
        let end_kw_loc = match def_node.end_keyword_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        // Check if the method has a body
        let has_body = match def_node.body() {
            None => false,
            Some(body) => {
                if let Some(stmts) = body.as_statements_node() {
                    !stmts.body().is_empty()
                } else {
                    true
                }
            }
        };

        // AllowIfMethodIsEmpty: skip empty methods when enabled (default true)
        if !has_body && allow_empty {
            return Vec::new();
        }

        let def_loc = def_node.def_keyword_loc();
        let (def_line, _) = source.offset_to_line_col(def_loc.start_offset());
        let (end_line, _) = source.offset_to_line_col(end_kw_loc.start_offset());

        if def_line == end_line {
            let (line, column) = source.offset_to_line_col(def_loc.start_offset());
            return vec![self.diagnostic(source, line, column, "Avoid single-line method definitions.".to_string())];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{run_cop_full, run_cop_full_with_config};

    crate::cop_fixture_tests!(SingleLineMethods, "cops/style/single_line_methods");

    #[test]
    fn empty_single_line_method_is_ok() {
        let source = b"def foo; end\n";
        let diags = run_cop_full(&SingleLineMethods, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn endless_method_is_ok() {
        let source = b"def foo = 42\n";
        let diags = run_cop_full(&SingleLineMethods, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn disallow_empty_single_line_methods() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("AllowIfMethodIsEmpty".into(), serde_yml::Value::Bool(false)),
            ]),
            ..CopConfig::default()
        };
        // Empty single-line `def foo; end` should be flagged when AllowIfMethodIsEmpty is false
        let source = b"def foo; end\n";
        let diags = run_cop_full_with_config(&SingleLineMethods, source, config);
        assert_eq!(diags.len(), 1, "Should flag empty single-line method when AllowIfMethodIsEmpty is false");
    }
}
