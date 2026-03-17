use crate::cop::node_type::{
    BEGIN_NODE, BLOCK_NODE, CLASS_NODE, DEF_NODE, MODULE_NODE, SINGLETON_CLASS_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks alignment of rescue/ensure keywords with their matching begin/def/class/module/block.
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
/// ## FN investigation (2026-03-17)
/// 42 FNs caused by three gaps:
/// 1. Def body wrapping: Prism wraps def bodies with rescue/ensure in an implicit
///    BeginNode, but the cop used `body.as_rescue_node()` which only matches bare
///    RescueNode. Fix: Check `body.as_begin_node()` and extract rescue/ensure.
/// 2. Missing ancestor types: Class, module, singleton class, and block bodies
///    with rescue/ensure were not handled. Prism wraps these in implicit BeginNodes
///    (begin_keyword_loc is None). Fix: Add handlers for all these node types.
/// 3. Rescue chains: Only the first rescue clause was checked; subsequent()
///    rescues in the chain were missed. Fix: Walk the full rescue chain.
///
/// ## FP investigation (2026-03-17)
/// 78 FPs caused by `private def` / `protected def` modifier alignment:
/// When a method is defined with a visibility modifier (`private def foo`),
/// RuboCop aligns rescue/ensure with the modifier (line start), not with `def`.
/// The cop was using `def_kw_loc` which points to `def`, ignoring the preceding
/// modifier. Fix: Compute line indent for the def line; if indent != def_col,
/// something precedes `def` (a modifier), so use indent as align_col instead.
/// Same pattern already used for BeginNode assignment alignment.
pub struct RescueEnsureAlignment;

impl RescueEnsureAlignment {
    /// Check rescue/ensure alignment in an implicit BeginNode body.
    /// `align_col` is the column the rescue/ensure should align to.
    /// `align_line` is the line of the enclosing keyword (for same-line checks).
    /// `keyword` is the name used in the diagnostic message (e.g., "def", "class").
    fn check_implicit_begin(
        &self,
        source: &SourceFile,
        begin_node: &ruby_prism::BeginNode<'_>,
        align_col: usize,
        align_line: usize,
        keyword: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Walk the rescue chain
        let mut rescue_opt = begin_node.rescue_clause();
        while let Some(rescue_node) = rescue_opt {
            let rescue_kw_loc = rescue_node.keyword_loc();
            let (rescue_line, rescue_col) = source.offset_to_line_col(rescue_kw_loc.start_offset());
            if rescue_line != align_line && rescue_col != align_col {
                diagnostics.push(self.diagnostic(
                    source,
                    rescue_line,
                    rescue_col,
                    format!("Align `rescue` with `{keyword}`."),
                ));
            }
            rescue_opt = rescue_node.subsequent();
        }

        if let Some(ensure_node) = begin_node.ensure_clause() {
            let ensure_kw_loc = ensure_node.ensure_keyword_loc();
            let (ensure_line, ensure_col) = source.offset_to_line_col(ensure_kw_loc.start_offset());
            if ensure_line != align_line && ensure_col != align_col {
                diagnostics.push(self.diagnostic(
                    source,
                    ensure_line,
                    ensure_col,
                    format!("Align `ensure` with `{keyword}`."),
                ));
            }
        }
    }
}

impl Cop for RescueEnsureAlignment {
    fn name(&self) -> &'static str {
        "Layout/RescueEnsureAlignment"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BEGIN_NODE,
            DEF_NODE,
            CLASS_NODE,
            MODULE_NODE,
            SINGLETON_CLASS_NODE,
            BLOCK_NODE,
        ]
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
            // Only handle explicit begin (with begin keyword).
            // Implicit begins (from def/class/module/block bodies) are handled
            // by their parent node handlers below.
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

            self.check_implicit_begin(
                source,
                &begin_node,
                align_col,
                begin_line,
                "begin",
                diagnostics,
            );
        } else if let Some(def_node) = node.as_def_node() {
            let def_kw_loc = def_node.def_keyword_loc();
            let (def_line, def_col) = source.offset_to_line_col(def_kw_loc.start_offset());

            // When def is preceded by a visibility modifier on the same line
            // (e.g., `private def foo`), RuboCop aligns rescue/ensure with
            // the modifier (line start), not with the `def` keyword.
            let align_col = {
                let bytes = source.as_bytes();
                let mut line_start = def_kw_loc.start_offset();
                while line_start > 0 && bytes[line_start - 1] != b'\n' {
                    line_start -= 1;
                }
                let mut indent = 0;
                while line_start + indent < bytes.len()
                    && (bytes[line_start + indent] == b' ' || bytes[line_start + indent] == b'\t')
                {
                    indent += 1;
                }
                if indent != def_col { indent } else { def_col }
            };

            if let Some(body) = def_node.body() {
                if let Some(begin_node) = body.as_begin_node() {
                    // Only handle implicit BeginNode (no begin keyword).
                    // Explicit begin...end blocks are handled by the BEGIN_NODE arm.
                    if begin_node.begin_keyword_loc().is_none() {
                        self.check_implicit_begin(
                            source,
                            &begin_node,
                            align_col,
                            def_line,
                            "def",
                            diagnostics,
                        );
                    }
                } else if let Some(rescue_node) = body.as_rescue_node() {
                    // Bare rescue node (rare but possible)
                    let rescue_kw_loc = rescue_node.keyword_loc();
                    let (rescue_line, rescue_col) =
                        source.offset_to_line_col(rescue_kw_loc.start_offset());
                    if rescue_line != def_line && rescue_col != align_col {
                        diagnostics.push(self.diagnostic(
                            source,
                            rescue_line,
                            rescue_col,
                            "Align `rescue` with `def`.".to_string(),
                        ));
                    }
                    // Walk the chain
                    let mut rescue_opt = rescue_node.subsequent();
                    while let Some(sub) = rescue_opt {
                        let kw_loc = sub.keyword_loc();
                        let (line, col) = source.offset_to_line_col(kw_loc.start_offset());
                        if line != def_line && col != align_col {
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                col,
                                "Align `rescue` with `def`.".to_string(),
                            ));
                        }
                        rescue_opt = sub.subsequent();
                    }
                }
            }
        } else if let Some(class_node) = node.as_class_node() {
            let kw_loc = class_node.class_keyword_loc();
            let (kw_line, kw_col) = source.offset_to_line_col(kw_loc.start_offset());

            if let Some(body) = class_node.body() {
                if let Some(begin_node) = body.as_begin_node() {
                    if begin_node.begin_keyword_loc().is_none() {
                        self.check_implicit_begin(
                            source,
                            &begin_node,
                            kw_col,
                            kw_line,
                            "class",
                            diagnostics,
                        );
                    }
                }
            }
        } else if let Some(module_node) = node.as_module_node() {
            let kw_loc = module_node.module_keyword_loc();
            let (kw_line, kw_col) = source.offset_to_line_col(kw_loc.start_offset());

            if let Some(body) = module_node.body() {
                if let Some(begin_node) = body.as_begin_node() {
                    if begin_node.begin_keyword_loc().is_none() {
                        self.check_implicit_begin(
                            source,
                            &begin_node,
                            kw_col,
                            kw_line,
                            "module",
                            diagnostics,
                        );
                    }
                }
            }
        } else if let Some(sclass_node) = node.as_singleton_class_node() {
            let kw_loc = sclass_node.class_keyword_loc();
            let (kw_line, kw_col) = source.offset_to_line_col(kw_loc.start_offset());

            if let Some(body) = sclass_node.body() {
                if let Some(begin_node) = body.as_begin_node() {
                    if begin_node.begin_keyword_loc().is_none() {
                        self.check_implicit_begin(
                            source,
                            &begin_node,
                            kw_col,
                            kw_line,
                            "class",
                            diagnostics,
                        );
                    }
                }
            }
        } else if let Some(block_node) = node.as_block_node() {
            let opening_loc = block_node.opening_loc();
            let opening_slice =
                &source.as_bytes()[opening_loc.start_offset()..opening_loc.end_offset()];
            // Only check do-end blocks, not brace blocks
            if opening_slice != b"do" {
                return;
            }

            let (do_line, _) = source.offset_to_line_col(opening_loc.start_offset());

            // For blocks, RuboCop aligns rescue/ensure with the start of the
            // line containing the `do` keyword (the call expression).
            let bytes = source.as_bytes();
            let mut line_start = opening_loc.start_offset();
            while line_start > 0 && bytes[line_start - 1] != b'\n' {
                line_start -= 1;
            }
            let mut indent = 0;
            while line_start + indent < bytes.len()
                && (bytes[line_start + indent] == b' ' || bytes[line_start + indent] == b'\t')
            {
                indent += 1;
            }
            let align_col = indent;

            if let Some(body) = block_node.body() {
                if let Some(begin_node) = body.as_begin_node() {
                    if begin_node.begin_keyword_loc().is_none() {
                        self.check_implicit_begin(
                            source,
                            &begin_node,
                            align_col,
                            do_line,
                            "do",
                            diagnostics,
                        );
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

    #[test]
    fn def_rescue_misaligned() {
        let src = b"def fetch\n  @store\n    rescue\n    handle\nend\n";
        let diags = run_cop_full(&RescueEnsureAlignment, src);
        assert_eq!(diags.len(), 1, "should flag misaligned rescue in def");
    }

    #[test]
    fn def_ensure_misaligned() {
        let src = b"def process\n  work\n    ensure\n    cleanup\nend\n";
        let diags = run_cop_full(&RescueEnsureAlignment, src);
        assert_eq!(diags.len(), 1, "should flag misaligned ensure in def");
    }

    #[test]
    fn def_rescue_aligned_no_offense() {
        let src = b"def fetch\n  @store\nrescue\n  handle\nend\n";
        let diags = run_cop_full(&RescueEnsureAlignment, src);
        assert!(diags.is_empty(), "aligned rescue in def should not fire");
    }

    #[test]
    fn class_rescue_misaligned() {
        let src = b"class Foo\n  bar\n    rescue\n    handle\nend\n";
        let diags = run_cop_full(&RescueEnsureAlignment, src);
        assert_eq!(diags.len(), 1, "should flag misaligned rescue in class");
    }

    #[test]
    fn module_ensure_misaligned() {
        let src = b"module Foo\n  bar\n    ensure\n    handle\nend\n";
        let diags = run_cop_full(&RescueEnsureAlignment, src);
        assert_eq!(diags.len(), 1, "should flag misaligned ensure in module");
    }

    #[test]
    fn block_rescue_misaligned() {
        let src = b"items.each do |i|\n  i.call\n    rescue\n    next\nend\n";
        let diags = run_cop_full(&RescueEnsureAlignment, src);
        assert_eq!(diags.len(), 1, "should flag misaligned rescue in block");
    }

    #[test]
    fn rescue_chain_subsequent_misaligned() {
        let src = b"begin\n  call\nrescue Timeout\n  retry\n  rescue\nend\n";
        let diags = run_cop_full(&RescueEnsureAlignment, src);
        assert_eq!(diags.len(), 1, "should flag misaligned subsequent rescue");
    }

    #[test]
    fn private_def_rescue_aligned_no_offense() {
        // rescue aligned with `private` (line start), not `def`
        let src = b"private def fetch\n  work\nrescue\n  handle\nend\n";
        let diags = run_cop_full(&RescueEnsureAlignment, src);
        assert!(
            diags.is_empty(),
            "rescue aligned with private modifier should not fire"
        );
    }

    #[test]
    fn private_def_ensure_aligned_no_offense() {
        // ensure aligned with `private` (line start), not `def`
        let src = b"private def process\n  work\nensure\n  cleanup\nend\n";
        let diags = run_cop_full(&RescueEnsureAlignment, src);
        assert!(
            diags.is_empty(),
            "ensure aligned with private modifier should not fire"
        );
    }

    #[test]
    fn private_def_rescue_misaligned() {
        // rescue aligned with `def` instead of `private` — should be flagged
        let src = b"private def fetch\n  work\n        rescue\n  handle\nend\n";
        let diags = run_cop_full(&RescueEnsureAlignment, src);
        assert_eq!(
            diags.len(),
            1,
            "rescue misaligned with private def should fire"
        );
    }
}
