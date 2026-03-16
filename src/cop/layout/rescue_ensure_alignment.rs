use crate::cop::node_type::{BEGIN_NODE, DEF_NODE, RESCUE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks alignment of rescue/ensure keywords with their matching begin/def.
///
/// ## FP investigation (2026-03-16)
/// 128 FPs caused by two bugs:
/// 1. Tab indentation: The assignment-detection heuristic counted only spaces
///    for line indent, but compared against byte-offset column. Tab-indented
///    code had indent=0 (no spaces) vs begin_col>0 (tabs), triggering the
///    assignment path which set align_col=0, causing false misalignment reports.
///    Fix: Count both spaces and tabs as leading whitespace.
/// 2. Same-line begin/rescue: `begin; something; rescue; nil; end` on a single
///    line was flagged because no same-line check existed. RuboCop skips these
///    via `same_line?`. Fix: Skip when rescue/ensure is on the same line as begin.
///
/// ## Remaining FN gaps (42)
/// Rescue/ensure inside class, module, singleton class, and block bodies are
/// not yet detected. Prism wraps these in implicit BeginNodes (begin_keyword_loc
/// is None), which the cop currently skips. Also, the def handler uses
/// `body.as_rescue_node()` but Prism wraps def bodies with rescue in a
/// BeginNode, not a bare RescueNode.
pub struct RescueEnsureAlignment;

impl Cop for RescueEnsureAlignment {
    fn name(&self) -> &'static str {
        "Layout/RescueEnsureAlignment"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BEGIN_NODE, DEF_NODE, RESCUE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if let Some(begin_node) = node.as_begin_node() {
            let begin_kw_loc = match begin_node.begin_keyword_loc() {
                Some(loc) => loc,
                None => return,
            };
            let (begin_line, begin_col) = source.offset_to_line_col(begin_kw_loc.start_offset());

            // When begin is used as an assignment value (e.g., `x = begin`),
            // RuboCop aligns rescue/ensure with the start of the line (the variable),
            // not with the `begin` keyword.
            let align_col = {
                let bytes = source.as_bytes();
                let mut line_start = begin_kw_loc.start_offset();
                while line_start > 0 && bytes[line_start - 1] != b'\n' {
                    line_start -= 1;
                }
                // Count leading whitespace (both spaces and tabs) to find
                // the column of the first non-whitespace character on this line.
                let mut indent = 0;
                while line_start + indent < bytes.len()
                    && (bytes[line_start + indent] == b' ' || bytes[line_start + indent] == b'\t')
                {
                    indent += 1;
                }
                // If begin is NOT at the start of the line (i.e., something
                // precedes it like `x = begin`), use the line's indent.
                if indent != begin_col {
                    indent
                } else {
                    begin_col
                }
            };

            if let Some(rescue_node) = begin_node.rescue_clause() {
                let rescue_kw_loc = rescue_node.keyword_loc();
                let (rescue_line, rescue_col) =
                    source.offset_to_line_col(rescue_kw_loc.start_offset());
                // Skip if rescue is on the same line as begin (single-line form)
                if rescue_line != begin_line && rescue_col != align_col {
                    diagnostics.push(self.diagnostic(
                        source,
                        rescue_line,
                        rescue_col,
                        "Align `rescue` with `begin`.".to_string(),
                    ));
                }
            }

            if let Some(ensure_node) = begin_node.ensure_clause() {
                let ensure_kw_loc = ensure_node.ensure_keyword_loc();
                let (ensure_line, ensure_col) =
                    source.offset_to_line_col(ensure_kw_loc.start_offset());
                // Skip if ensure is on the same line as begin (single-line form)
                if ensure_line != begin_line && ensure_col != align_col {
                    diagnostics.push(self.diagnostic(
                        source,
                        ensure_line,
                        ensure_col,
                        "Align `ensure` with `begin`.".to_string(),
                    ));
                }
            }
        } else if let Some(def_node) = node.as_def_node() {
            let def_kw_loc = def_node.def_keyword_loc();
            let (_, def_col) = source.offset_to_line_col(def_kw_loc.start_offset());

            // Check if the def body is a rescue node
            if let Some(body) = def_node.body() {
                if let Some(rescue_node) = body.as_rescue_node() {
                    let rescue_kw_loc = rescue_node.keyword_loc();
                    let (rescue_line, rescue_col) =
                        source.offset_to_line_col(rescue_kw_loc.start_offset());
                    if rescue_col != def_col {
                        diagnostics.push(self.diagnostic(
                            source,
                            rescue_line,
                            rescue_col,
                            "Align `rescue` with `def`.".to_string(),
                        ));
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

    crate::cop_fixture_tests!(RescueEnsureAlignment, "cops/layout/rescue_ensure_alignment");

    #[test]
    fn no_rescue_no_offense() {
        let source = b"begin\n  foo\nend\n";
        let diags = run_cop_full(&RescueEnsureAlignment, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn same_line_begin_rescue_no_offense() {
        // Single-line begin/rescue should not fire
        let src = b"begin; do_something; rescue LoadError; end\n";
        let diags = run_cop_full(&RescueEnsureAlignment, src);
        assert!(diags.is_empty(), "same-line begin/rescue should not fire");
    }

    #[test]
    fn tab_indented_begin_rescue_no_offense() {
        // Tab-indented begin/rescue correctly aligned should not fire
        let src = b"\tbegin\n\t\tdo_something\n\trescue\n\t\thandle\n\tend\n";
        let diags = run_cop_full(&RescueEnsureAlignment, src);
        assert!(
            diags.is_empty(),
            "tab-indented aligned begin/rescue should not fire"
        );
    }
}
