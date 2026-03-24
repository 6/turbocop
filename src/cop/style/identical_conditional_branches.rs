use crate::cop::node_type::{CASE_MATCH_NODE, CASE_NODE, IF_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Style/IdenticalConditionalBranches
///
/// Checks for identical expressions at the beginning (head) or end (tail) of
/// each branch of a conditional expression: `if/elsif/else`, `case/when/else`,
/// and `case/in/else` (pattern matching).
///
/// ## Investigation findings
///
/// Original implementation only handled simple `if/else` tail checks. FN=606
/// was caused by missing:
/// - `case/when/else` (CaseNode) support
/// - `case/in/else` (CaseMatchNode / pattern matching) support
/// - `if/elsif/else` chain expansion (walking nested elsif branches)
/// - Leading expression (head) detection
/// - Suppression for leading expressions when assigned to condition variable
/// - Suppression for leading expressions with single-child branches at end of parent
///
/// FP=10 were caused by firing on `if/elsif/else` chains without requiring ALL
/// branches (including elsif) to have the same expression — the old code only
/// checked the if and else branches, ignoring elsif.
pub struct IdenticalConditionalBranches;

/// Check if a node contains any heredoc string nodes.
fn contains_heredoc(node: &ruby_prism::Node<'_>) -> bool {
    struct HeredocChecker {
        found: bool,
    }
    impl<'pr> Visit<'pr> for HeredocChecker {
        fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
            if let Some(opening) = node.opening_loc() {
                if opening.as_slice().starts_with(b"<<") {
                    self.found = true;
                    return;
                }
            }
            ruby_prism::visit_string_node(self, node);
        }
        fn visit_interpolated_string_node(
            &mut self,
            node: &ruby_prism::InterpolatedStringNode<'pr>,
        ) {
            if let Some(opening) = node.opening_loc() {
                if opening.as_slice().starts_with(b"<<") {
                    self.found = true;
                    return;
                }
            }
            ruby_prism::visit_interpolated_string_node(self, node);
        }
        fn visit_interpolated_x_string_node(
            &mut self,
            node: &ruby_prism::InterpolatedXStringNode<'pr>,
        ) {
            if node.opening_loc().as_slice().starts_with(b"<<") {
                self.found = true;
                return;
            }
            ruby_prism::visit_interpolated_x_string_node(self, node);
        }
        fn visit_x_string_node(&mut self, node: &ruby_prism::XStringNode<'pr>) {
            if node.opening_loc().as_slice().starts_with(b"<<") {
                self.found = true;
                return;
            }
            ruby_prism::visit_x_string_node(self, node);
        }
    }
    let mut checker = HeredocChecker { found: false };
    checker.visit(node);
    checker.found
}

/// Extract the source text, location, and heredoc flag for a specific statement
/// in a StatementsNode (by index).
fn stmt_info(
    source: &SourceFile,
    stmts: &ruby_prism::StatementsNode<'_>,
    index: usize,
) -> Option<(String, usize, usize, bool)> {
    let body: Vec<_> = stmts.body().iter().collect();
    let node = body.get(index)?;
    let loc = node.location();
    let src = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
    let (line, col) = source.offset_to_line_col(loc.start_offset());
    let has_heredoc = contains_heredoc(node);
    Some((
        String::from_utf8_lossy(src).trim().to_string(),
        line,
        col,
        has_heredoc,
    ))
}

/// A branch in a conditional: its statements node (if present) and number of
/// statements.
struct BranchInfo<'pr> {
    stmts: Option<ruby_prism::StatementsNode<'pr>>,
    count: usize,
}

impl<'pr> BranchInfo<'pr> {
    fn from_stmts(stmts: Option<ruby_prism::StatementsNode<'pr>>) -> Self {
        let count = stmts.as_ref().map(|s| s.body().iter().count()).unwrap_or(0);
        Self { stmts, count }
    }
}

impl IdenticalConditionalBranches {
    /// Collect all branches from an if/elsif/else chain, expanding nested elsifs.
    fn collect_if_branches<'pr>(if_node: &ruby_prism::IfNode<'pr>) -> Option<Vec<BranchInfo<'pr>>> {
        let mut branches = Vec::new();
        branches.push(BranchInfo::from_stmts(if_node.statements()));

        let mut subsequent = if_node.subsequent();
        loop {
            match subsequent {
                None => {
                    // No else clause at all (if/elsif without else) — not exhaustive
                    return None;
                }
                Some(node) => {
                    if let Some(elsif_node) = node.as_if_node() {
                        // This is an elsif
                        branches.push(BranchInfo::from_stmts(elsif_node.statements()));
                        subsequent = elsif_node.subsequent();
                    } else if let Some(else_node) = node.as_else_node() {
                        // Terminal else
                        branches.push(BranchInfo::from_stmts(else_node.statements()));
                        break;
                    } else {
                        return None;
                    }
                }
            }
        }

        Some(branches)
    }

    /// Collect all branches from a case/when/else node.
    fn collect_case_branches<'pr>(
        case_node: &ruby_prism::CaseNode<'pr>,
    ) -> Option<Vec<BranchInfo<'pr>>> {
        // Must have an else clause
        let else_clause = case_node.else_clause()?;

        let mut branches = Vec::new();
        for when in case_node.conditions().iter() {
            if let Some(when_node) = when.as_when_node() {
                branches.push(BranchInfo::from_stmts(when_node.statements()));
            }
        }
        branches.push(BranchInfo::from_stmts(else_clause.statements()));
        Some(branches)
    }

    /// Collect all branches from a case/in/else node (pattern matching).
    fn collect_case_match_branches<'pr>(
        case_node: &ruby_prism::CaseMatchNode<'pr>,
    ) -> Option<Vec<BranchInfo<'pr>>> {
        // Must have an else clause
        let else_clause = case_node.else_clause()?;

        let mut branches = Vec::new();
        for in_node in case_node.conditions().iter() {
            if let Some(in_node) = in_node.as_in_node() {
                branches.push(BranchInfo::from_stmts(in_node.statements()));
            }
        }
        branches.push(BranchInfo::from_stmts(else_clause.statements()));
        Some(branches)
    }

    /// Remove duplicate diagnostics added from `start_idx` onwards (same line+col).
    fn dedup_diagnostics(diagnostics: &mut Vec<Diagnostic>, start_idx: usize) {
        let mut seen = std::collections::HashSet::new();
        let mut i = start_idx;
        while i < diagnostics.len() {
            let key = (diagnostics[i].location.line, diagnostics[i].location.column);
            if seen.contains(&key) {
                diagnostics.remove(i);
            } else {
                seen.insert(key);
                i += 1;
            }
        }
    }

    /// Check identical tail (last statement) across all branches.
    fn check_tails(
        &self,
        source: &SourceFile,
        branches: &[BranchInfo<'_>],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // All branches must have at least one statement
        if branches.iter().any(|b| b.count == 0) {
            return;
        }

        // Get tail (last statement) from each branch
        let mut tails: Vec<(String, usize, usize, bool)> = Vec::new();
        for branch in branches {
            let stmts = match &branch.stmts {
                Some(s) => s,
                None => return,
            };
            match stmt_info(source, stmts, branch.count - 1) {
                Some(info) => tails.push(info),
                None => return,
            }
        }

        // Skip if any tail contains a heredoc
        if tails.iter().any(|(_, _, _, has_heredoc)| *has_heredoc) {
            return;
        }

        // All tails must be identical
        let first_src = &tails[0].0;
        if first_src.is_empty() {
            return;
        }
        if !tails.iter().all(|(src, _, _, _)| src == first_src) {
            return;
        }

        // Report offense on every branch's tail (RuboCop flags all of them)
        let msg = format!("Move `{}` out of the conditional.", tails[0].0);
        for (_, line, col, _) in &tails {
            diagnostics.push(self.diagnostic(source, *line, *col, msg.clone()));
        }
    }

    /// Check identical head (first statement) across all branches.
    fn check_heads(
        &self,
        source: &SourceFile,
        branches: &[BranchInfo<'_>],
        condition_node: Option<&ruby_prism::Node<'_>>,
        is_last_child_of_parent: bool,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // All branches must have at least one statement
        if branches.iter().any(|b| b.count == 0) {
            return;
        }

        // Suppression: if this is the last child of the parent and any branch
        // has only a single statement, skip head check (can't extract without
        // changing return value semantics).
        if is_last_child_of_parent && branches.iter().any(|b| b.count == 1) {
            return;
        }

        // Get head (first statement) from each branch
        let mut heads: Vec<(String, usize, usize, bool)> = Vec::new();
        for branch in branches {
            let stmts = match &branch.stmts {
                Some(s) => s,
                None => return,
            };
            match stmt_info(source, stmts, 0) {
                Some(info) => heads.push(info),
                None => return,
            }
        }

        // Skip if any head contains a heredoc
        if heads.iter().any(|(_, _, _, has_heredoc)| *has_heredoc) {
            return;
        }

        // All heads must be identical
        let first_src = &heads[0].0;
        if first_src.is_empty() {
            return;
        }
        if !heads.iter().all(|(src, _, _, _)| src == first_src) {
            return;
        }

        // Suppression: if the head is an assignment and the LHS matches the
        // condition variable, skip (moving it before the conditional would
        // change semantics).
        if let Some(cond) = condition_node {
            if is_assignment_to_condition(source, first_src, cond) {
                return;
            }
        }

        // Report offense on every branch's head (RuboCop flags all of them)
        let msg = format!("Move `{}` out of the conditional.", heads[0].0);
        for (_, line, col, _) in &heads {
            diagnostics.push(self.diagnostic(source, *line, *col, msg.clone()));
        }
    }
}

/// Check if the head expression is an assignment whose LHS matches the
/// condition variable (or its receiver). RuboCop suppresses these to avoid
/// changing semantics.
fn is_assignment_to_condition(
    source: &SourceFile,
    head_src: &str,
    condition: &ruby_prism::Node<'_>,
) -> bool {
    // Check for `x = ...` style assignments
    // The head source might be `x = value`, `@x = value`, `x += 1`, etc.
    // Extract the LHS (before ` =`, ` +=`, ` ||=`, etc.)
    let lhs = if let Some(pos) = head_src.find(" =") {
        head_src[..pos].trim()
    } else if let Some(pos) = head_src.find(" +=") {
        head_src[..pos].trim()
    } else if let Some(pos) = head_src.find(" -=") {
        head_src[..pos].trim()
    } else if let Some(pos) = head_src.find(" ||=") {
        head_src[..pos].trim()
    } else if let Some(pos) = head_src.find(" &&=") {
        head_src[..pos].trim()
    } else {
        return false;
    };

    // Get condition source
    let cond_loc = condition.location();
    let cond_bytes = &source.as_bytes()[cond_loc.start_offset()..cond_loc.end_offset()];
    let cond_src = String::from_utf8_lossy(cond_bytes);
    let cond_src = cond_src.trim();

    // Direct match: `if x` and `x = ...`
    if lhs == cond_src {
        return true;
    }

    // Receiver match: `if x.something` or `if x&.something` and `x = ...`
    // Extract receiver from condition (before `.` or `&.`)
    if let Some(call_node) = condition.as_call_node() {
        if let Some(receiver) = call_node.receiver() {
            let recv_loc = receiver.location();
            let recv_bytes = &source.as_bytes()[recv_loc.start_offset()..recv_loc.end_offset()];
            let recv_src = String::from_utf8_lossy(recv_bytes);
            let recv_src = recv_src.trim();
            if lhs == recv_src {
                return true;
            }
        }
    }

    // Check for index-style `h[:key]` in condition and head
    // e.g., `if h[:key]` and `h[:key] = foo`
    if let Some(call_node) = condition.as_call_node() {
        if call_node.name().as_slice() == b"[]" {
            // The condition is an indexing operation like h[:key]
            if lhs == cond_src || head_src.starts_with(&format!("{cond_src} ")) {
                return true;
            }
        }
    }

    false
}

/// Check if the conditional `node` is the last expression in its parent scope
/// (method body, block, etc.). This is used to suppress head checks for
/// single-child branches, matching RuboCop's `last_child_of_parent?` behavior.
fn is_last_child_of_parent(
    node: &ruby_prism::Node<'_>,
    parse_result: &ruby_prism::ParseResult<'_>,
) -> bool {
    // Walk the AST to find the parent of our node.
    // We check if the node's start offset matches as the last statement in any
    // parent StatementsNode. This is a heuristic that works for method bodies,
    // blocks, etc.
    let target_offset = node.location().start_offset();

    struct ParentFinder {
        target_offset: usize,
        is_last: bool,
    }
    impl<'pr> Visit<'pr> for ParentFinder {
        fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
            let body: Vec<_> = node.body().iter().collect();
            if let Some(last) = body.last() {
                if last.location().start_offset() == self.target_offset {
                    self.is_last = true;
                }
            }
            ruby_prism::visit_statements_node(self, node);
        }
    }

    let mut finder = ParentFinder {
        target_offset,
        is_last: false,
    };
    finder.visit(&parse_result.node());
    finder.is_last
}

impl Cop for IdenticalConditionalBranches {
    fn name(&self) -> &'static str {
        "Style/IdenticalConditionalBranches"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, CASE_NODE, CASE_MATCH_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if let Some(if_node) = node.as_if_node() {
            // Skip elsif nodes — we process the full chain from the top-level if
            if let Some(kw_loc) = if_node.if_keyword_loc() {
                if kw_loc.as_slice() == b"elsif" {
                    return;
                }
            } else {
                // No keyword loc — this is a ternary or modifier if
                // RuboCop still flags ternaries, but we handle them via the
                // same branch expansion
            }

            let branches = match Self::collect_if_branches(&if_node) {
                Some(b) => b,
                None => return, // no else clause
            };

            let pre_len = diagnostics.len();

            // Check tails (last statement in each branch)
            self.check_tails(source, &branches, diagnostics);

            // Check heads (first statement in each branch)
            let condition = if_node.predicate();
            let last_child = is_last_child_of_parent(node, parse_result);
            self.check_heads(source, &branches, Some(&condition), last_child, diagnostics);

            // Deduplicate: when both head and tail fire on single-stmt branches
            Self::dedup_diagnostics(diagnostics, pre_len);
        } else if let Some(case_node) = node.as_case_node() {
            let branches = match Self::collect_case_branches(&case_node) {
                Some(b) => b,
                None => return,
            };

            let pre_len = diagnostics.len();

            self.check_tails(source, &branches, diagnostics);

            let condition = case_node.predicate();
            let last_child = is_last_child_of_parent(node, parse_result);
            self.check_heads(
                source,
                &branches,
                condition.as_ref(),
                last_child,
                diagnostics,
            );

            Self::dedup_diagnostics(diagnostics, pre_len);
        } else if let Some(case_match_node) = node.as_case_match_node() {
            let branches = match Self::collect_case_match_branches(&case_match_node) {
                Some(b) => b,
                None => return,
            };

            let pre_len = diagnostics.len();

            self.check_tails(source, &branches, diagnostics);

            let condition = case_match_node.predicate();
            let last_child = is_last_child_of_parent(node, parse_result);
            self.check_heads(
                source,
                &branches,
                condition.as_ref(),
                last_child,
                diagnostics,
            );

            Self::dedup_diagnostics(diagnostics, pre_len);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        IdenticalConditionalBranches,
        "cops/style/identical_conditional_branches"
    );
}
