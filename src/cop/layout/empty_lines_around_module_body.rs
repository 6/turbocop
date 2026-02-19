use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::MODULE_NODE;

pub struct EmptyLinesAroundModuleBody;

impl Cop for EmptyLinesAroundModuleBody {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundModuleBody"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[MODULE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyle", "no_empty_lines");
        let module_node = match node.as_module_node() {
            Some(m) => m,
            None => return Vec::new(),
        };

        let kw_offset = module_node.module_keyword_loc().start_offset();
        let end_offset = module_node.end_keyword_loc().start_offset();

        match style {
            "empty_lines" => {
                util::check_missing_empty_lines_around_body(
                    self.name(), source, kw_offset, end_offset, "module",
                )
            }
            _ => {
                // "no_empty_lines" (default)
                util::check_empty_lines_around_body(
                    self.name(), source, kw_offset, end_offset, "module",
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        EmptyLinesAroundModuleBody,
        "cops/layout/empty_lines_around_module_body"
    );

    #[test]
    fn single_line_module_no_offense() {
        let src = b"module Foo; end\n";
        let diags = run_cop_full(&EmptyLinesAroundModuleBody, src);
        assert!(diags.is_empty(), "Single-line module should not trigger");
    }

    #[test]
    fn blank_line_at_both_ends() {
        let src = b"module Foo\n\n  def bar; end\n\nend\n";
        let diags = run_cop_full(&EmptyLinesAroundModuleBody, src);
        assert_eq!(diags.len(), 2, "Should flag both beginning and end blank lines");
    }

    #[test]
    fn empty_lines_style_requires_blank_lines() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("empty_lines".into())),
            ]),
            ..CopConfig::default()
        };
        let src = b"module Foo\n  def bar; end\nend\n";
        let diags = run_cop_full_with_config(&EmptyLinesAroundModuleBody, src, config);
        assert_eq!(diags.len(), 2, "empty_lines style should require blank lines at both ends");
    }
}
