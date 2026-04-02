use crate::cop::node_type::{CASE_MATCH_NODE, CASE_NODE, IF_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Style/IdenticalConditionalBranches
///
/// Checks for identical expressions at the beginning (head) or end (tail) of
/// each branch of a conditional expression: `if/elsif/else`, `unless/else`,
/// `case/when/else`, and `case/in/else` (pattern matching).
///
/// ## Investigation findings (round 2)
///
/// 1. **FN: `unless/else` support** — Prism uses a separate `UnlessNode` type
///    (not `IfNode`). The cop now handles `UNLESS_NODE` to detect identical
///    heads/tails in `unless/else` blocks.
///
/// 2. **FP: assignment value vs condition variable** — RuboCop's
///    `duplicated_expressions?` suppresses identical assignments when the
///    value (RHS) matches a variable in the condition (e.g.,
///    `if obj.is_a?(X); @y = obj; else; @y = obj; end` inside a method
///    where `obj` is a local variable). Added `assignment_child_source`
///    check for both heads and tails.
///
/// 3. **FP: conditional inside assignment** — `y = if cond; ...; end`
///    makes the conditional the "last child" of the assignment node.
///    RuboCop's `last_child_of_parent?` returns true, suppressing single-
///    child-branch head checks. Fixed `is_last_child_of_parent` to also
///    check write nodes (LocalVariableWriteNode, etc.).
pub struct IdenticalConditionalBranches;

struct StatementInfo {
    src: String,
    key: String,
    line: usize,
    col: usize,
    has_heredoc: bool,
    index_assignment_receiver: Option<String>,
    /// Source of the "first child node" for assignments, used by RuboCop's
    /// `duplicated_expressions?` to suppress when the value (for simple writes)
    /// or LHS variable name (for operator writes) matches a condition variable.
    assignment_child_source: Option<String>,
}

fn node_source(source: &SourceFile, node: &ruby_prism::Node<'_>) -> String {
    let loc = node.location();
    let src = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
    String::from_utf8_lossy(src).trim().to_string()
}

fn normalized_source_key(src: &str) -> String {
    #[derive(Clone, Copy)]
    enum QuoteState {
        None,
        Single,
        Double,
    }

    let mut out = String::with_capacity(src.len());
    let mut quote = QuoteState::None;
    let mut escaped = false;
    let mut pending_space = false;

    for ch in src.chars() {
        match quote {
            QuoteState::None => {
                if ch.is_whitespace() {
                    pending_space = true;
                    continue;
                }

                if pending_space && !out.is_empty() {
                    out.push(' ');
                }
                pending_space = false;
                out.push(ch);

                quote = match ch {
                    '\'' => QuoteState::Single,
                    '"' => QuoteState::Double,
                    _ => QuoteState::None,
                };
            }
            QuoteState::Single => {
                out.push(ch);
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '\'' {
                    quote = QuoteState::None;
                }
            }
            QuoteState::Double => {
                out.push(ch);
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    quote = QuoteState::None;
                }
            }
        }
    }

    out
}

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

/// Extract the source that RuboCop's `duplicated_expressions?` compares against
/// condition variables.  For simple writes (lvasgn, ivasgn, …) this is the
/// VALUE (RHS); for operator writes (op_asgn) it is the variable NAME (LHS).
fn assignment_child_source(source: &SourceFile, node: &ruby_prism::Node<'_>) -> Option<String> {
    // Simple writes: child_nodes.first in RuboCop = value (RHS)
    if let Some(w) = node.as_local_variable_write_node() {
        return Some(node_source(source, &w.value()));
    }
    if let Some(w) = node.as_instance_variable_write_node() {
        return Some(node_source(source, &w.value()));
    }
    if let Some(w) = node.as_class_variable_write_node() {
        return Some(node_source(source, &w.value()));
    }
    if let Some(w) = node.as_global_variable_write_node() {
        return Some(node_source(source, &w.value()));
    }
    if let Some(w) = node.as_constant_write_node() {
        return Some(node_source(source, &w.value()));
    }
    // Operator writes: child_nodes.first in RuboCop = LHS variable name
    if let Some(w) = node.as_local_variable_operator_write_node() {
        return Some(String::from_utf8_lossy(w.name().as_slice()).to_string());
    }
    if let Some(w) = node.as_instance_variable_operator_write_node() {
        return Some(String::from_utf8_lossy(w.name().as_slice()).to_string());
    }
    if let Some(w) = node.as_local_variable_or_write_node() {
        return Some(String::from_utf8_lossy(w.name().as_slice()).to_string());
    }
    if let Some(w) = node.as_local_variable_and_write_node() {
        return Some(String::from_utf8_lossy(w.name().as_slice()).to_string());
    }
    if let Some(w) = node.as_instance_variable_or_write_node() {
        return Some(String::from_utf8_lossy(w.name().as_slice()).to_string());
    }
    if let Some(w) = node.as_instance_variable_and_write_node() {
        return Some(String::from_utf8_lossy(w.name().as_slice()).to_string());
    }
    None
}

/// Extract the source text, location, and heredoc flag for a specific statement
/// in a StatementsNode (by index).
fn stmt_info(
    source: &SourceFile,
    stmts: &ruby_prism::StatementsNode<'_>,
    index: usize,
) -> Option<StatementInfo> {
    let body: Vec<_> = stmts.body().iter().collect();
    let node = body.get(index)?;
    let loc = node.location();
    let (line, col) = source.offset_to_line_col(loc.start_offset());
    let has_heredoc = contains_heredoc(node);
    let src = node_source(source, node);
    Some(StatementInfo {
        key: normalized_source_key(&src),
        src,
        line,
        col,
        has_heredoc,
        index_assignment_receiver: index_assignment_receiver_source(source, node),
        assignment_child_source: assignment_child_source(source, node),
    })
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
        condition_node: Option<&ruby_prism::Node<'_>>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // All branches must have at least one statement
        if branches.iter().any(|b| b.count == 0) {
            return;
        }

        // Get tail (last statement) from each branch
        let mut tails: Vec<StatementInfo> = Vec::new();
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
        if tails.iter().any(|tail| tail.has_heredoc) {
            return;
        }

        // All tails must be identical
        let first_src = &tails[0].src;
        if first_src.is_empty() {
            return;
        }
        let first_key = &tails[0].key;
        if !tails.iter().all(|tail| tail.key == *first_key) {
            return;
        }

        if let Some(condition) = condition_node {
            if let Some(receiver) = tails[0].index_assignment_receiver.as_deref() {
                if condition_contains_variable_source(source, condition, receiver) {
                    return;
                }
            }

            // RuboCop's `duplicated_expressions?` suppression: if the tail is
            // an assignment and the value (or LHS for operator writes) matches
            // a variable in the condition, skip.
            if let Some(child_src) = &tails[0].assignment_child_source {
                if condition_contains_variable_source(source, condition, child_src) {
                    return;
                }
            }
        }

        // Report offense on every branch's tail (RuboCop flags all of them)
        let msg = format!("Move `{}` out of the conditional.", tails[0].src);
        for tail in &tails {
            diagnostics.push(self.diagnostic(source, tail.line, tail.col, msg.clone()));
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
        let mut heads: Vec<StatementInfo> = Vec::new();
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
        if heads.iter().any(|head| head.has_heredoc) {
            return;
        }

        // All heads must be identical
        let first_src = &heads[0].src;
        if first_src.is_empty() {
            return;
        }
        let first_key = &heads[0].key;
        if !heads.iter().all(|head| head.key == *first_key) {
            return;
        }

        // Suppression: if the head is an assignment and the LHS matches the
        // condition variable, skip (moving it before the conditional would
        // change semantics).
        if let Some(cond) = condition_node {
            if is_assignment_to_condition(source, first_src, cond) {
                return;
            }

            // RuboCop's `duplicated_expressions?` suppression: if the head is
            // an assignment and the value (or LHS for operator writes) matches
            // a variable in the condition, skip.
            if let Some(child_src) = &heads[0].assignment_child_source {
                if condition_contains_variable_source(source, cond, child_src) {
                    return;
                }
            }
        }

        // Report offense on every branch's head (RuboCop flags all of them)
        let msg = format!("Move `{}` out of the conditional.", heads[0].src);
        for head in &heads {
            diagnostics.push(self.diagnostic(source, head.line, head.col, msg.clone()));
        }
    }
}

fn index_assignment_receiver_source(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
) -> Option<String> {
    let call = node.as_call_node()?;
    if call.name().as_slice() != b"[]=" {
        return None;
    }

    let receiver = call.receiver()?;
    Some(node_source(source, &receiver))
}

fn condition_contains_variable_source(
    source: &SourceFile,
    condition: &ruby_prism::Node<'_>,
    needle: &str,
) -> bool {
    struct VariableFinder<'a> {
        source: &'a SourceFile,
        needle: &'a str,
        found: bool,
    }

    impl<'a, 'pr> Visit<'pr> for VariableFinder<'a> {
        fn visit_instance_variable_read_node(
            &mut self,
            node: &ruby_prism::InstanceVariableReadNode<'pr>,
        ) {
            if node_source(self.source, &node.as_node()) == self.needle {
                self.found = true;
            }
        }

        fn visit_local_variable_read_node(
            &mut self,
            node: &ruby_prism::LocalVariableReadNode<'pr>,
        ) {
            if node_source(self.source, &node.as_node()) == self.needle {
                self.found = true;
            }
        }
    }

    let mut finder = VariableFinder {
        source,
        needle,
        found: false,
    };
    finder.visit(condition);
    finder.found
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

    impl ParentFinder {
        /// Check if a value node matches the target (i.e. the conditional is
        /// the value of an assignment like `y = if ...`).
        fn check_value(&mut self, value: &ruby_prism::Node<'_>) {
            if value.location().start_offset() == self.target_offset {
                self.is_last = true;
            }
        }
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

        // Assignment nodes: the conditional is the "last child" when it's the
        // value of an assignment (e.g. `y = if ...`).
        fn visit_local_variable_write_node(
            &mut self,
            node: &ruby_prism::LocalVariableWriteNode<'pr>,
        ) {
            self.check_value(&node.value());
            ruby_prism::visit_local_variable_write_node(self, node);
        }
        fn visit_instance_variable_write_node(
            &mut self,
            node: &ruby_prism::InstanceVariableWriteNode<'pr>,
        ) {
            self.check_value(&node.value());
            ruby_prism::visit_instance_variable_write_node(self, node);
        }
        fn visit_class_variable_write_node(
            &mut self,
            node: &ruby_prism::ClassVariableWriteNode<'pr>,
        ) {
            self.check_value(&node.value());
            ruby_prism::visit_class_variable_write_node(self, node);
        }
        fn visit_global_variable_write_node(
            &mut self,
            node: &ruby_prism::GlobalVariableWriteNode<'pr>,
        ) {
            self.check_value(&node.value());
            ruby_prism::visit_global_variable_write_node(self, node);
        }
        fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
            self.check_value(&node.value());
            ruby_prism::visit_constant_write_node(self, node);
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
        &[IF_NODE, CASE_NODE, CASE_MATCH_NODE, UNLESS_NODE]
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
            let condition = if_node.predicate();

            // Check tails (last statement in each branch)
            self.check_tails(source, &branches, Some(&condition), diagnostics);

            // Check heads (first statement in each branch)
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
            let condition = case_node.predicate();

            self.check_tails(source, &branches, condition.as_ref(), diagnostics);

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
            let condition = case_match_node.predicate();

            self.check_tails(source, &branches, condition.as_ref(), diagnostics);

            let last_child = is_last_child_of_parent(node, parse_result);
            self.check_heads(
                source,
                &branches,
                condition.as_ref(),
                last_child,
                diagnostics,
            );

            Self::dedup_diagnostics(diagnostics, pre_len);
        } else if let Some(unless_node) = node.as_unless_node() {
            // unless/else — must have an else clause for comparison
            let else_clause = match unless_node.else_clause() {
                Some(e) => e,
                None => return,
            };

            let branches = vec![
                BranchInfo::from_stmts(unless_node.statements()),
                BranchInfo::from_stmts(else_clause.statements()),
            ];

            let pre_len = diagnostics.len();
            let condition = unless_node.predicate();

            self.check_tails(source, &branches, Some(&condition), diagnostics);

            let last_child = is_last_child_of_parent(node, parse_result);
            self.check_heads(source, &branches, Some(&condition), last_child, diagnostics);

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
