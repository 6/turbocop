use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ELSE_NODE, IF_NODE};

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
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
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
        // Use `end` keyword column as alignment target when available.
        // This correctly handles both variable-style (`x = if ... end` with end at LHS)
        // and keyword-style (`x = if ... end` with end at keyword) assignments,
        // as well as non-assignment contexts like `x << if ... end`.
        let base_col = if let Some(end_loc) = if_node.end_keyword_loc() {
            source.offset_to_line_col(end_loc.start_offset()).1
        } else {
            if_col
        };

        let mut current = if_node.subsequent();

        while let Some(subsequent) = current {
            if let Some(else_node) = subsequent.as_else_node() {
                let else_kw_loc = else_node.else_keyword_loc();
                let (else_line, else_col) =
                    source.offset_to_line_col(else_kw_loc.start_offset());
                if else_col != base_col {
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
                if elsif_col != base_col {
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
    fn assignment_context_else_aligns_with_lhs() {
        // `else` at column 0 aligns with `x` (LHS), not `if` (column 4)
        let source = b"x = if foo\n  bar\nelse\n  baz\nend\n";
        let diags = run_cop_full(&ElseAlignment, source);
        assert!(diags.is_empty(), "assignment context else should align with LHS: {:?}", diags);
    }

    #[test]
    fn assignment_context_elsif_aligns_with_lhs() {
        let source = b"x = if foo\n  bar\nelsif qux\n  baz\nelse\n  quux\nend\n";
        let diags = run_cop_full(&ElseAlignment, source);
        assert!(diags.is_empty(), "assignment context elsif should align with LHS: {:?}", diags);
    }

    #[test]
    fn assignment_context_else_wrong_alignment() {
        // Variable style: `end` at col 0 (LHS), but `else` at col 4 (wrong)
        let source = b"x = if foo\n  bar\n    else\n  baz\nend\n";
        let diags = run_cop_full(&ElseAlignment, source);
        assert_eq!(diags.len(), 1, "should flag else not aligned with LHS");
    }

    #[test]
    fn assignment_context_keyword_style_no_offense() {
        // Keyword style: `end` at col 4 (with `if`), body/else aligned with `if`
        let source = b"x = if foo\n      bar\n    else\n      baz\n    end\n";
        let diags = run_cop_full(&ElseAlignment, source);
        assert!(diags.is_empty(), "keyword style should not flag else aligned with if: {:?}", diags);
    }
}
