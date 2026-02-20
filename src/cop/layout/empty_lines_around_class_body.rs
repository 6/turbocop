use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CLASS_NODE;

pub struct EmptyLinesAroundClassBody;

impl Cop for EmptyLinesAroundClassBody {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundClassBody"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_NODE]
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
        let style = config.get_str("EnforcedStyle", "no_empty_lines");
        let class_node = match node.as_class_node() {
            Some(c) => c,
            None => return,
        };

        let kw_offset = class_node.class_keyword_loc().start_offset();
        let end_offset = class_node.end_keyword_loc().start_offset();

        match style {
            "empty_lines" => {
                diagnostics.extend(util::check_missing_empty_lines_around_body(
                    self.name(), source, kw_offset, end_offset, "class",
                ));
            }
            "beginning_only" => {
                // Require blank line at beginning, flag blank at end
                let mut diags = util::check_missing_empty_lines_around_body(
                    self.name(), source, kw_offset, end_offset, "class",
                );
                // Keep only "beginning" diagnostics
                diags.retain(|d| d.message.contains("beginning"));
                // Also flag extra blank at end
                let extra_end = util::check_empty_lines_around_body(
                    self.name(), source, kw_offset, end_offset, "class",
                );
                diags.extend(extra_end.into_iter().filter(|d| d.message.contains("end")));
                diagnostics.extend(diags);
            }
            "ending_only" => {
                // Require blank line at end, flag blank at beginning
                let mut diags = util::check_missing_empty_lines_around_body(
                    self.name(), source, kw_offset, end_offset, "class",
                );
                // Keep only "end" diagnostics
                diags.retain(|d| d.message.contains("end"));
                // Also flag extra blank at beginning
                let extra_begin = util::check_empty_lines_around_body(
                    self.name(), source, kw_offset, end_offset, "class",
                );
                diags.extend(extra_begin.into_iter().filter(|d| d.message.contains("beginning")));
                diagnostics.extend(diags);
            }
            _ => {
                // "no_empty_lines" (default)
                diagnostics.extend(util::check_empty_lines_around_body(
                    self.name(), source, kw_offset, end_offset, "class",
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        EmptyLinesAroundClassBody,
        "cops/layout/empty_lines_around_class_body"
    );

    #[test]
    fn single_line_class_no_offense() {
        let src = b"class Foo; end\n";
        let diags = run_cop_full(&EmptyLinesAroundClassBody, src);
        assert!(diags.is_empty(), "Single-line class should not trigger");
    }

    #[test]
    fn blank_line_at_both_ends() {
        let src = b"class Foo\n\n  def bar; end\n\nend\n";
        let diags = run_cop_full(&EmptyLinesAroundClassBody, src);
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
        let src = b"class Foo\n  def bar; end\nend\n";
        let diags = run_cop_full_with_config(&EmptyLinesAroundClassBody, src, config);
        assert_eq!(diags.len(), 2, "empty_lines style should require blank lines at both ends");
    }

    #[test]
    fn beginning_only_style() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("beginning_only".into())),
            ]),
            ..CopConfig::default()
        };
        // No blank at beginning => flag missing beginning blank
        let src = b"class Foo\n  def bar; end\nend\n";
        let diags = run_cop_full_with_config(&EmptyLinesAroundClassBody, src, config);
        assert!(diags.iter().any(|d| d.message.contains("beginning")),
            "beginning_only should flag missing blank at beginning");
    }
}
