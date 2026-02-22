use crate::cop::node_type::{ELSE_NODE, IF_NODE};
use crate::cop::util::assignment_context_base_col;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ElseAlignment;

impl Cop for ElseAlignment {
    fn name(&self) -> &'static str {
        "Layout/ElseAlignment"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ELSE_NODE, IF_NODE]
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
        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return,
        };

        // Must be a keyword if (not ternary)
        let if_kw_loc = match if_node.if_keyword_loc() {
            Some(loc) => loc,
            None => return,
        };

        // Only check top-level `if`, not `elsif` (which is also an IfNode)
        // An elsif has its keyword as "elsif", not "if"
        if if_kw_loc.as_slice() != b"if" && if_kw_loc.as_slice() != b"unless" {
            return;
        }

        let (_, if_col) = source.offset_to_line_col(if_kw_loc.start_offset());

        // Determine expected alignment column for else/elsif.
        // When `if` is the RHS of an assignment (e.g., `x = if cond`) and
        // Layout/EndAlignment.EnforcedStyleAlignWith is "variable", else/elsif
        // align with the assignment variable (start of line), not `if`.
        let end_style = config.get_str("EndAlignmentStyle", "keyword");
        let expected_col = if end_style == "variable" {
            if let Some(var_col) = assignment_context_base_col(source, if_kw_loc.start_offset()) {
                var_col
            } else {
                if_col
            }
        } else {
            if_col
        };

        let mut current = if_node.subsequent();

        while let Some(subsequent) = current {
            if let Some(else_node) = subsequent.as_else_node() {
                let else_kw_loc = else_node.else_keyword_loc();
                let (else_line, else_col) = source.offset_to_line_col(else_kw_loc.start_offset());
                if else_col != expected_col {
                    diagnostics.push(self.diagnostic(
                        source,
                        else_line,
                        else_col,
                        "Align `else` with `if`.".to_string(),
                    ));
                }
                current = None;
            } else if let Some(elsif_node) = subsequent.as_if_node() {
                let elsif_kw_loc = match elsif_node.if_keyword_loc() {
                    Some(loc) => loc,
                    None => break,
                };
                let (elsif_line, elsif_col) =
                    source.offset_to_line_col(elsif_kw_loc.start_offset());
                if elsif_col != expected_col {
                    diagnostics.push(self.diagnostic(
                        source,
                        elsif_line,
                        elsif_col,
                        "Align `elsif` with `if`.".to_string(),
                    ));
                }
                current = elsif_node.subsequent();
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(ElseAlignment, "cops/layout/else_alignment");

    #[test]
    fn ternary_no_offense() {
        let source = b"x = true ? 1 : 2\n";
        let diags = run_cop_full(&ElseAlignment, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn assignment_context_else_misaligned() {
        // `else` at column 0, `if` at column 4 — should be flagged
        let source = b"x = if foo\n  bar\nelse\n  baz\nend\n";
        let diags = run_cop_full(&ElseAlignment, source);
        assert_eq!(
            diags.len(),
            1,
            "else at col 0 should be flagged when if is at col 4"
        );
    }

    #[test]
    fn assignment_context_keyword_style_no_offense() {
        // Keyword style: `else` at col 4 (with `if`), body/else aligned with `if`
        let source = b"x = if foo\n      bar\n    else\n      baz\n    end\n";
        let diags = run_cop_full(&ElseAlignment, source);
        assert!(
            diags.is_empty(),
            "keyword style should not flag else aligned with if: {:?}",
            diags
        );
    }

    #[test]
    fn assignment_variable_style_else_aligned_with_variable() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EndAlignmentStyle".into(),
                serde_yml::Value::String("variable".into()),
            )]),
            ..CopConfig::default()
        };
        // Variable style: else at col 4 (aligned with `server`), not col 15 (with `if`)
        let source = b"    server = if cond\n      body\n    else\n      other\n    end\n";
        let diags = run_cop_full_with_config(&ElseAlignment, source, config);
        assert!(
            diags.is_empty(),
            "variable style should not flag else aligned with variable: {:?}",
            diags
        );
    }

    #[test]
    fn assignment_variable_style_elsif_aligned_with_variable() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EndAlignmentStyle".into(),
                serde_yml::Value::String("variable".into()),
            )]),
            ..CopConfig::default()
        };
        // Variable style: elsif at col 0 (aligned with `x`), not col 4 (with `if`)
        let source = b"x = if foo\n  bar\nelsif baz\n  qux\nelse\n  quux\nend\n";
        let diags = run_cop_full_with_config(&ElseAlignment, source, config);
        assert!(
            diags.is_empty(),
            "variable style should not flag elsif/else aligned with variable: {:?}",
            diags
        );
    }

    #[test]
    fn assignment_variable_style_flags_wrong_column() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EndAlignmentStyle".into(),
                serde_yml::Value::String("variable".into()),
            )]),
            ..CopConfig::default()
        };
        // Variable style: else at col 2 doesn't align with variable (col 0) or if (col 4)
        let source = b"x = if foo\n  bar\n  else\n  baz\nend\n";
        let diags = run_cop_full_with_config(&ElseAlignment, source, config);
        assert_eq!(
            diags.len(),
            1,
            "should flag else not aligned with variable: {:?}",
            diags
        );
    }

    #[test]
    fn shovel_operator_variable_style_no_offense() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EndAlignmentStyle".into(),
                serde_yml::Value::String("variable".into()),
            )]),
            ..CopConfig::default()
        };
        // << operator context with variable style: else aligns with receiver
        let source = b"html << if error\n  error\nelse\n  default\nend\n";
        let diags = run_cop_full_with_config(&ElseAlignment, source, config);
        assert!(
            diags.is_empty(),
            "variable style << context should not flag else aligned with receiver: {:?}",
            diags
        );
    }

    #[test]
    fn shovel_operator_indented_variable_style_no_offense() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EndAlignmentStyle".into(),
                serde_yml::Value::String("variable".into()),
            )]),
            ..CopConfig::default()
        };
        // << operator context with variable style: else aligns with receiver at col 8
        let source = b"        @buffer << if value.safe?\n          value\n        else\n          escape(value)\n        end\n";
        let diags = run_cop_full_with_config(&ElseAlignment, source, config);
        assert!(
            diags.is_empty(),
            "variable style << context should not flag else aligned with @buffer: {:?}",
            diags
        );
    }
}
