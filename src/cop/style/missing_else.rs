use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/MissingElse flags if/unless/case without an else clause.
///
/// Investigation (2026-03-15):
/// - Had 0 FP, 7,463 FN in corpus oracle.
/// - Root cause 1 (~90%): only checked `kw == b"if"`, missing `elsif` nodes.
///   RuboCop fires on the LAST elsif in an if/elsif chain when no final else.
/// - Root cause 2: missing `unless` handling (when Style/UnlessElse is disabled).
/// - Root cause 3: message didn't vary based on Style/EmptyElse EnforcedStyle.
/// - Fix: handle elsif chains (walk to last subsequent), handle unless keyword,
///   inject cross-cop config for UnlessElse.Enabled and EmptyElse.EnforcedStyle.
///
/// Investigation (2026-03-27):
/// - Remaining corpus FN are NOT a cop-side AST traversal bug.
/// - Added full-context fixtures for the five reported examples from
///   `oriuminc__vagrant-ariadne__bb22d52`; `cargo test --lib -- cop::style::missing_else`
///   passes, so `visit_if_node` / `visit_case_node` already detect the real syntax.
/// - Reproduced the divergence in the CLI path instead:
///   `target/release/nitrocop --config bench/corpus/baseline_rubocop.yml --only Style/MissingElse`
///   reports the expected 5 offenses on the cloned repo, but the generated overlay config from
///   `bench/corpus/gen_repo_config.py` (`/tmp/nitrocop_corpus_configs/...yml`) reports 0.
/// - The overlay only adds `AllCops: Exclude` and `inherit_from: <baseline>`, so the real bug is
///   in config inheritance / Enabled-state resolution for inherited configs loaded from that temp
///   file, not in this cop's detection logic. A cop-local workaround here would mask the config
///   bug and risks changing real default-enabled behavior.
pub struct MissingElse;

impl Cop for MissingElse {
    fn name(&self) -> &'static str {
        "Style/MissingElse"
    }

    fn default_enabled(&self) -> bool {
        false // Matches vendor config/default.yml: Enabled: false
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "both");
        let unless_else_enabled = config.get_bool("UnlessElseEnabled", true);
        let empty_else_style = config
            .options
            .get("EmptyElseStyle")
            .and_then(|v| v.as_str())
            .unwrap_or("both");
        let mut visitor = MissingElseVisitor {
            cop: self,
            source,
            style,
            unless_else_enabled,
            empty_else_style,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

fn make_message(empty_else_style: &str, node_type: &str) -> String {
    match empty_else_style {
        "empty" => format!("`{node_type}` condition requires an `else`-clause with `nil` in it."),
        "nil" => format!("`{node_type}` condition requires an empty `else`-clause."),
        _ => format!("`{node_type}` condition requires an `else`-clause."),
    }
}

struct MissingElseVisitor<'a> {
    cop: &'a MissingElse,
    source: &'a SourceFile,
    style: &'a str,
    unless_else_enabled: bool,
    empty_else_style: &'a str,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for MissingElseVisitor<'_> {
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        if self.style == "if" || self.style == "both" {
            if let Some(kw_loc) = node.if_keyword_loc() {
                let kw = kw_loc.as_slice();

                if kw == b"if" && node.end_keyword_loc().is_some() {
                    // Top-level block if. Walk to the last node in the elsif chain.
                    // If that last node has no subsequent, it's missing an else.
                    // Walk the elsif chain to find the last node.
                    // If it has no subsequent, it's missing an else.
                    let mut has_else = false;
                    let mut last_offset = node.location().start_offset();
                    let mut is_top_level = true;

                    let mut current_sub = node.subsequent();
                    loop {
                        match current_sub {
                            None => break, // end of chain, no else
                            Some(ref sub) => {
                                if let Some(if_sub) = sub.as_if_node() {
                                    // This is an elsif
                                    last_offset = if_sub
                                        .if_keyword_loc()
                                        .map(|l| l.start_offset())
                                        .unwrap_or_else(|| if_sub.location().start_offset());
                                    is_top_level = false;
                                    current_sub = if_sub.subsequent();
                                } else {
                                    // It's an ElseNode — has an else clause
                                    has_else = true;
                                    break;
                                }
                            }
                        }
                    }

                    if !has_else {
                        let report_offset = if is_top_level {
                            node.location().start_offset()
                        } else {
                            last_offset
                        };
                        let (line, column) = self.source.offset_to_line_col(report_offset);
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            make_message(self.empty_else_style, "if"),
                        ));
                    }
                } else if kw == b"unless" && node.end_keyword_loc().is_some() {
                    // unless without else — only flag when Style/UnlessElse is disabled
                    if !self.unless_else_enabled && node.subsequent().is_none() {
                        let loc = node.location();
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        // RuboCop uses "if" in the message even for unless
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            make_message(self.empty_else_style, "if"),
                        ));
                    }
                }
                // elsif nodes: don't visit independently — handled by the
                // top-level if chain walk above. Skip to avoid double-reporting.
            }
        }

        // Visit children (but NOT subsequent — we handle the chain above)
        self.visit(&node.predicate());
        if let Some(stmts) = node.statements() {
            self.visit(&stmts.as_node());
        }
        if let Some(sub) = node.subsequent() {
            self.visit(&sub);
        }
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        if (self.style == "case" || self.style == "both") && node.else_clause().is_none() {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                make_message(self.empty_else_style, "case"),
            ));
        }

        // Visit children
        if let Some(pred) = node.predicate() {
            self.visit(&pred);
        }
        for condition in node.conditions().iter() {
            self.visit(&condition);
        }
        if let Some(else_clause) = node.else_clause() {
            self.visit(&else_clause.as_node());
        }
    }

    // CaseMatchNode (pattern matching `case...in`) is intentionally not visited.
    // RuboCop's on_case_match is a no-op — pattern matching raises
    // NoMatchingPatternError if no branch matches, so an else is not required.
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MissingElse, "cops/style/missing_else");
}
