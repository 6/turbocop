use crate::cop::shared::node_type::{BEGIN_NODE, NIL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for empty rescue bodies (suppressed exceptions).
///
/// ## Investigation (2026-03-10)
/// Corpus: FP=10, FN=0 (1,804 matches, 99.4% conformance).
/// Original analysis incorrectly concluded RuboCop treats trailing comments on rescue
/// lines as satisfying AllowComments. This was reversed in the 2026-03-18 investigation.
///
/// ## Investigation (2026-03-14)
/// Corpus: FP=9, FN=49.
/// FP root cause: in multi-rescue chains, nitrocop scanned for comments only up to
/// the next rescue clause's start line. RuboCop's `comment_between_rescue_and_end?`
/// scans from the rescue line all the way to the ancestor's `end` keyword. This means
/// comments in subsequent rescue clauses or ensure/else blocks satisfy AllowComments
/// for earlier empty rescue clauses. Fix: always use the ancestor begin node's end
/// keyword as the scan boundary, matching RuboCop's behavior.
///
/// ## Investigation (2026-03-18)
/// Corpus: FP=0, FN=50.
/// FN root cause: trailing comments on the rescue line itself (e.g.,
/// `rescue LoadError # comment`) were incorrectly treated as satisfying AllowComments.
/// RuboCop's `comment_between_rescue_and_end?` uses `comment_line?` which matches
/// `/^\s*#/` — only standalone comment lines, not trailing comments. Additionally,
/// `processed_source[node.first_line...end_line]` skips the rescue line itself
/// (0-indexed array with 1-based line number), so trailing comments on the rescue
/// line are never even checked.
/// Fix: removed the trailing-comment-on-rescue-line check. Only standalone comment
/// lines between rescue+1 and end satisfy AllowComments.
pub struct SuppressedException;

impl Cop for SuppressedException {
    fn name(&self) -> &'static str {
        "Lint/SuppressedException"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BEGIN_NODE, NIL_NODE]
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
        // RescueNode is visited via visit_begin_node's specific method,
        // not via the generic visit() dispatch. So we match BeginNode
        // and check its rescue_clause.
        let begin_node = match node.as_begin_node() {
            Some(n) => n,
            None => return,
        };

        let first_rescue = match begin_node.rescue_clause() {
            Some(n) => n,
            None => return,
        };

        // AllowNil: when true, allow `rescue => e; nil; end`
        let allow_nil = config.get_bool("AllowNil", false);
        // AllowComments: if true (default), skip rescue bodies that contain only comments
        let allow_comments = config.get_bool("AllowComments", true);

        // Iterate through all rescue clauses (first + subsequent)
        let mut current_rescue = Some(first_rescue);
        while let Some(rescue_node) = current_rescue {
            let body_stmts = rescue_node.statements();
            let body_empty = match &body_stmts {
                None => true,
                Some(stmts) => stmts.body().is_empty(),
            };

            if body_empty {
                let mut suppressed = true;

                // Check for nil body with AllowNil
                // (empty body is always suppressed, AllowNil only applies to explicit `nil`)

                if allow_comments && suppressed {
                    let (rescue_line, _) =
                        source.offset_to_line_col(rescue_node.keyword_loc().start_offset());
                    // RuboCop scans from the rescue line to the ancestor's end keyword
                    // for comment lines, not just to the next rescue clause. This means
                    // comments anywhere in subsequent rescue/else/ensure blocks satisfy
                    // AllowComments for earlier empty rescue clauses.
                    let clause_end_line = if let Some(end_loc) = begin_node.end_keyword_loc() {
                        source.offset_to_line_col(end_loc.start_offset()).0
                    } else {
                        rescue_line + 1
                    };

                    let lines: Vec<&[u8]> = source.lines().collect();

                    // Check for standalone comment lines between rescue and clause end.
                    // RuboCop's comment_between_rescue_and_end? uses comment_line?
                    // which matches /^\s*#/ — only lines that START with a comment.
                    // Trailing comments on the rescue line (e.g., `rescue # skip`)
                    // do NOT satisfy AllowComments.
                    for line_num in (rescue_line + 1)..clause_end_line {
                        if let Some(line) = lines.get(line_num - 1) {
                            let trimmed = line
                                .iter()
                                .position(|&b| b != b' ' && b != b'\t')
                                .map(|start| &line[start..])
                                .unwrap_or(&[]);
                            if trimmed.starts_with(b"#") {
                                suppressed = false;
                                break;
                            }
                        }
                    }
                }

                if suppressed {
                    let kw_loc = rescue_node.keyword_loc();
                    let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Do not suppress exceptions.".to_string(),
                    ));
                }
            } else if allow_nil {
                if let Some(stmts) = &body_stmts {
                    let body_nodes: Vec<_> = stmts.body().iter().collect();
                    if body_nodes.len() == 1 && body_nodes[0].as_nil_node().is_some() {
                        // AllowNil and body is `nil` — skip this clause
                    }
                }
            }

            current_rescue = rescue_node.subsequent();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SuppressedException, "cops/lint/suppressed_exception");
}
