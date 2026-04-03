use crate::cop::shared::node_type::MODULE_NODE;
use crate::cop::shared::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=465, FN=0.
///
/// Investigation outcome: representative FPs were explored from corpus and an
/// attempted fix narrowed "empty line" detection to strictly empty/CR-only
/// lines in `no_empty_lines` mode.
///
/// Attempted fix effect: reduced FPs, but introduced large FN regression
/// (`check-cop --rerun` reported Missing=290), which violates conformance.
///
/// Resolution in this patch: revert the attempted logic change and keep current
/// behavior unchanged. A correct future fix likely needs config-sensitive
/// handling (for example excludes/department interactions), not a blanket
/// whitespace-line rule.
pub struct EmptyLinesAroundModuleBody;

impl Cop for EmptyLinesAroundModuleBody {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundModuleBody"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[MODULE_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "no_empty_lines");
        let module_node = match node.as_module_node() {
            Some(m) => m,
            None => return,
        };

        let kw_offset = module_node.module_keyword_loc().start_offset();
        let end_offset = module_node.end_keyword_loc().start_offset();

        match style {
            "empty_lines" => {
                diagnostics.extend(
                    util::check_missing_empty_lines_around_body_with_corrections(
                        self.name(),
                        source,
                        kw_offset,
                        end_offset,
                        "module",
                        corrections,
                    ),
                );
            }
            _ => {
                // "no_empty_lines" (default)
                diagnostics.extend(util::check_empty_lines_around_body_with_corrections(
                    self.name(),
                    source,
                    kw_offset,
                    end_offset,
                    "module",
                    corrections,
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
        EmptyLinesAroundModuleBody,
        "cops/layout/empty_lines_around_module_body"
    );
    crate::cop_autocorrect_fixture_tests!(
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
        assert_eq!(
            diags.len(),
            2,
            "Should flag both beginning and end blank lines"
        );
    }

    #[test]
    fn empty_lines_style_requires_blank_lines() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("empty_lines".into()),
            )]),
            ..CopConfig::default()
        };
        let src = b"module Foo\n  def bar; end\nend\n";
        let diags = run_cop_full_with_config(&EmptyLinesAroundModuleBody, src, config);
        assert_eq!(
            diags.len(),
            2,
            "empty_lines style should require blank lines at both ends"
        );
    }

    #[test]
    fn no_empty_lines_style_flags_crlf_empty_lines() {
        let src = b"module Foo\r\n\r\n  X = 1\r\n\r\nend\r\n";
        let diags = run_cop_full(&EmptyLinesAroundModuleBody, src);
        assert_eq!(
            diags.len(),
            2,
            "CRLF empty lines should still be treated as empty"
        );
    }
}
