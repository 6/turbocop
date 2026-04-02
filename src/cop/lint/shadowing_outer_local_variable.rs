use std::collections::HashSet;
use std::sync::Mutex;

use crate::cop::variable_force::{self, Variable, VariableTable};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for block parameters or block-local variables that shadow outer local variables.
///
/// ## Root causes of historical FP/FN (corpus conformance ~57%):
///
/// 1. **FP: Variable added to scope before RHS visited.** `visit_local_variable_write_node`
///    called `add_local` before visiting the value child. This caused `foo = bar { |foo| ... }`
///    to incorrectly flag `foo` as shadowing, because the LHS `foo` was already in scope when
///    the block was processed. RuboCop's VariableForce processes the RHS before declaring the
///    variable, so `foo` isn't in scope yet. Fix: visit the value first, then add to scope.
///
/// 2. **FN: Overly aggressive conditional suppression.** The `is_different_conditional_branch`
///    function had a `(None, Some(_)) => true` case that suppressed ALL shadowing when the
///    block was inside any conditional but the outer var was not. Per RuboCop, suppression
///    only applies when BOTH the outer var and the block are in different branches of the
///    SAME conditional node. Fix: remove the incorrect `(None, Some(_))` case.
///
/// ## Migration to VariableForce
///
/// This cop was migrated from a 1,857-line standalone AST visitor to use the shared
/// VariableForce engine. The cop uses `before_declaring_variable` to detect when a
/// block parameter shadows an outer local variable via `find_variable`. A lightweight
/// `check_source` pass pre-computes two things:
///
/// 1. **Ractor.new block offsets**: Ractor blocks have isolated scope by design;
///    shadowing inside them is intentional and not flagged.
///
/// 2. **Conditional branch context**: Maps byte offsets to their conditional branch
///    context (if/unless/case/when). Used to suppress shadowing when the outer
///    variable and block parameter are in different branches of the same conditional
///    (they can never both be in scope). This includes:
///    - Same-conditional different-branch suppression (Check 1)
///    - Adjacent elsif suppression (Check 2)
///    - Same-conditional-node condition-assignment suppression (Check 3)
///    - When-condition assignment suppression
///    - Inherited conditional context through single-statement block chains
///    - Expression depth tracking for nested-in-expression detection
pub struct ShadowingOuterLocalVariable {
    /// Byte offset ranges (start, end) of Ractor.new block bodies.
    /// Block params inside these are not flagged.
    ractor_block_ranges: Mutex<Vec<(usize, usize)>>,
    /// Sorted list of conditional branch intervals. Each entry covers a range
    /// of byte offsets and records the conditional context for that range.
    branch_intervals: Mutex<Vec<BranchInterval>>,
    /// Expression nesting ranges. Each entry is (start, end, depth) where depth
    /// is the expression nesting level when this range was entered. A block param
    /// is "nested in expression" relative to its branch if it's in an expression
    /// range whose depth exceeds the branch interval's expression_depth_base.
    expression_ranges: Mutex<Vec<(usize, usize, usize)>>,
    /// Offsets of block/lambda bodies that are single-statement.
    /// Used for inherited conditional context propagation.
    single_stmt_block_bodies: Mutex<HashSet<usize>>,
    /// Map from block body start offset to the inherited conditional branch context.
    /// Propagated through single-statement block body chains.
    inherited_cond_map: Mutex<Vec<InheritedCondEntry>>,
    /// Map from offset ranges to when_condition_of_case values.
    when_condition_ranges: Mutex<Vec<(usize, usize, usize)>>,
    /// Map from offset ranges to in_when_body_of_case values.
    when_body_ranges: Mutex<Vec<(usize, usize, usize)>>,
    /// Assignment LHS → RHS ranges. Each entry is (lhs_offset, rhs_start, rhs_end).
    /// Used to suppress shadowing when the block is in the RHS of the outer
    /// variable's own assignment (e.g., `foo = bar { |foo| baz(foo) }`).
    assignment_rhs_ranges: Mutex<Vec<(usize, usize, usize)>>,
    /// Block/lambda body ranges (block_node_start, body_start, body_end).
    /// Used to detect when a multi-statement block boundary separates the
    /// param from the conditional branch. block_node_start is used to look
    /// up whether the block is single-statement.
    block_body_ranges: Mutex<Vec<(usize, usize, usize)>>,
}

/// A conditional branch interval: all offsets in [start, end) have this context.
#[derive(Clone, Debug)]
struct BranchInterval {
    start: usize,
    end: usize,
    cond_offset: usize,
    branch_offset: usize,
    subsequent_offset: Option<usize>,
    is_body: bool,
    is_if_type: bool,
    single_stmt: bool,
    is_else_clause: bool,
    /// Expression depth base at the point this branch was entered.
    expression_depth_base: usize,
}

/// Inherited conditional context for a block body.
#[derive(Clone, Debug)]
struct InheritedCondEntry {
    /// Start offset of the block body.
    block_start: usize,
    /// End offset of the block body.
    block_end: usize,
    /// The inherited (cond_offset, branch_offset).
    cond_branch: (usize, usize),
    /// Whether the inherited context is from an if-type conditional.
    is_if_type: bool,
}

/// Info about where a variable was declared, used for suppression checks.
#[derive(Clone, Debug)]
struct VarBranchInfo {
    conditional_branch: Option<(usize, usize)>,
    cond_subsequent_offset: Option<usize>,
    when_condition_of_case: Option<usize>,
    is_condition_var: bool,
    is_if_type_cond: bool,
}

impl ShadowingOuterLocalVariable {
    pub fn new() -> Self {
        Self {
            ractor_block_ranges: Mutex::new(Vec::new()),
            branch_intervals: Mutex::new(Vec::new()),
            expression_ranges: Mutex::new(Vec::new()),
            single_stmt_block_bodies: Mutex::new(HashSet::new()),
            inherited_cond_map: Mutex::new(Vec::new()),
            when_condition_ranges: Mutex::new(Vec::new()),
            when_body_ranges: Mutex::new(Vec::new()),
            assignment_rhs_ranges: Mutex::new(Vec::new()),
            block_body_ranges: Mutex::new(Vec::new()),
        }
    }

    /// Look up the conditional branch context for a given byte offset.
    fn branch_info_at(&self, offset: usize) -> VarBranchInfo {
        let intervals = self.branch_intervals.lock().unwrap();
        // Find the innermost (last) interval containing this offset.
        let mut best: Option<&BranchInterval> = None;
        for interval in intervals.iter() {
            if interval.start <= offset && offset < interval.end {
                // Pick the innermost (narrowest) interval
                match best {
                    None => best = Some(interval),
                    Some(prev) => {
                        if interval.end - interval.start <= prev.end - prev.start {
                            best = Some(interval);
                        }
                    }
                }
            }
        }

        let when_cond_ranges = self.when_condition_ranges.lock().unwrap();
        let when_condition_of_case = when_cond_ranges
            .iter()
            .find(|(s, e, _)| *s <= offset && offset < *e)
            .map(|(_, _, case_off)| *case_off);

        match best {
            Some(interval) => VarBranchInfo {
                conditional_branch: Some((interval.cond_offset, interval.branch_offset)),
                cond_subsequent_offset: interval.subsequent_offset,
                when_condition_of_case,
                is_condition_var: !interval.is_body,
                is_if_type_cond: interval.is_if_type,
            },
            None => VarBranchInfo {
                conditional_branch: None,
                cond_subsequent_offset: None,
                when_condition_of_case,
                is_condition_var: false,
                is_if_type_cond: false,
            },
        }
    }

    /// Check if a given offset is inside an expression nesting relative to
    /// its enclosing branch interval. Returns true only if the expression
    /// nesting is deeper than the branch entry's expression_depth_base.
    fn is_in_expression_at(&self, offset: usize, branch_expr_depth_base: usize) -> bool {
        let ranges = self.expression_ranges.lock().unwrap();
        ranges
            .iter()
            .any(|(s, e, depth)| *s <= offset && offset < *e && *depth > branch_expr_depth_base)
    }

    /// Check if a given offset is inside a Ractor.new block.
    fn is_in_ractor_block(&self, offset: usize) -> bool {
        self.ractor_block_ranges
            .lock()
            .unwrap()
            .iter()
            .any(|(s, e)| *s <= offset && offset < *e)
    }

    /// Get the innermost branch interval for an offset.
    fn innermost_branch_at(&self, offset: usize) -> Option<BranchInterval> {
        let intervals = self.branch_intervals.lock().unwrap();
        let mut best: Option<&BranchInterval> = None;
        for interval in intervals.iter() {
            if interval.start <= offset && offset < interval.end {
                match best {
                    None => best = Some(interval),
                    Some(prev) => {
                        if interval.end - interval.start <= prev.end - prev.start {
                            best = Some(interval);
                        }
                    }
                }
            }
        }
        best.cloned()
    }

    /// Get the inherited conditional branch for an offset (from enclosing block bodies).
    fn inherited_cond_at(&self, offset: usize) -> Option<((usize, usize), bool)> {
        let map = self.inherited_cond_map.lock().unwrap();
        // Find the innermost block body containing this offset
        let mut best: Option<&InheritedCondEntry> = None;
        for entry in map.iter() {
            if entry.block_start <= offset && offset < entry.block_end {
                match best {
                    None => best = Some(entry),
                    Some(prev) => {
                        if entry.block_end - entry.block_start <= prev.block_end - prev.block_start
                        {
                            best = Some(entry);
                        }
                    }
                }
            }
        }
        best.map(|e| (e.cond_branch, e.is_if_type))
    }

    /// Check if an offset is in a when body of a particular case.
    fn in_when_body_of_case_at(&self, offset: usize) -> Option<usize> {
        let ranges = self.when_body_ranges.lock().unwrap();
        ranges
            .iter()
            .find(|(s, e, _)| *s <= offset && offset < *e)
            .map(|(_, _, case_off)| *case_off)
    }

    /// Check if there is a multi-statement block/lambda body boundary between
    /// the branch interval and the param offset. Single-statement blocks are
    /// transparent for suppression (matching RuboCop's behavior where
    /// `variable_node.parent` walks up through single-statement blocks).
    /// Multi-statement blocks truly nest the param, so suppression shouldn't apply.
    fn has_multi_stmt_block_boundary_between(
        &self,
        branch_start: usize,
        branch_end: usize,
        param_offset: usize,
    ) -> bool {
        let ranges = self.block_body_ranges.lock().unwrap();
        let single = self.single_stmt_block_bodies.lock().unwrap();
        ranges.iter().any(|(block_start, body_start, body_end)| {
            *body_start > branch_start
                && *body_end <= branch_end
                && *body_start <= param_offset
                && param_offset < *body_end
                && !single.contains(block_start)
        })
    }

    /// Check if a block param at `param_offset` is in the RHS of an assignment
    /// whose LHS is at `lhs_offset`. Used to suppress `foo = bar { |foo| }`.
    fn is_in_assignment_rhs(&self, lhs_offset: usize, param_offset: usize) -> bool {
        let ranges = self.assignment_rhs_ranges.lock().unwrap();
        ranges.iter().any(|(lhs, rhs_start, rhs_end)| {
            *lhs == lhs_offset && *rhs_start <= param_offset && param_offset < *rhs_end
        })
    }

    /// Check whether the block param should be suppressed due to conditional
    /// branch context.
    fn should_suppress(&self, outer_info: &VarBranchInfo, param_offset: usize) -> bool {
        let block_interval = self.innermost_branch_at(param_offset);

        let block_branch = block_interval
            .as_ref()
            .map(|i| (i.cond_offset, i.branch_offset));
        let block_is_in_body = block_interval.as_ref().is_some_and(|i| i.is_body);
        let block_single_stmt = block_interval.as_ref().is_some_and(|i| i.single_stmt);
        let is_in_else_clause = block_interval.as_ref().is_some_and(|i| i.is_else_clause);
        let expr_depth_base = block_interval
            .as_ref()
            .map_or(0, |i| i.expression_depth_base);
        let is_nested_in_expression = self.is_in_expression_at(param_offset, expr_depth_base);

        // If the param is inside a multi-statement block body that is nested
        // within the branch interval, the conditional suppression does not
        // apply — the block is truly nested, not a direct child of the branch.
        let has_block_boundary = block_interval.as_ref().is_some_and(|bi| {
            self.has_multi_stmt_block_boundary_between(bi.start, bi.end, param_offset)
        });

        // Check 1: same conditional, different branch
        if let Some(block_branch) = block_branch {
            if !is_nested_in_expression && !has_block_boundary {
                if let Some((outer_cond, outer_branch)) = outer_info.conditional_branch {
                    if outer_cond == block_branch.0 && outer_branch != block_branch.1 {
                        let should_suppress = if outer_info.is_if_type_cond {
                            is_in_else_clause || block_single_stmt
                        } else {
                            block_single_stmt
                        };
                        if should_suppress {
                            return true;
                        }
                    }
                }
            }
        }

        // Check 2: adjacent elsif suppression
        if let Some(block_branch) = block_branch {
            if !is_nested_in_expression
                && !has_block_boundary
                && block_single_stmt
                && (block_is_in_body || !outer_info.is_condition_var)
            {
                if let Some(subsequent_offset) = outer_info.cond_subsequent_offset {
                    if block_branch.0 == subsequent_offset {
                        return true;
                    }
                }
            }
        }

        // Check 3: same conditional node suppression (condition-assigned var)
        if let Some(block_branch) = block_branch {
            if outer_info.is_condition_var
                && block_is_in_body
                && block_single_stmt
                && !is_nested_in_expression
                && !has_block_boundary
            {
                if let Some((outer_cond, outer_branch)) = outer_info.conditional_branch {
                    if outer_cond == block_branch.0 && outer_branch == block_branch.1 {
                        return true;
                    }
                }
            }
        }

        // Check inherited conditional context (from enclosing blocks)
        if block_branch.is_none() {
            if let Some((inherited, is_if_type)) = self.inherited_cond_at(param_offset) {
                if let Some((outer_cond, outer_branch)) = outer_info.conditional_branch {
                    if outer_cond == inherited.0 && outer_branch != inherited.1 && is_if_type {
                        return true;
                    }
                }
            }
        }

        // Check when-condition assignment suppression
        if let (Some(var_case), Some(block_case)) = (
            outer_info.when_condition_of_case,
            self.in_when_body_of_case_at(param_offset),
        ) {
            if var_case == block_case {
                return true;
            }
        }

        false
    }
}

impl Default for ShadowingOuterLocalVariable {
    fn default() -> Self {
        Self::new()
    }
}

impl Cop for ShadowingOuterLocalVariable {
    fn name(&self) -> &'static str {
        "Lint/ShadowingOuterLocalVariable"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    /// This cop is disabled by default in RuboCop (Enabled: false).
    fn default_enabled(&self) -> bool {
        false
    }

    fn check_source(
        &self,
        _source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        _diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut collector = ContextCollector {
            ractor_block_ranges: Vec::new(),
            branch_intervals: Vec::new(),
            expression_ranges: Vec::new(),
            single_stmt_block_bodies: HashSet::new(),
            inherited_cond_map: Vec::new(),
            when_condition_ranges: Vec::new(),
            when_body_ranges: Vec::new(),
            assignment_rhs_ranges: Vec::new(),
            block_body_ranges: Vec::new(),
            conditional_branch_stack: Vec::new(),
            when_condition_case_offset: None,
            in_when_body_of_case: None,
            expression_depth: 0,
            inherited_cond_branch: None,
        };
        collector.visit(&parse_result.node());

        *self.ractor_block_ranges.lock().unwrap() = collector.ractor_block_ranges;
        *self.branch_intervals.lock().unwrap() = collector.branch_intervals;
        *self.expression_ranges.lock().unwrap() = collector.expression_ranges;
        *self.single_stmt_block_bodies.lock().unwrap() = collector.single_stmt_block_bodies;
        *self.inherited_cond_map.lock().unwrap() = collector.inherited_cond_map;
        *self.when_condition_ranges.lock().unwrap() = collector.when_condition_ranges;
        *self.when_body_ranges.lock().unwrap() = collector.when_body_ranges;
        *self.assignment_rhs_ranges.lock().unwrap() = collector.assignment_rhs_ranges;
        *self.block_body_ranges.lock().unwrap() = collector.block_body_ranges;
    }

    fn as_variable_force_consumer(&self) -> Option<&dyn variable_force::VariableForceConsumer> {
        Some(self)
    }
}

impl variable_force::VariableForceConsumer for ShadowingOuterLocalVariable {
    fn before_declaring_variable(
        &self,
        variable: &Variable,
        variable_table: &VariableTable,
        source: &SourceFile,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Only check block parameters and block-local variables (shadow args).
        // Method parameters (in def scopes) can't shadow — they're in a hard scope.
        if !variable.is_argument()
            && variable.declaration_kind != variable_force::DeclarationKind::ShadowArg
        {
            return;
        }

        let name = &variable.name;

        // Skip underscore-prefixed names
        if name.first() == Some(&b'_') {
            return;
        }

        // Check if there's an outer variable with the same name.
        // find_variable walks the scope stack respecting hard boundaries,
        // so it naturally handles def/class/module isolation.
        let outer = variable_table.find_variable(name);
        let Some(outer_var) = outer else {
            return;
        };

        let param_offset = variable.declaration_offset;
        let outer_offset = outer_var.declaration_offset;

        // Check if we're inside a Ractor.new block — shadowing is intentional
        if self.is_in_ractor_block(param_offset) {
            return;
        }

        // Look up the outer variable's conditional branch context
        let outer_info = self.branch_info_at(outer_offset);

        // Check if the block is in the RHS of the outer variable's assignment.
        // e.g., `foo = bar { |foo| baz(foo) }` — the block is the RHS of foo's
        // assignment, so foo is not yet semantically in scope (RuboCop suppresses).
        // However, do NOT suppress when the outer variable is in a conditional
        // branch body — a sibling branch may also declare the variable, and
        // RuboCop's VF visits branches in a different order (Parser gem order)
        // where the variable may already exist from a sibling branch.
        if self.is_in_assignment_rhs(outer_offset, param_offset) {
            let outer_in_branch_body = outer_info
                .conditional_branch
                .is_some_and(|_| !outer_info.is_condition_var);
            if !outer_in_branch_body {
                return;
            }
        }

        // Check conditional branch suppression
        if self.should_suppress(&outer_info, param_offset) {
            return;
        }

        // Adjust offset to include the sigil prefix for sigiled params.
        // RuboCop reports at the full parameter location (including `*`, `**`,
        // `&` sigils) for top-level block params, but at the name only for
        // params inside destructured multi-target (mlhs). The VF engine always
        // stores the name offset. We adjust only when the preceding bytes are
        // the expected sigil AND the param is not inside a destructured context
        // (no `(` between the enclosing `|` and the sigil).
        let src = source.as_bytes();
        let is_destructured = |offset: usize| -> bool {
            // Scan backward from just before the sigil to find `|` or `(`.
            // If we hit `(` before `|`, it's a destructured (mlhs) context.
            for i in (0..offset).rev() {
                match src.get(i) {
                    Some(b'(') => return true,
                    Some(b'|') => return false,
                    _ => {}
                }
            }
            false
        };
        let report_offset = match variable.declaration_kind {
            variable_force::DeclarationKind::RestArg
                if param_offset > 0
                    && src.get(param_offset - 1) == Some(&b'*')
                    && !is_destructured(param_offset - 1) =>
            {
                param_offset - 1
            }
            variable_force::DeclarationKind::KeywordRestArg
                if param_offset > 1
                    && src.get(param_offset - 2) == Some(&b'*')
                    && src.get(param_offset - 1) == Some(&b'*')
                    && !is_destructured(param_offset - 2) =>
            {
                param_offset - 2
            }
            variable_force::DeclarationKind::BlockArg
                if param_offset > 0
                    && src.get(param_offset - 1) == Some(&b'&')
                    && !is_destructured(param_offset - 1) =>
            {
                param_offset - 1
            }
            _ => param_offset,
        };
        let (line, column) = source.offset_to_line_col(report_offset);
        let display_name = String::from_utf8_lossy(name);
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Shadowing outer local variable - `{display_name}`."),
        ));
    }
}

// ── Context collector (pre-computation visitor) ───────────────────────

/// Entry in the conditional branch stack during context collection.
#[derive(Clone, Copy)]
struct CondBranchEntry {
    cond_offset: usize,
    branch_offset: usize,
    subsequent_offset: Option<usize>,
    is_body: bool,
    is_if_type: bool,
    single_stmt: bool,
    is_else_clause: bool,
    expression_depth_base: usize,
}

/// Lightweight AST visitor that pre-computes conditional branch context,
/// Ractor.new block ranges, and expression nesting for the VF hook to query.
struct ContextCollector {
    // Output data
    ractor_block_ranges: Vec<(usize, usize)>,
    branch_intervals: Vec<BranchInterval>,
    expression_ranges: Vec<(usize, usize, usize)>,
    single_stmt_block_bodies: HashSet<usize>,
    inherited_cond_map: Vec<InheritedCondEntry>,
    when_condition_ranges: Vec<(usize, usize, usize)>,
    when_body_ranges: Vec<(usize, usize, usize)>,
    assignment_rhs_ranges: Vec<(usize, usize, usize)>,
    block_body_ranges: Vec<(usize, usize, usize)>,

    // Tracking state
    conditional_branch_stack: Vec<CondBranchEntry>,
    when_condition_case_offset: Option<usize>,
    in_when_body_of_case: Option<usize>,
    expression_depth: usize,
    inherited_cond_branch: Option<((usize, usize), bool)>,
}

impl ContextCollector {
    fn push_branch(&mut self, entry: CondBranchEntry, start: usize, end: usize) {
        self.branch_intervals.push(BranchInterval {
            start,
            end,
            cond_offset: entry.cond_offset,
            branch_offset: entry.branch_offset,
            subsequent_offset: entry.subsequent_offset,
            is_body: entry.is_body,
            is_if_type: entry.is_if_type,
            single_stmt: entry.single_stmt,
            is_else_clause: entry.is_else_clause,
            expression_depth_base: entry.expression_depth_base,
        });
        self.conditional_branch_stack.push(entry);
    }

    fn pop_branch(&mut self) {
        self.conditional_branch_stack.pop();
    }

    fn current_cond_branch(&self) -> Option<(usize, usize)> {
        self.conditional_branch_stack
            .last()
            .map(|e| (e.cond_offset, e.branch_offset))
    }

    fn current_is_if_type(&self) -> bool {
        self.conditional_branch_stack
            .last()
            .is_some_and(|e| e.is_if_type)
    }

    /// Record that offsets in [start, end) are inside an expression nesting
    /// at the current expression depth.
    fn record_expression_range(&mut self, start: usize, end: usize) {
        self.expression_ranges
            .push((start, end, self.expression_depth));
    }

    fn visit_if_node_impl(&mut self, node: &ruby_prism::IfNode<'_>) {
        let if_offset = node.location().start_offset();
        let subsequent_offset = node.subsequent().map(|s| s.location().start_offset());

        let then_branch_offset = node
            .statements()
            .map(|s| s.location().start_offset())
            .unwrap_or(if_offset);

        let then_single_stmt = node.statements().is_none_or(|s| s.body().len() <= 1);

        // Visit predicate with then-body conditional context (is_body=false)
        let pred_start = node.predicate().location().start_offset();
        let pred_end = node.predicate().location().end_offset();
        let pred_entry = CondBranchEntry {
            cond_offset: if_offset,
            branch_offset: then_branch_offset,
            subsequent_offset,
            is_body: false,
            is_if_type: true,
            single_stmt: then_single_stmt,
            is_else_clause: false,
            expression_depth_base: self.expression_depth,
        };
        self.push_branch(pred_entry, pred_start, pred_end);
        self.visit(&node.predicate());
        self.pop_branch();

        // Visit then-body
        if let Some(stmts) = node.statements() {
            let body_start = stmts.location().start_offset();
            let body_end = stmts.location().end_offset();
            let body_entry = CondBranchEntry {
                cond_offset: if_offset,
                branch_offset: then_branch_offset,
                subsequent_offset,
                is_body: true,
                is_if_type: true,
                single_stmt: then_single_stmt,
                is_else_clause: false,
                expression_depth_base: self.expression_depth,
            };
            self.push_branch(body_entry, body_start, body_end);
            self.visit_statements_node(&stmts);
            self.pop_branch();
        }

        // Visit else/elsif
        if let Some(subsequent) = node.subsequent() {
            if let Some(elsif_node) = subsequent.as_if_node() {
                let branch_offset = subsequent.location().start_offset();
                let sub_start = subsequent.location().start_offset();
                let sub_end = subsequent.location().end_offset();
                let elsif_outer_entry = CondBranchEntry {
                    cond_offset: if_offset,
                    branch_offset,
                    subsequent_offset: None,
                    is_body: true,
                    is_if_type: true,
                    single_stmt: false,
                    is_else_clause: true,
                    expression_depth_base: self.expression_depth,
                };
                self.push_branch(elsif_outer_entry, sub_start, sub_end);
                self.visit_if_node_impl(&elsif_node);
                self.pop_branch();
            } else {
                let branch_offset = subsequent.location().start_offset();
                let else_single_stmt = subsequent
                    .as_else_node()
                    .and_then(|e| e.statements())
                    .is_none_or(|s| s.body().len() <= 1);
                let sub_start = subsequent.location().start_offset();
                let sub_end = subsequent.location().end_offset();
                let else_entry = CondBranchEntry {
                    cond_offset: if_offset,
                    branch_offset,
                    subsequent_offset: None,
                    is_body: true,
                    is_if_type: true,
                    single_stmt: else_single_stmt,
                    is_else_clause: true,
                    expression_depth_base: self.expression_depth,
                };
                self.push_branch(else_entry, sub_start, sub_end);
                self.visit(&subsequent);
                self.pop_branch();
            }
        }
    }

    fn visit_when_node_with_case_offset(
        &mut self,
        node: &ruby_prism::WhenNode<'_>,
        case_offset: usize,
    ) {
        // Visit when conditions
        let saved = self.when_condition_case_offset;
        self.when_condition_case_offset = Some(case_offset);
        let cond_offset = node.location().start_offset();

        // Record when condition range
        for condition in node.conditions().iter() {
            let start = condition.location().start_offset();
            let end = condition.location().end_offset();
            self.when_condition_ranges.push((start, end, case_offset));

            let cond_entry = CondBranchEntry {
                cond_offset: case_offset,
                branch_offset: cond_offset,
                subsequent_offset: None,
                is_body: false,
                is_if_type: false,
                single_stmt: false,
                is_else_clause: false,
                expression_depth_base: self.expression_depth,
            };
            self.push_branch(cond_entry, start, end);
            self.visit(&condition);
            self.pop_branch();
        }
        self.when_condition_case_offset = saved;

        // Visit when body
        if let Some(stmts) = node.statements() {
            let saved_body = self.in_when_body_of_case;
            self.in_when_body_of_case = Some(case_offset);
            let body_start = stmts.location().start_offset();
            let body_end = stmts.location().end_offset();
            self.when_body_ranges
                .push((body_start, body_end, case_offset));
            self.visit_statements_node(&stmts);
            self.in_when_body_of_case = saved_body;
        }
    }
}

impl<'pr> Visit<'pr> for ContextCollector {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Detect Ractor.new blocks
        if is_ractor_new_call(node) {
            if let Some(block) = node.block() {
                if let Some(block_node) = block.as_block_node() {
                    self.ractor_block_ranges.push((
                        block_node.location().start_offset(),
                        block_node.location().end_offset(),
                    ));
                }
            }
            // Visit receiver and arguments normally
            if let Some(receiver) = node.receiver() {
                self.visit(&receiver);
            }
            if let Some(arguments) = node.arguments() {
                self.visit_arguments_node(&arguments);
            }
            if let Some(block) = node.block() {
                if let Some(block_node) = block.as_block_node() {
                    ruby_prism::visit_block_node(self, &block_node);
                }
            }
            return;
        }

        // Visit receiver with expression depth
        if let Some(receiver) = node.receiver() {
            let start = receiver.location().start_offset();
            let end = receiver.location().end_offset();
            self.expression_depth += 1;
            self.record_expression_range(start, end);
            self.visit(&receiver);
            self.expression_depth -= 1;
        }
        if let Some(arguments) = node.arguments() {
            let start = arguments.location().start_offset();
            let end = arguments.location().end_offset();
            self.expression_depth += 1;
            self.record_expression_range(start, end);
            self.visit_arguments_node(&arguments);
            self.expression_depth -= 1;
        }
        if let Some(block) = node.block() {
            self.visit(&block);
        }
    }

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        let lhs_offset = node.location().start_offset();
        let start = node.value().location().start_offset();
        let end = node.value().location().end_offset();
        self.assignment_rhs_ranges.push((lhs_offset, start, end));
        self.expression_depth += 1;
        self.record_expression_range(start, end);
        self.visit(&node.value());
        self.expression_depth -= 1;
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        let start = node.value().location().start_offset();
        let end = node.value().location().end_offset();
        self.expression_depth += 1;
        self.record_expression_range(start, end);
        self.visit(&node.value());
        self.expression_depth -= 1;
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        let start = node.value().location().start_offset();
        let end = node.value().location().end_offset();
        self.expression_depth += 1;
        self.record_expression_range(start, end);
        self.visit(&node.value());
        self.expression_depth -= 1;
    }

    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        let start = node.value().location().start_offset();
        let end = node.value().location().end_offset();
        self.expression_depth += 1;
        self.record_expression_range(start, end);
        self.visit(&node.value());
        self.expression_depth -= 1;
    }

    fn visit_multi_write_node(&mut self, node: &ruby_prism::MultiWriteNode<'pr>) {
        let rhs_start = node.value().location().start_offset();
        let rhs_end = node.value().location().end_offset();
        // Record each LHS target's offset as mapping to the RHS range
        for target in node.lefts().iter() {
            if let Some(t) = target.as_local_variable_target_node() {
                self.assignment_rhs_ranges
                    .push((t.location().start_offset(), rhs_start, rhs_end));
            }
        }
        if let Some(rest) = node.rest() {
            if let Some(splat) = rest.as_splat_node() {
                if let Some(expr) = splat.expression() {
                    if let Some(t) = expr.as_local_variable_target_node() {
                        self.assignment_rhs_ranges.push((
                            t.location().start_offset(),
                            rhs_start,
                            rhs_end,
                        ));
                    }
                }
            }
        }
        for target in node.rights().iter() {
            if let Some(t) = target.as_local_variable_target_node() {
                self.assignment_rhs_ranges
                    .push((t.location().start_offset(), rhs_start, rhs_end));
            }
        }
        self.expression_depth += 1;
        self.record_expression_range(rhs_start, rhs_end);
        self.visit(&node.value());
        self.expression_depth -= 1;
        // Visit targets (but don't add expression depth)
        for target in node.lefts().iter() {
            self.visit(&target);
        }
        if let Some(rest) = node.rest() {
            self.visit(&rest);
        }
        for target in node.rights().iter() {
            self.visit(&target);
        }
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let block_body_single_stmt = node
            .body()
            .and_then(|body| body.as_statements_node())
            .is_none_or(|body| body.body().len() <= 1);

        if block_body_single_stmt {
            self.single_stmt_block_bodies
                .insert(node.location().start_offset());
        }

        // Compute inherited conditional context for inner blocks
        let current_cond = self.current_cond_branch();
        let current_if_type = self.current_is_if_type();
        let saved_inherited = self.inherited_cond_branch;
        let new_inherited = if block_body_single_stmt {
            current_cond
                .map(|cb| (cb, current_if_type))
                .or(self.inherited_cond_branch)
        } else {
            None
        };
        self.inherited_cond_branch = new_inherited;

        // Record inherited conditional context for the block body
        if let Some((cond_branch, is_if_type)) = new_inherited {
            if let Some(body) = node.body() {
                self.inherited_cond_map.push(InheritedCondEntry {
                    block_start: body.location().start_offset(),
                    block_end: body.location().end_offset(),
                    cond_branch,
                    is_if_type,
                });
            }
        }

        // Record block body range for block-boundary checks
        if let Some(body) = node.body() {
            self.block_body_ranges.push((
                node.location().start_offset(),
                body.location().start_offset(),
                body.location().end_offset(),
            ));
        }

        // Clear conditional branch stack for block body
        let saved_cond_stack = std::mem::take(&mut self.conditional_branch_stack);
        let saved_when_body = self.in_when_body_of_case.take();
        ruby_prism::visit_block_node(self, node);
        self.conditional_branch_stack = saved_cond_stack;
        self.in_when_body_of_case = saved_when_body;
        self.inherited_cond_branch = saved_inherited;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        let lambda_body_single_stmt = node
            .body()
            .and_then(|body| body.as_statements_node())
            .is_none_or(|body| body.body().len() <= 1);

        if lambda_body_single_stmt {
            self.single_stmt_block_bodies
                .insert(node.location().start_offset());
        }

        let current_cond = self.current_cond_branch();
        let current_if_type = self.current_is_if_type();
        let saved_inherited = self.inherited_cond_branch;
        let new_inherited = if lambda_body_single_stmt {
            current_cond
                .map(|cb| (cb, current_if_type))
                .or(self.inherited_cond_branch)
        } else {
            None
        };
        self.inherited_cond_branch = new_inherited;

        if let Some((cond_branch, is_if_type)) = new_inherited {
            if let Some(body) = node.body() {
                self.inherited_cond_map.push(InheritedCondEntry {
                    block_start: body.location().start_offset(),
                    block_end: body.location().end_offset(),
                    cond_branch,
                    is_if_type,
                });
            }
        }

        // Record lambda body range for block-boundary checks
        if let Some(body) = node.body() {
            self.block_body_ranges.push((
                node.location().start_offset(),
                body.location().start_offset(),
                body.location().end_offset(),
            ));
        }

        let saved_cond_stack = std::mem::take(&mut self.conditional_branch_stack);
        let saved_when_body = self.in_when_body_of_case.take();
        ruby_prism::visit_lambda_node(self, node);
        self.conditional_branch_stack = saved_cond_stack;
        self.in_when_body_of_case = saved_when_body;
        self.inherited_cond_branch = saved_inherited;
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        self.visit_if_node_impl(node);
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        let unless_offset = node.location().start_offset();
        let body_offset = node.statements().map(|s| s.location().start_offset());

        let body_single_stmt = node.statements().is_none_or(|s| s.body().len() <= 1);

        // Visit predicate normally
        self.visit(&node.predicate());

        // Visit else clause FIRST (Parser gem's then-body).
        if let Some(else_clause) = node.else_clause() {
            let branch_offset = else_clause.location().start_offset();
            let else_start = else_clause.location().start_offset();
            let else_end = else_clause.location().end_offset();
            let else_single_stmt = else_clause.statements().is_none_or(|s| s.body().len() <= 1);
            let else_entry = CondBranchEntry {
                cond_offset: unless_offset,
                branch_offset,
                subsequent_offset: body_offset,
                is_body: true,
                is_if_type: true,
                single_stmt: else_single_stmt,
                is_else_clause: false,
                expression_depth_base: self.expression_depth,
            };
            self.push_branch(else_entry, else_start, else_end);
            self.visit_else_node(&else_clause);
            self.pop_branch();
        }

        // Visit body SECOND (Parser gem's else).
        if let Some(stmts) = node.statements() {
            let branch_offset = stmts.location().start_offset();
            let body_start = stmts.location().start_offset();
            let body_end = stmts.location().end_offset();
            let body_entry = CondBranchEntry {
                cond_offset: unless_offset,
                branch_offset,
                subsequent_offset: None,
                is_body: true,
                is_if_type: true,
                single_stmt: body_single_stmt,
                is_else_clause: true,
                expression_depth_base: self.expression_depth,
            };
            self.push_branch(body_entry, body_start, body_end);
            self.visit_statements_node(&stmts);
            self.pop_branch();
        }
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        let while_offset = node.location().start_offset();
        let pred_offset = node.predicate().location().start_offset();
        let start = node.location().start_offset();
        let end = node.location().end_offset();
        let entry = CondBranchEntry {
            cond_offset: while_offset,
            branch_offset: pred_offset,
            subsequent_offset: None,
            is_body: true,
            is_if_type: false,
            single_stmt: false,
            is_else_clause: false,
            expression_depth_base: self.expression_depth,
        };
        self.push_branch(entry, start, end);
        ruby_prism::visit_while_node(self, node);
        self.pop_branch();
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        let until_offset = node.location().start_offset();
        let pred_offset = node.predicate().location().start_offset();
        let start = node.location().start_offset();
        let end = node.location().end_offset();
        let entry = CondBranchEntry {
            cond_offset: until_offset,
            branch_offset: pred_offset,
            subsequent_offset: None,
            is_body: true,
            is_if_type: false,
            single_stmt: false,
            is_else_clause: false,
            expression_depth_base: self.expression_depth,
        };
        self.push_branch(entry, start, end);
        ruby_prism::visit_until_node(self, node);
        self.pop_branch();
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        let case_offset = node.location().start_offset();

        // Visit predicate
        if let Some(pred) = node.predicate() {
            let pred_start = pred.location().start_offset();
            let pred_end = pred.location().end_offset();
            let pred_entry = CondBranchEntry {
                cond_offset: case_offset,
                branch_offset: pred_start,
                subsequent_offset: None,
                is_body: false,
                is_if_type: false,
                single_stmt: true,
                is_else_clause: false,
                expression_depth_base: self.expression_depth,
            };
            self.push_branch(pred_entry, pred_start, pred_end);
            self.visit(&pred);
            self.pop_branch();
        }

        // Visit each when clause
        for condition in node.conditions().iter() {
            let branch_offset = condition.location().start_offset();
            let when_start = condition.location().start_offset();
            let when_end = condition.location().end_offset();
            let when_single_stmt = condition
                .as_when_node()
                .and_then(|w| w.statements())
                .is_none_or(|s| s.body().len() <= 1);
            let when_entry = CondBranchEntry {
                cond_offset: case_offset,
                branch_offset,
                subsequent_offset: None,
                is_body: true,
                is_if_type: false,
                single_stmt: when_single_stmt,
                is_else_clause: false,
                expression_depth_base: self.expression_depth,
            };
            self.push_branch(when_entry, when_start, when_end);
            if let Some(when_node) = condition.as_when_node() {
                self.visit_when_node_with_case_offset(&when_node, case_offset);
            } else {
                self.visit(&condition);
            }
            self.pop_branch();
        }

        // Visit else clause
        if let Some(else_clause) = node.else_clause() {
            let branch_offset = else_clause.location().start_offset();
            let else_start = else_clause.location().start_offset();
            let else_end = else_clause.location().end_offset();
            let else_single_stmt = else_clause.statements().is_none_or(|s| s.body().len() <= 1);
            let else_entry = CondBranchEntry {
                cond_offset: case_offset,
                branch_offset,
                subsequent_offset: None,
                is_body: true,
                is_if_type: false,
                single_stmt: else_single_stmt,
                is_else_clause: true,
                expression_depth_base: self.expression_depth,
            };
            self.push_branch(else_entry, else_start, else_end);
            self.visit_else_node(&else_clause);
            self.pop_branch();
        }
    }

    // Don't need to override def/class/module — the context collector
    // only cares about conditional branches, not scope management.
    // VF handles scopes. But we DO need to enter them to find conditionals
    // inside method bodies.
}

/// Check if a CallNode is `Ractor.new(...)` or `::Ractor.new(...)`.
fn is_ractor_new_call(node: &ruby_prism::CallNode<'_>) -> bool {
    let name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
    if name != "new" {
        return false;
    }
    if let Some(receiver) = node.receiver() {
        if let Some(constant) = receiver.as_constant_read_node() {
            let const_name = std::str::from_utf8(constant.name().as_slice()).unwrap_or("");
            return const_name == "Ractor";
        }
        if let Some(path) = receiver.as_constant_path_node() {
            if let Some(child) = path.name() {
                let const_name = std::str::from_utf8(child.as_slice()).unwrap_or("");
                return const_name == "Ractor";
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ShadowingOuterLocalVariable::new(),
        "cops/lint/shadowing_outer_local_variable"
    );
}
