use std::collections::{HashMap, HashSet};

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for every useless assignment to local variable in every scope.
///
/// ## Implementation approach
///
/// RuboCop uses VariableForce, a sophisticated control-flow-aware analysis that
/// tracks individual assignments through branches, loops, rescue, etc. Our
/// implementation approximates this with a per-scope sequential analysis that:
///
/// 1. Walks statements sequentially, tracking the "last write" for each variable
/// 2. When a variable is read, marks the last write as "used"
/// 3. When a variable is written, the previous write (if not yet used) is "useless"
/// 4. For branches (if/else, case/when), merges liveness conservatively
/// 5. For loops, assumes writes may be read in the next iteration
/// 6. For blocks (closures), treats them as may-execute
///
/// ## Root causes of historical FP/FN
///
/// - FN (3,565 in corpus): The old implementation used flat variable-name-level
///   tracking ("is variable X ever read?") instead of per-assignment tracking.
///   This missed the common pattern `x = 1; puts x; x = 2` where the last
///   assignment is useless, and `x = 1; x = 2; puts x` where the first is useless.
/// - FN: Top-level scope was not analyzed at all.
/// - FN: For-loop variables were not checked.
/// - FP (503 in corpus): Various control-flow patterns where conservative
///   analysis incorrectly flagged assignments used through branches/loops.
///
/// ## Fixes applied
///
/// - FP fix: `begin/rescue` and `begin/ensure` blocks now protect pre-begin
///   writes from being marked useless during body analysis. Any statement in
///   the body could raise, so the pre-begin value may still be live when
///   control reaches the rescue/ensure or post-begin code. This fixed 270+
///   FPs from the `result = nil; begin; result = expr; rescue; end; result`
///   pattern common in Rails and other frameworks.
/// - FP fix: `class << expr` now correctly analyzes the expression as a read.
///   Previously `obj = Object.new; class << obj; end` flagged `obj` as useless
///   because the singleton class expression was not traversed for variable reads.
/// - FP fix: Rescue clauses now see writes from inside the begin body, not just
///   pre-begin writes. Previously `counter = 0; begin; counter += 1; rescue;
///   retry if counter < 3; end` flagged `counter += 1` as useless because the
///   rescue clause started from pre-body state and couldn't see the body write.
///   This fixes ~180-240 FPs from retry counters, boolean flags, and timing
///   variables written in begin bodies and read in rescue clauses.
///
/// ## Remaining gaps (758 FP, 1366 FN as of corpus investigation)
///
/// - FP: Multi-write targets (`a, b = expr`) where one target is unused —
///   RuboCop's VariableForce has more nuanced handling of multi-writes.
/// - FP: Complex control flow in loop bodies with if/elsif branches that
///   reset variables — the 2-pass simulation doesn't fully capture all
///   back-edge read patterns.
/// - FP: `rescue => e` where `e` is unused — some RuboCop configurations
///   suppress this detection.
/// - FN: Various patterns requiring deeper flow analysis that our sequential
///   approximation misses.
pub struct UselessAssignment;

impl Cop for UselessAssignment {
    fn name(&self) -> &'static str {
        "Lint/UselessAssignment"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = UselessAssignVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            inside_def: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct UselessAssignVisitor<'a, 'src> {
    cop: &'a UselessAssignment,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// True when inside a def node. Blocks inside defs are analyzed as closures
    /// by the def's ScopeAnalyzer, so they don't need separate analysis.
    inside_def: bool,
}

// ── Liveness tracking ───────────────────────────────────────────────────────

/// Tracks per-assignment liveness within a scope.
///
/// `live_writes` maps variable name -> offset of the last write that hasn't
/// been "used" (read) yet. When we see a read, we remove the entry (marking
/// the write as used). When we see a write and there's already an entry, the
/// previous write is useless.
#[derive(Clone, Default)]
struct LiveState {
    /// variable name -> offset of last unread write
    live_writes: HashMap<String, usize>,
}

impl LiveState {
    fn new() -> Self {
        Self::default()
    }

    /// Record a write. Returns the previous unread write offset if any (useless).
    fn record_write(&mut self, name: &str, offset: usize) -> Option<usize> {
        self.live_writes.insert(name.to_string(), offset)
    }

    /// Record a read. Marks the current write as used.
    /// Returns the offset of the consumed write, if any.
    fn record_read(&mut self, name: &str) -> Option<usize> {
        self.live_writes.remove(name)
    }

    /// Get all remaining unread writes (useless assignments at end of scope).
    fn unread_writes(&self) -> impl Iterator<Item = (&str, usize)> {
        self.live_writes.iter().map(|(k, &v)| (k.as_str(), v))
    }

    /// Merge two states from branches (if/else). A write is "live" (not useless)
    /// if it's live in EITHER branch (conservative — the branch may or may not
    /// execute).
    fn merge_branches(a: &LiveState, b: &LiveState) -> LiveState {
        let mut merged = LiveState::new();
        for (name, &offset) in &a.live_writes {
            merged.live_writes.insert(name.clone(), offset);
        }
        for (name, &offset) in &b.live_writes {
            // If both branches have a write, keep the one from branch b
            // (arbitrary, both are unread). If only one has it, keep it.
            merged.live_writes.insert(name.clone(), offset);
        }
        merged
    }

    /// Merge state from a single branch (if without else, or a block that
    /// may not execute). The branch state is merged with the pre-branch state
    /// conservatively: a write from before the branch is NOT useless if the
    /// branch may not execute (the value could still be read after).
    fn merge_optional_branch(before: &LiveState, branch: &LiveState) -> LiveState {
        let mut merged = LiveState::new();
        // Keep writes from before the branch that are still live
        for (name, &offset) in &before.live_writes {
            merged.live_writes.insert(name.clone(), offset);
        }
        // Also keep writes from inside the branch — but DON'T override
        // the "before" state because the branch may not have executed.
        // Actually, we SHOULD override: if the branch wrote to the same
        // variable, that write also needs to be checked. But since the
        // branch may not execute, the pre-branch write could still be
        // the one that's live. We keep the pre-branch write.
        // Only add NEW variables from the branch.
        for (name, &offset) in &branch.live_writes {
            if !merged.live_writes.contains_key(name) {
                merged.live_writes.insert(name.clone(), offset);
            }
        }
        merged
    }
}

/// Information about a useless write detected during analysis.
struct UselessWrite {
    name: String,
    offset: usize,
}

/// Context for analyzing a scope (def body, block body, or top-level).
struct ScopeAnalyzer {
    /// Useless writes found during analysis.
    useless: Vec<UselessWrite>,
    /// Whether a `binding` call was found (suppresses all reports).
    has_binding: bool,
    /// Whether a bare `super` (forwarding) was found.
    has_forwarding_super: bool,
    /// Variables that are read in ANY block/closure within this scope.
    /// These reads could happen at any time (blocks are closures that can be
    /// stored and called later), so writes to these variables are always "used."
    closure_reads: HashSet<String>,
    /// Variables that are written in any block/closure within this scope.
    /// A closure write means the outer write may still be live (the closure
    /// may not execute).
    closure_writes: HashSet<String>,
    /// Set of write offsets from before a branch. When a write inside a branch
    /// overrides a pre-branch write, the pre-branch write is NOT useless (the
    /// branch may not execute). These offsets are "protected" from being marked
    /// as useless.
    protected_offsets: HashSet<usize>,
    /// Set of (variable name, write offset) pairs that were ever "consumed" by
    /// a read anywhere in this scope — including inside branches and loops that
    /// may not execute. Used at end-of-scope to suppress FPs: if a specific
    /// write was consumed by any reachable read path, it is not useless even if
    /// the merge logic re-inserts it into `live_writes`.
    ever_read_offsets: HashSet<usize>,
    /// Extra unread writes from branch merges where both branches wrote to the
    /// same variable. Since `live_writes` can only hold one offset per variable,
    /// the "other" branch's offset is stored here. At end-of-scope, these are
    /// reported as useless if the variable was not read after the branch.
    branch_extra_writes: Vec<(String, usize)>,
}

impl ScopeAnalyzer {
    fn new() -> Self {
        Self {
            useless: Vec::new(),
            has_binding: false,
            has_forwarding_super: false,
            closure_reads: HashSet::new(),
            closure_writes: HashSet::new(),
            protected_offsets: HashSet::new(),
            ever_read_offsets: HashSet::new(),
            branch_extra_writes: Vec::new(),
        }
    }

    /// Add all current live write offsets as "protected" — writes from before
    /// a branch that should not be marked useless if overwritten in the branch.
    /// Returns the previous protected_offsets for restoration.
    fn protect_live_writes(&mut self, state: &LiveState) -> HashSet<usize> {
        let prev = self.protected_offsets.clone();
        for &offset in state.live_writes.values() {
            self.protected_offsets.insert(offset);
        }
        prev
    }

    /// Restore protected offsets to a previous state.
    fn restore_protected(&mut self, prev: HashSet<usize>) {
        self.protected_offsets = prev;
    }

    /// Record a read and track which write offset was consumed.
    fn record_read(&mut self, state: &mut LiveState, name: &str) {
        if let Some(offset) = state.record_read(name) {
            self.ever_read_offsets.insert(offset);
        }
        // Also clear any branch_extra_writes for this variable, since the
        // variable is being read (meaning the branch writes were not useless).
        self.branch_extra_writes.retain(|(n, _)| n != name);
    }

    /// Record a write and mark the previous write as useless if appropriate.
    fn record_write_and_check(&mut self, state: &mut LiveState, name: &str, offset: usize) {
        if let Some(prev_offset) = state.record_write(name, offset) {
            if !name.starts_with('_') && !self.protected_offsets.contains(&prev_offset) {
                self.useless.push(UselessWrite {
                    name: name.to_string(),
                    offset: prev_offset,
                });
            }
        }
    }

    /// Analyze a sequence of statements for useless assignments.
    fn analyze_statements(&mut self, stmts: &[ruby_prism::Node<'_>], state: &mut LiveState) {
        for stmt in stmts {
            self.analyze_node(stmt, state);
        }
    }

    /// Analyze a single node, updating the live state.
    fn analyze_node(&mut self, node: &ruby_prism::Node<'_>, state: &mut LiveState) {
        // Local variable write: x = expr
        if let Some(write_node) = node.as_local_variable_write_node() {
            // First analyze the value (RHS) — it may read variables
            self.analyze_node(&write_node.value(), state);
            let name = node_name(write_node.name().as_slice());
            let offset = write_node.name_loc().start_offset();
            self.record_write_and_check(state, &name, offset);
            return;
        }

        // Local variable read
        if let Some(read_node) = node.as_local_variable_read_node() {
            let name = node_name(read_node.name().as_slice());
            self.record_read(state, &name);
            return;
        }

        // Compound assignment (+=, -=, etc.) — reads then writes
        if let Some(op_node) = node.as_local_variable_operator_write_node() {
            self.analyze_node(&op_node.value(), state);
            let name = node_name(op_node.name().as_slice());
            self.record_read(state, &name); // reads first
            // It also writes, but compound assignment is both read+write,
            // so the new value replaces. We record it as a write.
            state.record_write(&name, op_node.name_loc().start_offset());
            return;
        }

        // ||= reads then conditionally writes
        if let Some(or_node) = node.as_local_variable_or_write_node() {
            self.analyze_node(&or_node.value(), state);
            let name = node_name(or_node.name().as_slice());
            self.record_read(state, &name);
            // It may or may not write — conservative: record as write
            state.record_write(&name, or_node.name_loc().start_offset());
            return;
        }

        // &&= reads then conditionally writes
        if let Some(and_node) = node.as_local_variable_and_write_node() {
            self.analyze_node(&and_node.value(), state);
            let name = node_name(and_node.name().as_slice());
            self.record_read(state, &name);
            state.record_write(&name, and_node.name_loc().start_offset());
            return;
        }

        // Multi-write (a, b = expr)
        if let Some(multi) = node.as_multi_write_node() {
            // Analyze RHS first
            self.analyze_node(&multi.value(), state);
            // Process targets — each target is a write
            for target in multi.lefts().iter() {
                self.analyze_multi_target(&target, state);
            }
            if let Some(rest) = multi.rest() {
                if let Some(splat) = rest.as_splat_node() {
                    if let Some(expr) = splat.expression() {
                        self.analyze_multi_target(&expr, state);
                    }
                }
            }
            for target in multi.rights().iter() {
                self.analyze_multi_target(&target, state);
            }
            return;
        }

        // If/unless — analyze branches
        if let Some(if_node) = node.as_if_node() {
            self.analyze_if(&if_node, state);
            return;
        }

        // Unless
        if let Some(unless_node) = node.as_unless_node() {
            self.analyze_unless(&unless_node, state);
            return;
        }

        // Case/when
        if let Some(case_node) = node.as_case_node() {
            self.analyze_case(&case_node, state);
            return;
        }

        // Case/in (pattern matching)
        if let Some(case_match) = node.as_case_match_node() {
            self.analyze_case_match(&case_match, state);
            return;
        }

        // While loop
        if let Some(while_node) = node.as_while_node() {
            self.analyze_while(&while_node, state);
            return;
        }

        // Until loop
        if let Some(until_node) = node.as_until_node() {
            self.analyze_until(&until_node, state);
            return;
        }

        // For loop
        if let Some(for_node) = node.as_for_node() {
            self.analyze_for(&for_node, state);
            return;
        }

        // Begin/rescue/ensure
        if let Some(begin_node) = node.as_begin_node() {
            self.analyze_begin(&begin_node, state);
            return;
        }

        // Rescue modifier (expr rescue fallback)
        if let Some(rescue_mod) = node.as_rescue_modifier_node() {
            self.analyze_node(&rescue_mod.expression(), state);
            let before = state.clone();
            let saved_protected = self.protect_live_writes(&before);
            self.analyze_node(&rescue_mod.rescue_expression(), state);
            *state = LiveState::merge_optional_branch(&before, state);
            self.restore_protected(saved_protected);
            return;
        }

        // Def node — new hard scope, analyze separately
        if let Some(def_node) = node.as_def_node() {
            // The receiver of a singleton def (def var.method) reads the variable
            if let Some(receiver) = def_node.receiver() {
                self.analyze_node(&receiver, state);
            }
            // Don't recurse into the body — it's a separate scope
            return;
        }

        // Class/module — new hard scope
        if node.as_class_node().is_some()
            || node.as_module_node().is_some()
            || node.as_singleton_class_node().is_some()
        {
            // Don't recurse — new scope. But class < expr reads the superclass expr.
            if let Some(class_node) = node.as_class_node() {
                if let Some(superclass) = class_node.superclass() {
                    self.analyze_node(&superclass, state);
                }
            }
            // class << expr reads the expression (e.g., `class << obj` reads `obj`)
            if let Some(sclass_node) = node.as_singleton_class_node() {
                self.analyze_node(&sclass_node.expression(), state);
            }
            return;
        }

        // Block node (closure) — special handling
        // Note: BlockNode is always a child of CallNode. The call's receiver/args
        // are handled by the CallNode case below. Here we just handle the block body.
        if let Some(block_node) = node.as_block_node() {
            // The block body is a closure — it may read/write outer variables.
            // Collect reads and writes from the block body.
            if let Some(body) = block_node.body() {
                let mut block_collector = ClosureCollector::new();
                block_collector.visit(&body);
                // Any variable read in the closure means the current write is "used"
                for read_name in &block_collector.reads {
                    self.record_read(state, read_name);
                    self.closure_reads.insert(read_name.clone());
                }
                for write_name in &block_collector.writes {
                    self.closure_writes.insert(write_name.clone());
                }
                if block_collector.has_binding {
                    self.has_binding = true;
                }
            }
            return;
        }

        // Lambda node — also a closure
        if let Some(lambda_node) = node.as_lambda_node() {
            if let Some(body) = lambda_node.body() {
                let mut block_collector = ClosureCollector::new();
                block_collector.visit(&body);
                for read_name in &block_collector.reads {
                    self.record_read(state, read_name);
                    self.closure_reads.insert(read_name.clone());
                }
                for write_name in &block_collector.writes {
                    self.closure_writes.insert(write_name.clone());
                }
                if block_collector.has_binding {
                    self.has_binding = true;
                }
            }
            return;
        }

        // Call node — check for binding, then analyze children
        if let Some(call_node) = node.as_call_node() {
            if call_node.receiver().is_none()
                && call_node.name().as_slice() == b"binding"
                && call_node
                    .arguments()
                    .is_none_or(|a| a.arguments().is_empty())
            {
                self.has_binding = true;
            }
            // Analyze receiver, arguments, etc.
            if let Some(recv) = call_node.receiver() {
                self.analyze_node(&recv, state);
            }
            if let Some(args) = call_node.arguments() {
                for arg in args.arguments().iter() {
                    self.analyze_node(&arg, state);
                }
            }
            // Block is handled separately via block_node
            if let Some(block) = call_node.block() {
                self.analyze_node(&block, state);
            }
            return;
        }

        // Forwarding super
        if node.as_forwarding_super_node().is_some() {
            self.has_forwarding_super = true;
            return;
        }

        // Super with args
        if let Some(super_node) = node.as_super_node() {
            if let Some(args) = super_node.arguments() {
                for arg in args.arguments().iter() {
                    self.analyze_node(&arg, state);
                }
            }
            if let Some(block) = super_node.block() {
                self.analyze_node(&block, state);
            }
            return;
        }

        // Yield
        if let Some(yield_node) = node.as_yield_node() {
            if let Some(args) = yield_node.arguments() {
                for arg in args.arguments().iter() {
                    self.analyze_node(&arg, state);
                }
            }
            return;
        }

        // Return
        if let Some(ret_node) = node.as_return_node() {
            if let Some(args) = ret_node.arguments() {
                for arg in args.arguments().iter() {
                    self.analyze_node(&arg, state);
                }
            }
            return;
        }

        // Statements node (sequence of statements)
        if let Some(stmts_node) = node.as_statements_node() {
            let stmts: Vec<_> = stmts_node.body().iter().collect();
            self.analyze_statements(&stmts, state);
            return;
        }

        // Parentheses
        if let Some(parens) = node.as_parentheses_node() {
            if let Some(body) = parens.body() {
                self.analyze_node(&body, state);
            }
            return;
        }

        // And/or (short-circuit) — RHS may not execute
        if let Some(and_node) = node.as_and_node() {
            self.analyze_node(&and_node.left(), state);
            let before = state.clone();
            let saved_protected = self.protect_live_writes(&before);
            self.analyze_node(&and_node.right(), state);
            *state = LiveState::merge_optional_branch(&before, state);
            self.restore_protected(saved_protected);
            return;
        }
        if let Some(or_node) = node.as_or_node() {
            self.analyze_node(&or_node.left(), state);
            let before = state.clone();
            let saved_protected = self.protect_live_writes(&before);
            self.analyze_node(&or_node.right(), state);
            *state = LiveState::merge_optional_branch(&before, state);
            self.restore_protected(saved_protected);
            return;
        }

        // Ternary (if_node covers this via as_if_node already)

        // Array/hash literals — analyze elements
        if let Some(array_node) = node.as_array_node() {
            for elem in array_node.elements().iter() {
                self.analyze_node(&elem, state);
            }
            return;
        }
        if let Some(hash_node) = node.as_hash_node() {
            for elem in hash_node.elements().iter() {
                self.analyze_node(&elem, state);
            }
            return;
        }
        if let Some(kw_hash) = node.as_keyword_hash_node() {
            for elem in kw_hash.elements().iter() {
                self.analyze_node(&elem, state);
            }
            return;
        }
        if let Some(assoc) = node.as_assoc_node() {
            self.analyze_node(&assoc.key(), state);
            self.analyze_node(&assoc.value(), state);
            return;
        }
        if let Some(splat) = node.as_assoc_splat_node() {
            if let Some(val) = splat.value() {
                self.analyze_node(&val, state);
            }
            return;
        }
        if let Some(splat) = node.as_splat_node() {
            if let Some(expr) = splat.expression() {
                self.analyze_node(&expr, state);
            }
            return;
        }

        // Range
        if let Some(range) = node.as_range_node() {
            if let Some(left) = range.left() {
                self.analyze_node(&left, state);
            }
            if let Some(right) = range.right() {
                self.analyze_node(&right, state);
            }
            return;
        }

        // String interpolation
        if let Some(interp) = node.as_interpolated_string_node() {
            for part in interp.parts().iter() {
                self.analyze_node(&part, state);
            }
            return;
        }
        if let Some(interp) = node.as_interpolated_symbol_node() {
            for part in interp.parts().iter() {
                self.analyze_node(&part, state);
            }
            return;
        }
        if let Some(interp) = node.as_interpolated_x_string_node() {
            for part in interp.parts().iter() {
                self.analyze_node(&part, state);
            }
            return;
        }
        if let Some(interp) = node.as_interpolated_regular_expression_node() {
            for part in interp.parts().iter() {
                self.analyze_node(&part, state);
            }
            return;
        }
        if let Some(embedded) = node.as_embedded_statements_node() {
            if let Some(stmts) = embedded.statements() {
                self.analyze_node(&stmts.as_node(), state);
            }
            return;
        }

        // Match/regex with named captures
        if let Some(match_write) = node.as_match_write_node() {
            // The regex match can create local variable writes from named captures
            self.analyze_node(&match_write.call().as_node(), state);
            for target in match_write.targets().iter() {
                if let Some(target_node) = target.as_local_variable_target_node() {
                    let name = node_name(target_node.name().as_slice());
                    state.record_write(&name, target_node.location().start_offset());
                }
            }
            return;
        }

        // Defined? node
        if let Some(defined_node) = node.as_defined_node() {
            self.analyze_node(&defined_node.value(), state);
            return;
        }

        // Instance/class/global variable reads/writes — just traverse for local var reads
        // These don't affect local variable liveness, but their values might contain
        // local variable reads.
        if let Some(ivar_write) = node.as_instance_variable_write_node() {
            self.analyze_node(&ivar_write.value(), state);
            return;
        }
        if let Some(cvar_write) = node.as_class_variable_write_node() {
            self.analyze_node(&cvar_write.value(), state);
            return;
        }
        if let Some(gvar_write) = node.as_global_variable_write_node() {
            self.analyze_node(&gvar_write.value(), state);
            return;
        }
        if let Some(const_write) = node.as_constant_write_node() {
            self.analyze_node(&const_write.value(), state);
            return;
        }
        if let Some(const_path_write) = node.as_constant_path_write_node() {
            self.analyze_node(&const_path_write.target().as_node(), state);
            self.analyze_node(&const_path_write.value(), state);
            return;
        }

        // Operator writes for non-local variables
        if let Some(ivar_op) = node.as_instance_variable_operator_write_node() {
            self.analyze_node(&ivar_op.value(), state);
            return;
        }
        if let Some(cvar_op) = node.as_class_variable_operator_write_node() {
            self.analyze_node(&cvar_op.value(), state);
            return;
        }
        if let Some(gvar_op) = node.as_global_variable_operator_write_node() {
            self.analyze_node(&gvar_op.value(), state);
            return;
        }
        if let Some(ivar_or) = node.as_instance_variable_or_write_node() {
            self.analyze_node(&ivar_or.value(), state);
            return;
        }
        if let Some(ivar_and) = node.as_instance_variable_and_write_node() {
            self.analyze_node(&ivar_and.value(), state);
            return;
        }
        if let Some(cvar_or) = node.as_class_variable_or_write_node() {
            self.analyze_node(&cvar_or.value(), state);
            return;
        }
        if let Some(cvar_and) = node.as_class_variable_and_write_node() {
            self.analyze_node(&cvar_and.value(), state);
            return;
        }
        if let Some(const_op) = node.as_constant_operator_write_node() {
            self.analyze_node(&const_op.value(), state);
            return;
        }
        if let Some(const_or) = node.as_constant_or_write_node() {
            self.analyze_node(&const_or.value(), state);
            return;
        }
        if let Some(const_and) = node.as_constant_and_write_node() {
            self.analyze_node(&const_and.value(), state);
            return;
        }
        if let Some(gvar_or) = node.as_global_variable_or_write_node() {
            self.analyze_node(&gvar_or.value(), state);
            return;
        }
        if let Some(gvar_and) = node.as_global_variable_and_write_node() {
            self.analyze_node(&gvar_and.value(), state);
            return;
        }
        if let Some(const_path_op) = node.as_constant_path_operator_write_node() {
            self.analyze_node(&const_path_op.value(), state);
            return;
        }
        if let Some(const_path_or) = node.as_constant_path_or_write_node() {
            self.analyze_node(&const_path_or.value(), state);
            return;
        }
        if let Some(const_path_and) = node.as_constant_path_and_write_node() {
            self.analyze_node(&const_path_and.value(), state);
            return;
        }

        // Index operator write (x[y] = z, x[y] += z etc.)
        if let Some(idx_op) = node.as_index_operator_write_node() {
            if let Some(recv) = idx_op.receiver() {
                self.analyze_node(&recv, state);
            }
            if let Some(args) = idx_op.arguments() {
                for arg in args.arguments().iter() {
                    self.analyze_node(&arg, state);
                }
            }
            self.analyze_node(&idx_op.value(), state);
            return;
        }
        if let Some(idx_or) = node.as_index_or_write_node() {
            if let Some(recv) = idx_or.receiver() {
                self.analyze_node(&recv, state);
            }
            if let Some(args) = idx_or.arguments() {
                for arg in args.arguments().iter() {
                    self.analyze_node(&arg, state);
                }
            }
            self.analyze_node(&idx_or.value(), state);
            return;
        }
        if let Some(idx_and) = node.as_index_and_write_node() {
            if let Some(recv) = idx_and.receiver() {
                self.analyze_node(&recv, state);
            }
            if let Some(args) = idx_and.arguments() {
                for arg in args.arguments().iter() {
                    self.analyze_node(&arg, state);
                }
            }
            self.analyze_node(&idx_and.value(), state);
            return;
        }

        // Call operator write (x.y += z)
        if let Some(call_op) = node.as_call_operator_write_node() {
            if let Some(recv) = call_op.receiver() {
                self.analyze_node(&recv, state);
            }
            self.analyze_node(&call_op.value(), state);
            return;
        }
        if let Some(call_or) = node.as_call_or_write_node() {
            if let Some(recv) = call_or.receiver() {
                self.analyze_node(&recv, state);
            }
            self.analyze_node(&call_or.value(), state);
            return;
        }
        if let Some(call_and) = node.as_call_and_write_node() {
            if let Some(recv) = call_and.receiver() {
                self.analyze_node(&recv, state);
            }
            self.analyze_node(&call_and.value(), state);
            return;
        }

        // Local variable target (in multi-assignment, for-loop, etc.)
        if let Some(target) = node.as_local_variable_target_node() {
            let name = node_name(target.name().as_slice());
            let offset = target.location().start_offset();
            self.record_write_and_check(state, &name, offset);
            return;
        }

        // For other node types, fall through — use generic Visit to find reads
        // This handles literals, constants, etc. that don't affect liveness.
        // We use a simple visitor to find any local variable reads in the subtree.
        let mut read_finder = ReadFinder { reads: Vec::new() };
        read_finder.visit(node);
        for name in read_finder.reads {
            self.record_read(state, &name);
        }
    }

    fn analyze_multi_target(&mut self, target: &ruby_prism::Node<'_>, state: &mut LiveState) {
        if let Some(target_node) = target.as_local_variable_target_node() {
            let name = node_name(target_node.name().as_slice());
            let offset = target_node.location().start_offset();
            self.record_write_and_check(state, &name, offset);
        }
        // Other targets (instance vars, etc.) — ignore for local var analysis
    }

    fn analyze_if(&mut self, node: &ruby_prism::IfNode<'_>, state: &mut LiveState) {
        // Protect pre-if writes before analyzing the predicate. This handles
        // modifier-if patterns like `puts a if (a = 123)` where the predicate
        // contains a local variable assignment that overwrites a prior write,
        // but the body reads the variable. RuboCop does not flag the prior
        // write as useless in this case.
        let has_predicate_lvar_write = contains_local_variable_write(&node.predicate());
        let saved_protected = if has_predicate_lvar_write {
            self.protect_live_writes(state)
        } else {
            self.protected_offsets.clone()
        };

        // Analyze predicate
        self.analyze_node(&node.predicate(), state);

        let before = state.clone();
        // Protect pre-branch writes from being marked useless when overwritten
        // inside a branch (the branch may not execute).
        self.protect_live_writes(&before);

        // Analyze consequent (if-branch)
        let mut if_state = before.clone();
        if let Some(stmts) = node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            self.analyze_statements(&body, &mut if_state);
        }

        // Analyze else branch
        if let Some(else_clause) = node.subsequent() {
            let mut else_state = before.clone();
            // else clause can be ElseNode or IfNode (elsif)
            if let Some(else_node) = else_clause.as_else_node() {
                if let Some(stmts) = else_node.statements() {
                    let body: Vec<_> = stmts.body().iter().collect();
                    self.analyze_statements(&body, &mut else_state);
                }
            } else if let Some(elsif_node) = else_clause.as_if_node() {
                self.analyze_if(&elsif_node, &mut else_state);
            }
            // Before merging, detect variables that are unread in BOTH
            // branches but were NOT in the pre-branch state. Both writes are
            // useless, but merge_branches keeps only one offset. Store the
            // "lost" offset to be reported at end-of-scope if the variable
            // is not read after the branch.
            for (name, &if_offset) in &if_state.live_writes {
                if let Some(&else_offset) = else_state.live_writes.get(name) {
                    if !before.live_writes.contains_key(name) && if_offset != else_offset {
                        // merge_branches keeps else_offset. Store if_offset as extra.
                        self.branch_extra_writes.push((name.clone(), if_offset));
                    }
                }
            }
            // Both branches exist — merge
            *state = LiveState::merge_branches(&if_state, &else_state);
        } else {
            // Single-branch if — merge with pre-branch state
            *state = LiveState::merge_optional_branch(&before, &if_state);
        }

        self.restore_protected(saved_protected);
    }

    fn analyze_unless(&mut self, node: &ruby_prism::UnlessNode<'_>, state: &mut LiveState) {
        let has_predicate_lvar_write = contains_local_variable_write(&node.predicate());
        let saved_protected = if has_predicate_lvar_write {
            self.protect_live_writes(state)
        } else {
            self.protected_offsets.clone()
        };

        self.analyze_node(&node.predicate(), state);
        let before = state.clone();
        self.protect_live_writes(&before);

        let mut body_state = before.clone();
        if let Some(stmts) = node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            self.analyze_statements(&body, &mut body_state);
        }

        if let Some(else_clause) = node.else_clause() {
            let mut else_state = before.clone();
            if let Some(stmts) = else_clause.statements() {
                let body: Vec<_> = stmts.body().iter().collect();
                self.analyze_statements(&body, &mut else_state);
            }
            // Track extra writes from both branches (same as analyze_if)
            for (name, &body_offset) in &body_state.live_writes {
                if let Some(&else_offset) = else_state.live_writes.get(name) {
                    if !before.live_writes.contains_key(name) && body_offset != else_offset {
                        self.branch_extra_writes.push((name.clone(), body_offset));
                    }
                }
            }
            *state = LiveState::merge_branches(&body_state, &else_state);
        } else {
            *state = LiveState::merge_optional_branch(&before, &body_state);
        }

        self.restore_protected(saved_protected);
    }

    fn analyze_case(&mut self, node: &ruby_prism::CaseNode<'_>, state: &mut LiveState) {
        if let Some(predicate) = node.predicate() {
            self.analyze_node(&predicate, state);
        }
        let before = state.clone();
        let saved_protected = self.protect_live_writes(&before);
        let mut has_else = false;
        let mut branch_states = Vec::new();

        for condition in node.conditions().iter() {
            if let Some(when_node) = condition.as_when_node() {
                let mut branch_state = before.clone();
                for cond in when_node.conditions().iter() {
                    self.analyze_node(&cond, &mut branch_state);
                }
                if let Some(stmts) = when_node.statements() {
                    let body: Vec<_> = stmts.body().iter().collect();
                    self.analyze_statements(&body, &mut branch_state);
                }
                branch_states.push(branch_state);
            }
        }

        if let Some(else_clause) = node.else_clause() {
            has_else = true;
            let mut else_state = before.clone();
            if let Some(stmts) = else_clause.statements() {
                let body: Vec<_> = stmts.body().iter().collect();
                self.analyze_statements(&body, &mut else_state);
            }
            branch_states.push(else_state);
        }

        if has_else && branch_states.len() > 1 {
            let mut merged = branch_states[0].clone();
            for bs in &branch_states[1..] {
                merged = LiveState::merge_branches(&merged, bs);
            }
            *state = merged;
        } else {
            let mut merged = before;
            for bs in &branch_states {
                merged = LiveState::merge_optional_branch(&merged, bs);
            }
            *state = merged;
        }

        self.restore_protected(saved_protected);
    }

    fn analyze_case_match(&mut self, node: &ruby_prism::CaseMatchNode<'_>, state: &mut LiveState) {
        if let Some(predicate) = node.predicate() {
            self.analyze_node(&predicate, state);
        }
        let before = state.clone();
        let saved_protected = self.protect_live_writes(&before);
        let mut branch_states = Vec::new();
        let mut has_else = false;

        for condition in node.conditions().iter() {
            if let Some(in_node) = condition.as_in_node() {
                let mut branch_state = before.clone();
                self.analyze_node(&in_node.pattern(), &mut branch_state);
                if let Some(stmts) = in_node.statements() {
                    let body: Vec<_> = stmts.body().iter().collect();
                    self.analyze_statements(&body, &mut branch_state);
                }
                branch_states.push(branch_state);
            }
        }

        if let Some(else_clause) = node.else_clause() {
            has_else = true;
            let mut else_state = before.clone();
            if let Some(stmts) = else_clause.statements() {
                let body: Vec<_> = stmts.body().iter().collect();
                self.analyze_statements(&body, &mut else_state);
            }
            branch_states.push(else_state);
        }

        if has_else && branch_states.len() > 1 {
            let mut merged = branch_states[0].clone();
            for bs in &branch_states[1..] {
                merged = LiveState::merge_branches(&merged, bs);
            }
            *state = merged;
        } else {
            let mut merged = before;
            for bs in &branch_states {
                merged = LiveState::merge_optional_branch(&merged, bs);
            }
            *state = merged;
        }

        self.restore_protected(saved_protected);
    }

    fn analyze_while(&mut self, node: &ruby_prism::WhileNode<'_>, state: &mut LiveState) {
        // Protect pre-loop writes if the condition contains an lvar assignment
        // (handles `puts a while (a = false)` pattern).
        let has_predicate_lvar_write = contains_local_variable_write(&node.predicate());
        let saved_protected = if has_predicate_lvar_write {
            self.protect_live_writes(state)
        } else {
            self.protected_offsets.clone()
        };

        // Analyze condition
        self.analyze_node(&node.predicate(), state);

        let before = state.clone();
        self.protect_live_writes(&before);

        // Analyze body twice to simulate loop back-edge:
        // First pass: find reads/writes in the body
        // Second pass: propagate reads from start of loop to end
        if let Some(stmts) = node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            let mut loop_state = before.clone();
            self.analyze_statements(&body, &mut loop_state);
            self.analyze_node(&node.predicate(), &mut loop_state);
            // Protect writes from pass 1 before pass 2 — a write in the loop
            // body from the first iteration may be read in the next iteration
            // or after the loop. Don't flag it as useless when overwritten in
            // the second simulation pass.
            self.protect_live_writes(&loop_state);
            let mut loop_state2 = loop_state.clone();
            self.analyze_statements(&body, &mut loop_state2);
            self.analyze_node(&node.predicate(), &mut loop_state2);
            *state = LiveState::merge_optional_branch(&before, &loop_state2);
        }

        self.restore_protected(saved_protected);
    }

    fn analyze_until(&mut self, node: &ruby_prism::UntilNode<'_>, state: &mut LiveState) {
        let has_predicate_lvar_write = contains_local_variable_write(&node.predicate());
        let saved_protected = if has_predicate_lvar_write {
            self.protect_live_writes(state)
        } else {
            self.protected_offsets.clone()
        };

        self.analyze_node(&node.predicate(), state);
        let before = state.clone();
        self.protect_live_writes(&before);
        if let Some(stmts) = node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            let mut loop_state = before.clone();
            self.analyze_statements(&body, &mut loop_state);
            self.analyze_node(&node.predicate(), &mut loop_state);
            self.protect_live_writes(&loop_state);
            let mut loop_state2 = loop_state.clone();
            self.analyze_statements(&body, &mut loop_state2);
            self.analyze_node(&node.predicate(), &mut loop_state2);
            *state = LiveState::merge_optional_branch(&before, &loop_state2);
        }
        self.restore_protected(saved_protected);
    }

    fn analyze_for(&mut self, node: &ruby_prism::ForNode<'_>, state: &mut LiveState) {
        // Analyze collection expression
        self.analyze_node(&node.collection(), state);

        // The index variable is a write
        self.analyze_for_index(&node.index(), state);

        let before = state.clone();
        let saved_protected = self.protect_live_writes(&before);
        if let Some(stmts) = node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            let mut loop_state = before.clone();
            self.analyze_statements(&body, &mut loop_state);
            self.analyze_for_index(&node.index(), &mut loop_state);
            let mut loop_state2 = loop_state.clone();
            self.analyze_statements(&body, &mut loop_state2);
            *state = LiveState::merge_optional_branch(&before, &loop_state2);
        }
        self.restore_protected(saved_protected);
    }

    fn analyze_for_index(&mut self, index: &ruby_prism::Node<'_>, state: &mut LiveState) {
        if let Some(target) = index.as_local_variable_target_node() {
            let name = node_name(target.name().as_slice());
            let offset = target.location().start_offset();
            self.record_write_and_check(state, &name, offset);
        } else if let Some(multi) = index.as_multi_target_node() {
            for target in multi.lefts().iter() {
                self.analyze_for_index(&target, state);
            }
            if let Some(rest) = multi.rest() {
                if let Some(splat) = rest.as_splat_node() {
                    if let Some(expr) = splat.expression() {
                        self.analyze_for_index(&expr, state);
                    }
                }
            }
            for target in multi.rights().iter() {
                self.analyze_for_index(&target, state);
            }
        }
    }

    fn analyze_begin(&mut self, node: &ruby_prism::BeginNode<'_>, state: &mut LiveState) {
        let has_rescue = node.rescue_clause().is_some();
        let has_ensure = node.ensure_clause().is_some();

        // When a rescue or ensure clause exists, protect pre-begin writes from
        // being marked useless during begin body analysis. Any statement in the
        // body could raise, so the pre-begin value may still be the live one
        // when control reaches the rescue/ensure or post-begin code.
        let saved_protected = if has_rescue || has_ensure {
            self.protect_live_writes(state)
        } else {
            self.protected_offsets.clone()
        };

        // Begin body
        let before = state.clone();
        if let Some(stmts) = node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            self.analyze_statements(&body, state);
        }

        // Rescue clauses — each is an optional branch from the begin body.
        // The rescue starting state must include writes from the begin body,
        // because any statement in the body can raise and the rescue clause
        // may read variables that were written before the exception. Using
        // `before` alone (the pre-body state) causes FPs on patterns like:
        //   counter = 0; begin; counter += 1; work; rescue; retry if counter < 3; end
        // where the rescue reads a variable written inside the begin body.
        // We merge post-body state (primary) with pre-body state (fallback for
        // variables not touched in the body), so rescue sees all writes.
        if let Some(rescue_node) = node.rescue_clause() {
            let mut rescue_start = state.clone();
            // Add any pre-body writes for variables NOT written in the body
            for (name, &offset) in &before.live_writes {
                if !rescue_start.live_writes.contains_key(name) {
                    rescue_start.live_writes.insert(name.clone(), offset);
                }
            }
            self.protect_live_writes(&before);
            self.analyze_rescue_chain(&rescue_node, &rescue_start, state);
        }

        // Else clause (runs if no exception)
        if let Some(else_node) = node.else_clause() {
            if let Some(stmts) = else_node.statements() {
                let body: Vec<_> = stmts.body().iter().collect();
                self.analyze_statements(&body, state);
            }
        }

        // Ensure clause (always runs)
        if let Some(ensure_node) = node.ensure_clause() {
            if let Some(stmts) = ensure_node.statements() {
                let body: Vec<_> = stmts.body().iter().collect();
                self.analyze_statements(&body, state);
            }
        }

        self.restore_protected(saved_protected);
    }

    fn analyze_rescue_chain(
        &mut self,
        rescue_node: &ruby_prism::RescueNode<'_>,
        before_body: &LiveState,
        state: &mut LiveState,
    ) {
        // Each rescue clause is like an optional branch
        let mut rescue_state = before_body.clone();

        // Analyze exception classes
        for exc in rescue_node.exceptions().iter() {
            self.analyze_node(&exc, &mut rescue_state);
        }

        // Exception variable assignment
        if let Some(reference) = rescue_node.reference() {
            if let Some(target) = reference.as_local_variable_target_node() {
                let name = node_name(target.name().as_slice());
                let offset = target.location().start_offset();
                rescue_state.record_write(&name, offset);
            }
        }

        // Rescue body
        if let Some(stmts) = rescue_node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            self.analyze_statements(&body, &mut rescue_state);
        }

        // Merge rescue state (optional branch)
        *state = LiveState::merge_optional_branch(state, &rescue_state);

        // Chain to next rescue clause
        if let Some(next_rescue) = rescue_node.subsequent() {
            self.analyze_rescue_chain(&next_rescue, before_body, state);
        }
    }
}

/// Collect names of variables that are written with depth > 0 in a block body.
/// These are outer-scope variables captured by the closure and should not be
/// flagged as useless within the block's own scope analysis.
fn collect_outer_scope_write_names(body: &ruby_prism::Node<'_>) -> HashSet<String> {
    struct OuterWriteCollector {
        names: HashSet<String>,
    }
    impl<'pr> Visit<'pr> for OuterWriteCollector {
        fn visit_local_variable_write_node(
            &mut self,
            node: &ruby_prism::LocalVariableWriteNode<'pr>,
        ) {
            if node.depth() > 0 {
                self.names.insert(node_name(node.name().as_slice()));
            }
            ruby_prism::visit_local_variable_write_node(self, node);
        }
        fn visit_local_variable_operator_write_node(
            &mut self,
            node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
        ) {
            if node.depth() > 0 {
                self.names.insert(node_name(node.name().as_slice()));
            }
            ruby_prism::visit_local_variable_operator_write_node(self, node);
        }
        fn visit_local_variable_or_write_node(
            &mut self,
            node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
        ) {
            if node.depth() > 0 {
                self.names.insert(node_name(node.name().as_slice()));
            }
            ruby_prism::visit_local_variable_or_write_node(self, node);
        }
        fn visit_local_variable_and_write_node(
            &mut self,
            node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
        ) {
            if node.depth() > 0 {
                self.names.insert(node_name(node.name().as_slice()));
            }
            ruby_prism::visit_local_variable_and_write_node(self, node);
        }
        fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
        fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
        fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
    }
    let mut collector = OuterWriteCollector {
        names: HashSet::new(),
    };
    collector.visit(body);
    collector.names
}

/// Check if a node subtree contains any local variable write.
/// Used to detect modifier-if patterns where the condition contains an assignment.
fn contains_local_variable_write(node: &ruby_prism::Node<'_>) -> bool {
    struct WriteDetector {
        found: bool,
    }
    impl<'pr> Visit<'pr> for WriteDetector {
        fn visit_local_variable_write_node(
            &mut self,
            _node: &ruby_prism::LocalVariableWriteNode<'pr>,
        ) {
            self.found = true;
        }
        fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
        fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
        fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
    }
    let mut detector = WriteDetector { found: false };
    detector.visit(node);
    detector.found
}

/// Helper to convert name bytes to String.
fn node_name(bytes: &[u8]) -> String {
    std::str::from_utf8(bytes).unwrap_or("").to_string()
}

/// Simple visitor that collects all local variable reads in a subtree.
/// Used as fallback for node types not explicitly handled by the analyzer.
struct ReadFinder {
    reads: Vec<String>,
}

impl<'pr> Visit<'pr> for ReadFinder {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        self.reads.push(node_name(node.name().as_slice()));
    }

    // Don't recurse into new scopes
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        // Singleton def receiver reads the variable
        if let Some(receiver) = node.receiver() {
            self.visit(&receiver);
        }
    }
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        // The expression (e.g., `obj` in `class << obj`) reads the variable
        self.visit(&node.expression());
        // Don't recurse into body — new scope
    }
}

/// Collects reads and writes from a closure (block/lambda body).
/// Doesn't recurse into nested hard scopes (def/class/module).
struct ClosureCollector {
    reads: HashSet<String>,
    writes: HashSet<String>,
    has_binding: bool,
    /// Block parameter names — these shadow outer variables.
    params: HashSet<String>,
}

impl ClosureCollector {
    fn new() -> Self {
        Self {
            reads: HashSet::new(),
            writes: HashSet::new(),
            has_binding: false,
            params: HashSet::new(),
        }
    }
}

impl<'pr> Visit<'pr> for ClosureCollector {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        let name = node_name(node.name().as_slice());
        if !self.params.contains(&name) {
            self.reads.insert(name);
        }
    }

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        let name = node_name(node.name().as_slice());
        if !self.params.contains(&name) {
            self.writes.insert(name);
        }
        self.visit(&node.value());
    }

    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        let name = node_name(node.name().as_slice());
        if !self.params.contains(&name) {
            self.reads.insert(name.clone());
            self.writes.insert(name);
        }
        self.visit(&node.value());
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        let name = node_name(node.name().as_slice());
        if !self.params.contains(&name) {
            self.reads.insert(name.clone());
            self.writes.insert(name);
        }
        self.visit(&node.value());
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        let name = node_name(node.name().as_slice());
        if !self.params.contains(&name) {
            self.reads.insert(name.clone());
            self.writes.insert(name);
        }
        self.visit(&node.value());
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.receiver().is_none()
            && node.name().as_slice() == b"binding"
            && node.arguments().is_none_or(|a| a.arguments().is_empty())
        {
            self.has_binding = true;
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_forwarding_super_node(&mut self, _node: &ruby_prism::ForwardingSuperNode<'pr>) {
        // forwarding super in a block doesn't affect outer scope
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        // Singleton def receiver reads the variable
        if let Some(receiver) = node.receiver() {
            if let Some(read_node) = receiver.as_local_variable_read_node() {
                let name = node_name(read_node.name().as_slice());
                if !self.params.contains(&name) {
                    self.reads.insert(name);
                }
            }
        }
        // Don't recurse into body — new scope
    }

    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        // The expression (e.g., `obj` in `class << obj`) reads the variable
        if let Some(read_node) = node.expression().as_local_variable_read_node() {
            let name = node_name(read_node.name().as_slice());
            if !self.params.contains(&name) {
                self.reads.insert(name);
            }
        }
        // Don't recurse into body — new scope
    }

    // Nested blocks create child closures that also capture outer variables
    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        ruby_prism::visit_block_node(self, node);
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        ruby_prism::visit_lambda_node(self, node);
    }
}

/// Extract parameter names from a DefNode's parameter list.
fn collect_param_names(node: &ruby_prism::DefNode<'_>) -> HashSet<String> {
    let mut names = HashSet::new();
    if let Some(params) = node.parameters() {
        collect_parameters_node_names(&params, &mut names);
        if let Some(block) = params.block() {
            if let Some(name) = block.name() {
                if let Ok(s) = std::str::from_utf8(name.as_slice()) {
                    names.insert(s.to_string());
                }
            }
        }
    }
    names
}

/// Collect parameter names from a ParametersNode into a set.
fn collect_parameters_node_names(
    params: &ruby_prism::ParametersNode<'_>,
    names: &mut HashSet<String>,
) {
    for p in params.requireds().iter() {
        if let Some(req) = p.as_required_parameter_node() {
            if let Ok(s) = std::str::from_utf8(req.name().as_slice()) {
                names.insert(s.to_string());
            }
        }
    }
    for p in params.optionals().iter() {
        if let Some(opt) = p.as_optional_parameter_node() {
            if let Ok(s) = std::str::from_utf8(opt.name().as_slice()) {
                names.insert(s.to_string());
            }
        }
    }
    if let Some(rest) = params.rest() {
        if let Some(rest_param) = rest.as_rest_parameter_node() {
            if let Some(name) = rest_param.name() {
                if let Ok(s) = std::str::from_utf8(name.as_slice()) {
                    names.insert(s.to_string());
                }
            }
        }
    }
    for p in params.keywords().iter() {
        if let Some(kw) = p.as_required_keyword_parameter_node() {
            if let Ok(s) = std::str::from_utf8(kw.name().as_slice()) {
                names.insert(s.trim_end_matches(':').to_string());
            }
        } else if let Some(kw) = p.as_optional_keyword_parameter_node() {
            if let Ok(s) = std::str::from_utf8(kw.name().as_slice()) {
                names.insert(s.trim_end_matches(':').to_string());
            }
        }
    }
    if let Some(kw_rest) = params.keyword_rest() {
        if let Some(kw_rest_param) = kw_rest.as_keyword_rest_parameter_node() {
            if let Some(name) = kw_rest_param.name() {
                if let Ok(s) = std::str::from_utf8(name.as_slice()) {
                    names.insert(s.to_string());
                }
            }
        }
    }
}

/// Collect parameter names from a BlockNode's parameter list into a reads set.
fn collect_block_param_names(params: &ruby_prism::Node<'_>, reads: &mut HashSet<String>) {
    if let Some(block_params) = params.as_block_parameters_node() {
        if let Some(inner_params) = block_params.parameters() {
            collect_parameters_node_names(&inner_params, reads);
        }
    } else if let Some(numbered) = params.as_numbered_parameters_node() {
        for i in 1..=numbered.maximum() {
            reads.insert(format!("_{i}"));
        }
    }
}

// ── Top-level visitor ───────────────────────────────────────────────────────

impl UselessAssignVisitor<'_, '_> {
    fn report_useless(
        &mut self,
        analyzer: &ScopeAnalyzer,
        state: &LiveState,
        param_names: &HashSet<String>,
    ) {
        if analyzer.has_binding {
            return;
        }
        // Report writes that were detected as useless during sequential analysis
        for w in &analyzer.useless {
            if w.name.starts_with('_') || param_names.contains(&w.name) {
                continue;
            }
            // If the variable is read in any closure, don't flag it
            if analyzer.closure_reads.contains(&w.name) {
                continue;
            }
            let (line, column) = self.source.offset_to_line_col(w.offset);
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                format!("Useless assignment to variable - `{}`.", w.name),
            ));
        }
        // Report writes that are still live at end of scope (never read)
        for (name, offset) in state.unread_writes() {
            if name.starts_with('_') || param_names.contains(name) {
                continue;
            }
            // If the variable is read in any closure, don't flag it
            if analyzer.closure_reads.contains(name) {
                continue;
            }
            // If this specific write was ever consumed by a read anywhere in
            // the scope (including inside branches/loops), it's not useless.
            // The merge logic may have re-inserted it as "live" (unread) but
            // there exists a code path where it IS read.
            if analyzer.ever_read_offsets.contains(&offset) {
                continue;
            }
            let (line, column) = self.source.offset_to_line_col(offset);
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                format!("Useless assignment to variable - `{}`.", name),
            ));
        }
        // Report extra writes from branch merges that were never read.
        // These occur when both branches of an if/unless write the same
        // variable and the merge can only keep one offset.
        for (name, offset) in &analyzer.branch_extra_writes {
            if name.starts_with('_') || param_names.contains(name) {
                continue;
            }
            if analyzer.closure_reads.contains(name) {
                continue;
            }
            if analyzer.ever_read_offsets.contains(offset) {
                continue;
            }
            let (line, column) = self.source.offset_to_line_col(*offset);
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                format!("Useless assignment to variable - `{}`.", name),
            ));
        }
    }

    fn analyze_def(&mut self, node: &ruby_prism::DefNode<'_>) {
        if let Some(body) = node.body() {
            let mut analyzer = ScopeAnalyzer::new();
            let mut state = LiveState::new();
            analyzer.analyze_node(&body, &mut state);

            let param_names = collect_param_names(node);

            // If bare super is used, all params are implicitly read
            if analyzer.has_forwarding_super {
                for name in &param_names {
                    state.record_read(name);
                }
            }

            self.report_useless(&analyzer, &state, &param_names);
        }
    }

    fn analyze_block_scope(
        &mut self,
        body: &ruby_prism::Node<'_>,
        params: &HashSet<String>,
        outer_vars: &HashSet<String>,
    ) {
        let mut analyzer = ScopeAnalyzer::new();
        let mut state = LiveState::new();

        // Block params are implicitly "read"
        for name in params {
            state.record_read(name);
        }

        // Outer scope variables (depth > 0) are excluded from reporting:
        // merge them into params so they're treated as "known" and skipped.
        let mut skip_names = params.clone();
        skip_names.extend(outer_vars.iter().cloned());

        analyzer.analyze_node(body, &mut state);
        self.report_useless(&analyzer, &state, &skip_names);
    }
}

impl<'pr> Visit<'pr> for UselessAssignVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        self.analyze_def(node);
        // Mark that we're inside a def so blocks don't get separate analysis
        let was_inside_def = self.inside_def;
        self.inside_def = true;
        ruby_prism::visit_def_node(self, node);
        self.inside_def = was_inside_def;
    }

    fn visit_program_node(&mut self, node: &ruby_prism::ProgramNode<'pr>) {
        // Analyze top-level scope
        let body = node.statements().as_node();
        let mut analyzer = ScopeAnalyzer::new();
        let mut state = LiveState::new();
        analyzer.analyze_node(&body, &mut state);
        let empty_params = HashSet::new();
        self.report_useless(&analyzer, &state, &empty_params);

        // Continue visiting to find defs/blocks at top level
        ruby_prism::visit_program_node(self, node);
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // Blocks inside defs are handled as closures by the def's ScopeAnalyzer
        // — they don't need independent scope analysis.
        // Blocks NOT inside defs always get their own analysis to detect useless
        // assignments within them. Even nested blocks are analyzed independently
        // — the parent's ClosureCollector handles cross-scope variable effects,
        // while the child's ScopeAnalyzer handles internal per-assignment tracking.
        if !self.inside_def {
            if let Some(body) = node.body() {
                let mut params = HashSet::new();
                if let Some(p) = node.parameters() {
                    collect_block_param_names(&p, &mut params);
                }
                let outer_vars = collect_outer_scope_write_names(&body);
                self.analyze_block_scope(&body, &params, &outer_vars);
            }
        }
        // Continue visiting to find nested defs and blocks
        ruby_prism::visit_block_node(self, node);
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        if !self.inside_def {
            if let Some(body) = node.body() {
                let mut params = HashSet::new();
                if let Some(p) = node.parameters() {
                    if let Some(block_params) = p.as_block_parameters_node() {
                        if let Some(inner) = block_params.parameters() {
                            collect_parameters_node_names(&inner, &mut params);
                        }
                    }
                }
                let outer_vars = collect_outer_scope_write_names(&body);
                self.analyze_block_scope(&body, &params, &outer_vars);
            }
        }
        ruby_prism::visit_lambda_node(self, node);
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        if let Some(body) = node.body() {
            let mut analyzer = ScopeAnalyzer::new();
            let mut state = LiveState::new();
            analyzer.analyze_node(&body, &mut state);
            let empty_params = HashSet::new();
            self.report_useless(&analyzer, &state, &empty_params);
        }
        ruby_prism::visit_class_node(self, node);
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        if let Some(body) = node.body() {
            let mut analyzer = ScopeAnalyzer::new();
            let mut state = LiveState::new();
            analyzer.analyze_node(&body, &mut state);
            let empty_params = HashSet::new();
            self.report_useless(&analyzer, &state, &empty_params);
        }
        ruby_prism::visit_module_node(self, node);
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        if let Some(body) = node.body() {
            let mut analyzer = ScopeAnalyzer::new();
            let mut state = LiveState::new();
            analyzer.analyze_node(&body, &mut state);
            let empty_params = HashSet::new();
            self.report_useless(&analyzer, &state, &empty_params);
        }
        ruby_prism::visit_singleton_class_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessAssignment, "cops/lint/useless_assignment");
}
