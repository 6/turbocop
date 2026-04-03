use crate::cop::shared::node_type::{CASE_MATCH_NODE, CASE_NODE, WHEN_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Investigation findings:
/// - **FN root cause (109 FNs):** Cop only handled `CaseNode` (case/when) but not
///   `CaseMatchNode` (case/in pattern matching, Ruby 3.0+). Prism uses separate node
///   types: `CaseNode` for `case/when` and `CaseMatchNode` for `case/in`.
/// - **Fix:** Added `CASE_MATCH_NODE` to `interested_node_types` and handle `InNode`
///   conditions with `.in_loc()` for the `in` keyword location, using `in` instead of
///   `when` in diagnostic messages.
/// - **FP (4):** Root cause: missing `end_and_last_conditional_same_line?` guard from
///   RuboCop. When using `end` style, RuboCop skips the check if the `end` keyword is on
///   the same line as the last conditional (`else` or last `when`/`in`). Without this
///   guard, nitrocop would flag `when`/`in` in compact trailing forms as misindented.
pub struct CaseIndentation;

impl Cop for CaseIndentation {
    fn name(&self) -> &'static str {
        "Layout/CaseIndentation"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CASE_NODE, CASE_MATCH_NODE, WHEN_NODE]
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
        let style = config.get_str("EnforcedStyle", "case");
        let indent_one_step = config.get_bool("IndentOneStep", false);
        let indent_width = config.get_usize("IndentationWidth", 2);

        // Handle both CaseNode (case/when) and CaseMatchNode (case/in pattern matching)
        if let Some(case_node) = node.as_case_node() {
            let case_loc = case_node.case_keyword_loc();
            let (case_line, case_col) = source.offset_to_line_col(case_loc.start_offset());

            // Skip single-line case expressions (RuboCop skips these)
            let (end_line, _) =
                source.offset_to_line_col(case_node.end_keyword_loc().start_offset());
            if case_line == end_line {
                return;
            }

            // Skip when "end" style and the end keyword is on the same line as
            // the last conditional (else or last when). RuboCop skips these to
            // avoid false positives on compact trailing forms.
            if style == "end" {
                let last_cond_line = if let Some(else_clause) = case_node.else_clause() {
                    Some(
                        source
                            .offset_to_line_col(else_clause.else_keyword_loc().start_offset())
                            .0,
                    )
                } else {
                    // Use the last when's keyword line
                    case_node.conditions().iter().last().and_then(|c| {
                        c.as_when_node()
                            .map(|w| source.offset_to_line_col(w.keyword_loc().start_offset()).0)
                    })
                };
                if last_cond_line == Some(end_line) {
                    return;
                }
            }

            let base_col = if style == "end" {
                source
                    .offset_to_line_col(case_node.end_keyword_loc().start_offset())
                    .1
            } else {
                case_col
            };

            let expected_col = if indent_one_step {
                base_col + indent_width
            } else {
                base_col
            };

            let message = if indent_one_step {
                "Indent `when` one step more than `case`.".to_string()
            } else if style == "end" {
                "Indent `when` as deep as `end`.".to_string()
            } else {
                "Indent `when` as deep as `case`.".to_string()
            };

            for condition in case_node.conditions().iter() {
                if let Some(when_node) = condition.as_when_node() {
                    let when_loc = when_node.keyword_loc();
                    let (when_line, when_col) = source.offset_to_line_col(when_loc.start_offset());

                    if when_col != expected_col {
                        diagnostics.push(self.diagnostic(
                            source,
                            when_line,
                            when_col,
                            message.clone(),
                        ));
                    }
                }
            }
        } else if let Some(case_match_node) = node.as_case_match_node() {
            let case_loc = case_match_node.case_keyword_loc();
            let (case_line, case_col) = source.offset_to_line_col(case_loc.start_offset());

            // Skip single-line case expressions (RuboCop skips these)
            let (end_line, _) =
                source.offset_to_line_col(case_match_node.end_keyword_loc().start_offset());
            if case_line == end_line {
                return;
            }

            // Skip when "end" style and the end keyword is on the same line as
            // the last conditional (else or last in). RuboCop skips these to
            // avoid false positives on compact trailing forms.
            if style == "end" {
                let last_cond_line = if let Some(else_clause) = case_match_node.else_clause() {
                    Some(
                        source
                            .offset_to_line_col(else_clause.else_keyword_loc().start_offset())
                            .0,
                    )
                } else {
                    // Use the last in's keyword line
                    case_match_node.conditions().iter().last().and_then(|c| {
                        c.as_in_node()
                            .map(|i| source.offset_to_line_col(i.in_loc().start_offset()).0)
                    })
                };
                if last_cond_line == Some(end_line) {
                    return;
                }
            }

            let base_col = if style == "end" {
                source
                    .offset_to_line_col(case_match_node.end_keyword_loc().start_offset())
                    .1
            } else {
                case_col
            };

            let expected_col = if indent_one_step {
                base_col + indent_width
            } else {
                base_col
            };

            let message = if indent_one_step {
                "Indent `in` one step more than `case`.".to_string()
            } else if style == "end" {
                "Indent `in` as deep as `end`.".to_string()
            } else {
                "Indent `in` as deep as `case`.".to_string()
            };

            for condition in case_match_node.conditions().iter() {
                if let Some(in_node) = condition.as_in_node() {
                    let in_loc = in_node.in_loc();
                    let (in_line, in_col) = source.offset_to_line_col(in_loc.start_offset());

                    if in_col != expected_col {
                        diagnostics.push(self.diagnostic(source, in_line, in_col, message.clone()));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(CaseIndentation, "cops/layout/case_indentation");

    #[test]
    fn nested_case_respects_own_indent() {
        let src = b"case x\nwhen 1\n  case y\n  when :a\n    puts :a\n  end\nend\n";
        let diags = run_cop_full(&CaseIndentation, src);
        assert!(
            diags.is_empty(),
            "Properly indented nested case should not trigger"
        );
    }

    #[test]
    fn multiple_when_some_misaligned() {
        let src = b"case x\nwhen 1\n  puts 1\n  when 2\n  puts 2\nend\n";
        let diags = run_cop_full(&CaseIndentation, src);
        assert_eq!(diags.len(), 1, "Only the misaligned when should trigger");
        assert_eq!(diags[0].location.line, 4);
        assert_eq!(diags[0].location.column, 2);
    }

    #[test]
    fn indent_one_step_requires_indented_when() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("IndentOneStep".into(), serde_yml::Value::Bool(true))]),
            ..CopConfig::default()
        };
        // `when` at same level as `case` should be flagged
        let src = b"case x\nwhen 1\n  puts 1\nend\n";
        let diags = run_cop_full_with_config(&CaseIndentation, src, config.clone());
        assert_eq!(
            diags.len(),
            1,
            "IndentOneStep should flag when at case level"
        );

        // `when` indented one step from `case` should be OK
        let src2 = b"case x\n  when 1\n    puts 1\nend\n";
        let diags2 = run_cop_full_with_config(&CaseIndentation, src2, config);
        assert!(
            diags2.is_empty(),
            "IndentOneStep should accept indented when"
        );
    }

    #[test]
    fn end_style_skips_when_end_on_same_line_as_else() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("end".into()),
            )]),
            ..CopConfig::default()
        };

        // end on same line as else — RuboCop skips the check entirely
        let src = b"x = case foo\n    when 1 then :a\n    when 2 then :b\n    else :c end\n";
        let diags = run_cop_full_with_config(&CaseIndentation, src, config.clone());
        assert!(
            diags.is_empty(),
            "end style should skip when end is on same line as else"
        );

        // end on same line as last when (no else) — RuboCop skips
        let src2 = b"x = case foo\n    when 1 then :a\n    when 2 then :b end\n";
        let diags2 = run_cop_full_with_config(&CaseIndentation, src2, config);
        assert!(
            diags2.is_empty(),
            "end style should skip when end is on same line as last when"
        );
    }

    #[test]
    fn end_style_aligns_when_with_end() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("end".into()),
            )]),
            ..CopConfig::default()
        };
        // `case` at col 4 (via assignment), `end` at col 0, `when` at col 4 (aligned with case)
        // "end" style expects `when` at col 0 (aligned with end), so should flag
        let src = b"x = case foo\n    when 1\n      :a\n    end\n";
        let diags = run_cop_full_with_config(&CaseIndentation, src, config.clone());
        // "end" is at col 4, "when" is at col 4 — should be OK since both match
        // The interesting case is where end is at a different column:
        // Actually in this case end_keyword_loc gives us col 4, same as when.
        // The "end" style only differs from "case" when case_col != end_col.
        // Let's just verify "end" style accepts when aligned at end_col
        assert!(
            diags.is_empty(),
            "end style should accept when aligned with end"
        );

        // Verify "case" style still works — when at case_col should be OK
        let config_case = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("case".into()),
            )]),
            ..CopConfig::default()
        };
        let src2 = b"case x\nwhen 1\n  puts 1\nend\n";
        let diags2 = run_cop_full_with_config(&CaseIndentation, src2, config_case);
        assert!(
            diags2.is_empty(),
            "case style should accept when aligned with case"
        );
    }
}
