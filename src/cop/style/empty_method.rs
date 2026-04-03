use crate::cop::shared::node_type::{DEF_NODE, STATEMENTS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-11)
///
/// Corpus oracle reported FP=1, FN=0.
///
/// FP=1: empty methods with an inline comment on the `end` line were still
/// being flagged. RuboCop treats any comment within the method's line span as
/// making the method non-empty, so the fix checks parsed comments across the
/// full `def..end` range instead of only looking at the `def` line and comment-
/// only body lines.
///
/// FN=0: no missed detections were reported by the corpus oracle for this run.
pub struct EmptyMethod;

fn method_has_comment(
    source: &SourceFile,
    parse_result: &ruby_prism::ParseResult<'_>,
    def_line: usize,
    end_line: usize,
) -> bool {
    parse_result.comments().any(|comment| {
        let (comment_line, _) = source.offset_to_line_col(comment.location().start_offset());
        (def_line..=end_line).contains(&comment_line)
    })
}

impl Cop for EmptyMethod {
    fn name(&self) -> &'static str {
        "Style/EmptyMethod"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "compact");
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        // Skip endless methods (no end keyword)
        let end_kw_loc = match def_node.end_keyword_loc() {
            Some(loc) => loc,
            None => return,
        };

        let def_loc = def_node.def_keyword_loc();
        let (def_line, _) = source.offset_to_line_col(def_loc.start_offset());
        let (end_line, _) = source.offset_to_line_col(end_kw_loc.start_offset());
        let is_single_line = def_line == end_line;

        let is_empty = match def_node.body() {
            None => true,
            Some(body) => {
                if let Some(stmts) = body.as_statements_node() {
                    stmts.body().is_empty()
                } else {
                    false
                }
            }
        };

        if !is_empty {
            return;
        }

        if method_has_comment(source, parse_result, def_line, end_line) {
            return;
        }

        match enforced_style {
            "compact" if !is_single_line => {
                let (line, column) = source.offset_to_line_col(def_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Put empty method definitions on a single line.".to_string(),
                ));
            }
            "expanded" if is_single_line => {
                let (line, column) = source.offset_to_line_col(def_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Put the `end` on the next line.".to_string(),
                ));
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{run_cop_full, run_cop_full_with_config};

    crate::cop_fixture_tests!(EmptyMethod, "cops/style/empty_method");

    #[test]
    fn expanded_style_flags_single_line() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("expanded".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"def foo; end\n";
        let diags = run_cop_full_with_config(&EmptyMethod, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("next line"));
    }

    #[test]
    fn expanded_style_allows_multiline() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("expanded".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"def foo\nend\n";
        let diags = run_cop_full_with_config(&EmptyMethod, source, config);
        assert!(
            diags.is_empty(),
            "Should allow multiline empty method with expanded style"
        );
    }

    #[test]
    fn compact_style_allows_single_line() {
        // Default compact style should not flag single-line empty methods
        let source = b"def foo; end\n";
        let diags = run_cop_full(&EmptyMethod, source);
        assert!(diags.is_empty());
    }
}
