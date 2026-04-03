use crate::cop::shared::node_type::{
    BEGIN_NODE, CASE_MATCH_NODE, CASE_NODE, DEF_NODE, ELSE_NODE, IF_NODE, UNLESS_NODE,
};
use crate::cop::shared::util::assignment_context_base_col;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Layout/ElseAlignment — checks that `else`/`elsif` aligns with the
/// corresponding keyword (`if`/`unless`/`case`/`begin`/`def`).
///
/// **Investigation (2026-03):** 110 FPs on single-line if/then/else/end
/// expressions (e.g., `if val then 'a' else 'b' end`).  RuboCop skips
/// alignment checks when the `else` is on the same line as the opening
/// keyword — alignment is inherently satisfied on a single line.  Fixed by
/// comparing the line numbers: if `else`/`elsif` shares a line with the
/// opening `if`/`unless`, skip the check.
///
/// **Investigation (2026-03, FN fix):** 44 FN caused by only handling
/// if/unless nodes. Added support for:
/// - case/when: else aligns with last `when` keyword
/// - case/in (pattern matching): else aligns with last `in` keyword
/// - begin/rescue/else: else aligns with `begin` keyword
/// - def/rescue/else: else aligns with `def` keyword
///
/// **Investigation (2026-03-17, FP=8, FN=6):**
/// FP=7: `else` on the same line as `when`/`in` keyword in case expressions
/// (e.g., `when 1 then 2 else 3`). The single-line skip only checked
/// `else_line == case_line`, not `else_line == last_when_line`. Fixed by
/// also checking if else is on the same line as the last when/in keyword.
/// FN=6: Prism has a separate `UnlessNode` that wasn't handled — only
/// `IfNode` was checked. `unless` keywords go through `UnlessNode`, not
/// `IfNode`. Fixed by adding `UNLESS_NODE` to interested types and
/// handling `as_unless_node()` with the same alignment logic.
///
/// **Investigation (2026-03-17, FP=1):**
/// FP=1: camping minified Ruby — `else` keyword mid-line after semicolons
/// (e.g., `...;s else raise "no template"`). RuboCop's `begins_its_line?`
/// skips alignment when `else` is not the first non-whitespace token on its
/// line. Fixed by adding an equivalent check.
pub struct ElseAlignment;

impl ElseAlignment {
    /// Returns true if the token at `offset` is the first non-whitespace on its line.
    /// Mirrors RuboCop's `begins_its_line?` — alignment checks are skipped when
    /// `else`/`elsif` does not begin its line (e.g., compressed/minified code).
    fn begins_its_line(source: &SourceFile, offset: usize) -> bool {
        let bytes = source.as_bytes();
        let mut pos = offset;
        while pos > 0 && bytes[pos - 1] != b'\n' {
            pos -= 1;
        }
        // pos is now the start of the line; scan forward for first non-whitespace
        let mut first_nonws = pos;
        while first_nonws < bytes.len()
            && (bytes[first_nonws] == b' ' || bytes[first_nonws] == b'\t')
        {
            first_nonws += 1;
        }
        first_nonws == offset
    }

    /// Check else alignment for begin/rescue/else constructs.
    /// `base_keyword` is the keyword name to use in the message (e.g., "begin", "def").
    fn check_begin_else(
        &self,
        source: &SourceFile,
        begin_node: &ruby_prism::BeginNode<'_>,
        base_col: usize,
        base_line: usize,
        base_keyword: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let else_clause = match begin_node.else_clause() {
            Some(ec) => ec,
            None => return,
        };
        let else_kw_loc = else_clause.else_keyword_loc();
        if !Self::begins_its_line(source, else_kw_loc.start_offset()) {
            return;
        }
        let (else_line, else_col) = source.offset_to_line_col(else_kw_loc.start_offset());
        // Skip single-line constructs
        if else_line == base_line {
            return;
        }
        if else_col != base_col {
            diagnostics.push(self.diagnostic(
                source,
                else_line,
                else_col,
                format!("Align `else` with `{base_keyword}`."),
            ));
        }
    }
}

impl Cop for ElseAlignment {
    fn name(&self) -> &'static str {
        "Layout/ElseAlignment"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ELSE_NODE,
            IF_NODE,
            UNLESS_NODE,
            CASE_NODE,
            CASE_MATCH_NODE,
            BEGIN_NODE,
            DEF_NODE,
        ]
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
        // --- case/when ---
        if let Some(case_node) = node.as_case_node() {
            let else_clause = match case_node.else_clause() {
                Some(ec) => ec,
                None => return,
            };
            // Align else with the last `when` keyword
            let last_when = case_node
                .conditions()
                .iter()
                .last()
                .and_then(|c| c.as_when_node());
            let (last_when_line, expected_col) = match last_when {
                Some(w) => {
                    let (line, col) = source.offset_to_line_col(w.keyword_loc().start_offset());
                    (line, col)
                }
                None => return,
            };
            let else_kw_loc = else_clause.else_keyword_loc();
            if !Self::begins_its_line(source, else_kw_loc.start_offset()) {
                return;
            }
            let (else_line, else_col) = source.offset_to_line_col(else_kw_loc.start_offset());
            // Skip single-line: else on same line as case OR same line as last when
            let case_line = source
                .offset_to_line_col(case_node.case_keyword_loc().start_offset())
                .0;
            if else_line == case_line || else_line == last_when_line {
                return;
            }
            if else_col != expected_col {
                diagnostics.push(self.diagnostic(
                    source,
                    else_line,
                    else_col,
                    "Align `else` with `when`.".to_string(),
                ));
            }
            return;
        }

        // --- case/in (pattern matching) ---
        if let Some(case_match_node) = node.as_case_match_node() {
            let else_clause = match case_match_node.else_clause() {
                Some(ec) => ec,
                None => return,
            };
            // Align else with the last `in` keyword
            let last_in = case_match_node
                .conditions()
                .iter()
                .last()
                .and_then(|c| c.as_in_node());
            let (last_in_line, expected_col) = match last_in {
                Some(i) => {
                    let (line, col) = source.offset_to_line_col(i.in_loc().start_offset());
                    (line, col)
                }
                None => return,
            };
            let else_kw_loc = else_clause.else_keyword_loc();
            if !Self::begins_its_line(source, else_kw_loc.start_offset()) {
                return;
            }
            let (else_line, else_col) = source.offset_to_line_col(else_kw_loc.start_offset());
            // Skip single-line: else on same line as case OR same line as last in
            let case_line = source
                .offset_to_line_col(case_match_node.case_keyword_loc().start_offset())
                .0;
            if else_line == case_line || else_line == last_in_line {
                return;
            }
            if else_col != expected_col {
                diagnostics.push(self.diagnostic(
                    source,
                    else_line,
                    else_col,
                    "Align `else` with `in`.".to_string(),
                ));
            }
            return;
        }

        // --- begin/rescue/else (explicit begin) ---
        if let Some(begin_node) = node.as_begin_node() {
            let begin_kw_loc = match begin_node.begin_keyword_loc() {
                Some(loc) => loc,
                // Implicit begin (e.g., def body) — handled by DefNode below
                None => return,
            };
            let (begin_line, begin_col) = source.offset_to_line_col(begin_kw_loc.start_offset());
            self.check_begin_else(
                source,
                &begin_node,
                begin_col,
                begin_line,
                "begin",
                diagnostics,
            );
            return;
        }

        // --- def/rescue/else ---
        if let Some(def_node) = node.as_def_node() {
            let body = match def_node.body() {
                Some(b) => b,
                None => return,
            };
            let begin_node = match body.as_begin_node() {
                Some(bn) => bn,
                None => return,
            };
            let def_kw_loc = def_node.def_keyword_loc();
            let (def_line, def_col) = source.offset_to_line_col(def_kw_loc.start_offset());

            // RuboCop checks for `private def ...` — if the def is preceded by
            // a method modifier on the same line, align with the modifier instead.
            // We approximate this by checking if there's a non-whitespace char
            // before `def` on the same line that isn't just indentation.
            let base_col = {
                let bytes = source.as_bytes();
                let mut line_start = def_kw_loc.start_offset();
                while line_start > 0 && bytes[line_start - 1] != b'\n' {
                    line_start -= 1;
                }
                // Find first non-whitespace on the line
                let mut first_nonws = line_start;
                while first_nonws < bytes.len()
                    && (bytes[first_nonws] == b' ' || bytes[first_nonws] == b'\t')
                {
                    first_nonws += 1;
                }
                let first_nonws_col = first_nonws - line_start;
                if first_nonws_col != def_col {
                    // Something like `private def foo` — use the line indent
                    // (which is the column of the modifier keyword)
                    first_nonws_col
                } else {
                    def_col
                }
            };

            self.check_begin_else(source, &begin_node, base_col, def_line, "def", diagnostics);
            return;
        }

        // --- unless ---
        // Prism uses a separate UnlessNode (not IfNode) for `unless` keywords.
        if let Some(unless_node) = node.as_unless_node() {
            let else_clause = match unless_node.else_clause() {
                Some(ec) => ec,
                None => return,
            };
            let unless_kw_loc = unless_node.keyword_loc();
            // Skip modifier unless (no end keyword)
            if unless_node.end_keyword_loc().is_none() {
                return;
            }
            let (unless_line, unless_col) = source.offset_to_line_col(unless_kw_loc.start_offset());

            let end_style = config.get_str("EndAlignmentStyle", "keyword");
            let expected_col = if end_style == "variable" {
                if let Some(var_col) =
                    assignment_context_base_col(source, unless_kw_loc.start_offset())
                {
                    var_col
                } else {
                    unless_col
                }
            } else {
                unless_col
            };

            let else_kw_loc = else_clause.else_keyword_loc();
            if !Self::begins_its_line(source, else_kw_loc.start_offset()) {
                return;
            }
            let (else_line, else_col) = source.offset_to_line_col(else_kw_loc.start_offset());
            // Single-line unless/else — skip
            if else_line == unless_line {
                return;
            }
            if else_col != expected_col {
                diagnostics.push(self.diagnostic(
                    source,
                    else_line,
                    else_col,
                    "Align `else` with `unless`.".to_string(),
                ));
            }
            return;
        }

        // --- if/unless (via IfNode — handles `if` keyword only) ---
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
        // Note: `unless` is handled by UnlessNode above, not here.
        if if_kw_loc.as_slice() != b"if" {
            return;
        }

        let (if_line, if_col) = source.offset_to_line_col(if_kw_loc.start_offset());

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
                if !Self::begins_its_line(source, else_kw_loc.start_offset()) {
                    current = None;
                    continue;
                }
                let (else_line, else_col) = source.offset_to_line_col(else_kw_loc.start_offset());
                // Single-line if/else — alignment is inherently satisfied
                if else_line == if_line {
                    current = None;
                    continue;
                }
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
                if !Self::begins_its_line(source, elsif_kw_loc.start_offset()) {
                    current = elsif_node.subsequent();
                    continue;
                }
                let (elsif_line, elsif_col) =
                    source.offset_to_line_col(elsif_kw_loc.start_offset());
                // Single-line elsif — skip alignment check
                if elsif_line == if_line {
                    current = elsif_node.subsequent();
                    continue;
                }
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
    fn unless_assignment_else_misaligned() {
        // FN: `else` at col 10 should be flagged when `unless` is at col 22
        // (keyword style — else should align with `unless`)
        let source = b"          response = unless identity\n            service.call\n          else\n            other.call\n          end\n";
        let diags = run_cop_full(&ElseAlignment, source);
        assert_eq!(
            diags.len(),
            1,
            "else at col 10 should be flagged when unless is at col 22: {:?}",
            diags
        );
    }

    #[test]
    fn unless_else_correctly_aligned() {
        // No offense: `else` at same column as `unless`
        let source = b"unless condition\n  one\nelse\n  two\nend\n";
        let diags = run_cop_full(&ElseAlignment, source);
        assert!(
            diags.is_empty(),
            "correctly aligned unless/else: {:?}",
            diags
        );
    }

    #[test]
    fn single_line_when_else_no_offense() {
        // FP: `else` on the same line as `when` — no alignment check needed
        let source = b"case\n when 1 then 2 else 3\n end\n";
        let diags = run_cop_full(&ElseAlignment, source);
        assert!(
            diags.is_empty(),
            "single-line when/else should not be flagged: {:?}",
            diags
        );
    }

    #[test]
    fn single_line_in_else_no_offense() {
        // FP: `else` on the same line as `in` — no alignment check needed
        let source = b"case 1\n in a then a + 2 else ;\n 3\n end\n";
        let diags = run_cop_full(&ElseAlignment, source);
        assert!(
            diags.is_empty(),
            "single-line in/else should not be flagged: {:?}",
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
