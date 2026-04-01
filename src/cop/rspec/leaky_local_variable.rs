use crate::cop::node_type::{BLOCK_NODE, CALL_NODE};
use crate::cop::util::{self, RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// Flags local variable assignments at the example-group level that are then
/// referenced inside examples, hooks, let, or subject blocks. Use `let` instead.
///
/// ## Root cause of previous FP/FN gap (23 FP, 933 FN)
///
/// The old implementation only collected direct `LocalVariableWriteNode` children
/// of the block body (top-level statements). Assignments nested inside
/// `if`/`unless`/`case`/`begin` or iterator blocks were missed (933 FN).
///
/// FPs came from not properly handling block parameter shadowing and variables
/// used only in example descriptions/metadata.
///
/// ## Current approach
///
/// Instead of VariableForce (which RuboCop uses), we take a pragmatic approach:
/// 1. When visiting an example group block, recursively collect ALL local variable
///    assignments within the block body, stopping at scope boundaries (examples,
///    hooks, let, subject, nested example groups).
/// 2. For each assignment, check if the variable is referenced inside any example
///    scope (examples, hooks, let, subject, includes args).
/// 3. Exclude "allowed" references: variables used only in example descriptions,
///    metadata keyword args, `it_behaves_like` first arg, or interpolated
///    string/symbol args to includes methods.
/// 4. Respect block parameter shadowing throughout.
///
/// ## Investigation (FP=41, FN=409, 2026-03-10)
///
/// **FP fix: reassignment-before-use (41 FPs)**
/// RuboCop's VariableForce performs flow-sensitive analysis, tracking that a
/// variable reassigned inside an example block before any read creates a new
/// binding that doesn't reference the outer scope. Our implementation now checks
/// `var_written_before_read_in_stmts` to suppress offenses when the first mention
/// of the variable in the block is an unconditional write.
///
/// **FN fix: missing `include_context` (contributes to 409 FNs)**
/// `is_includes_method` was missing `include_context`. RuboCop's `Includes.all`
/// includes both `Examples` (`it_behaves_like`, `it_should_behave_like`,
/// `include_examples`) and `Context` (`include_context`). Variables passed as
/// non-first args to `include_context` should be flagged.
///
/// **Remaining FN gap:** The bulk of the 409 FNs likely comes from cases where
/// RuboCop's VariableForce tracks variable references across Ruby scope
/// boundaries that our AST-walking approach doesn't replicate (e.g., variables
/// assigned before the top-level `describe` block, complex flow-sensitive
/// reassignment patterns). A full VariableForce implementation would close this
/// gap but is a significant engineering effort.
///
/// ## Investigation (FP=32, FN=409, 2026-03-11)
///
/// **FN fix: file-level variables (major FN source)**
/// Added `check_source` to detect variables assigned at file level (outside
/// describe blocks) that are referenced inside example scopes within describe
/// blocks. Corpus FN examples showed patterns like `spec_helper/xcscheme.rb:5`
/// where variables are assigned at line 2-6 before any describe block.
/// Implementation uses `check_file_level_vars` which collects file-level
/// assignments and checks them against all describe blocks in the file.
///
/// **FP fix: begin-block reassignment (reduces remaining FPs)**
/// Improved `is_unconditional_var_write` to recurse into `begin` blocks and
/// parenthesized expressions. A write inside `begin; x = ...; end` at the
/// start of an example block means the outer variable is never read, matching
/// RuboCop's VariableForce behavior.
///
/// **Remaining gaps:** 32 FPs from prior cycle likely involve complex
/// reassignment patterns (e.g., reassignment after non-reading statements,
/// or inside rescue blocks). 409 FNs from prior cycle partially addressed
/// by file-level variable detection; remaining FNs likely from VariableForce's
/// comprehensive scope tracking that we don't fully replicate.
///
/// ## Investigation (FP=53, FN=75, 2026-03-12)
///
/// **FN fix: operator-write nodes (`x += 1`, `x -= 1`, etc.)**
/// `LocalVariableOperatorWriteNode` was not handled in `node_references_var`,
/// `node_reads_var`, or `collect_assignments_in_scope`. Operator-writes both
/// read and write the variable (`x += 1` is `x = x + 1`). Inside example
/// blocks, `x += 1` was invisible as a reference to outer `x`. At group
/// level, `x += 1` was not collected as an assignment. Added handling for
/// all three functions.
///
/// **FN fix: interpolated regular expressions (`/#{x}/`)**
/// `InterpolatedRegularExpressionNode` was not handled in `node_references_var`.
/// Variables used only in regex interpolation inside example blocks were missed.
///
/// **FN fix: `for` loops in `node_references_var`**
/// `ForNode` was handled in `collect_assignments_in_scope` but not in
/// `node_references_var`, so variable references inside for-loop bodies
/// were invisible.
///
/// ## Investigation (FP=53, FN=64, 2026-03-14)
///
/// No example locations available in corpus data. Investigated by comparing
/// implementations and vendor spec.
///
/// **FP fix: file-level variable shadowed by group-level reassignment**
/// `check_var_used_in_describe_blocks` (used by `check_source` for file-level
/// variables) was using `check_var_used_in_example_scopes` to detect if a
/// file-level variable is referenced inside a describe block's example scopes.
/// It was NOT checking whether the variable was unconditionally reassigned at
/// the describe group's scope level before any example reference. This caused
/// false positives when a file-level variable was reassigned at the group scope
/// (making the file-level value dead), but the group-level assignment's value
/// was then used in examples.
///
/// Fix: added a `var_reassigned_before_example_ref_in_stmts` check at the top
/// of the describe block's statement traversal in `check_var_used_in_describe_blocks`.
/// If the variable is unconditionally reassigned at the group's top-level scope
/// before any example scope reference, the file-level assignment is considered
/// dead and no offense is reported for it. The group-level reassignment would
/// itself be detected and reported separately by `check_node` / `check_scope_for_leaky_vars`.
///
/// **Remaining gaps:**
/// - Any FPs from the prior investigation cycle that stemmed from complex
///   VariableForce flow analysis (e.g., conditional assignments, `begin/rescue`
///   paths) are not addressed without implementing a full VariableForce equivalent.
/// - The 64 FNs likely stem from the same root cause: RuboCop's VariableForce
///   tracks variable lifetime across all Ruby scope boundaries, while our
///   AST-walking approach uses heuristics for common patterns.
///
/// ## Investigation (FP=53, FN=75, 2026-03-15)
///
/// **FP fix: flow-aware dead assignment analysis**
/// RuboCop's VariableForce tracks per-assignment references: if a variable
/// assigned at group scope is unconditionally reassigned inside an example
/// scope (e.g., a `before` hook or `it` block) before being read, subsequent
/// example-scope reads belong to the example-scope assignment, not the
/// group-level one. Common patterns:
///   - `result = nil` at group scope, `result = compute()` in `before` hook
///   - `data = []` at group scope, `data = [1,2,3]` in first `it` block,
///     `expect(data)` in second `it` block
///
/// Our previous implementation (`check_var_used_in_example_scopes_with_reassign`)
/// checked each example scope independently but didn't do linear flow analysis
/// across statements. The new `var_value_reaches_example_scope_in_stmts` walks
/// statements linearly after the assignment, tracking whether the group-level
/// value has been "killed" by an example-scope reassignment. Once killed,
/// subsequent example-scope reads don't count as references to the group-level
/// assignment.
///
/// **Remaining gaps:**
/// - FPs from rswag/swagger DSL patterns (discourse): variables assigned inside
///   `post`/`response` blocks (non-standard DSL methods) are collected as
///   group-level assignments. These may require recognizing rswag DSL methods
///   as scope boundaries.
/// - FPs from `||=` and flow-through reassignment at file level (dev-sec):
///   `flags = parse(...)`, `flags ||= ''`, `flags = flags.split(' ')` —
///   requires tracking that `||=` and reads-then-writes are different from
///   unconditional writes.
/// - 75 FNs: VariableForce's comprehensive scope tracking across all Ruby
///   scope boundaries that our AST-walking approach doesn't replicate.
///
/// ## Investigation (FP=38, FN=286, 2026-03-19)
///
/// **FP fix: block parameter shadowing in reference checks**
/// `stmt_example_scope_var_interaction`, `check_var_used_in_example_scopes`,
/// and `check_var_used_in_describe_blocks` all recurse into "other calls with
/// blocks" (e.g., `.each do |x| ... end`) without checking if the block has a
/// parameter that shadows the variable. This caused FPs when a variable
/// assigned in one `.each` block was referenced in a later `.each` block where
/// the same name was a block parameter (openproject pattern: `schema_name`
/// assigned in first `.each`, shadowed by block param in second `.each`).
/// Fix: added `block_has_param` check before recursing into block bodies.
///
/// **FP fix: `collect_file_level_assignments` stopping at example scopes**
/// The function recursed into example scope methods (`it`, `before`, `let`,
/// `subject`, etc.) collecting assignments inside them as "file-level" vars.
/// This caused FPs for non-describe-block wrappers like
/// `Capybara::SpecHelper.spec` where `it` blocks with local vars were
/// incorrectly collected as file-level assignments.
/// Fix: added `is_example_scope` and `is_includes_method` checks.
///
/// **FP fix: dead file-level assignment filtering**
/// File-level variables assigned multiple times (e.g., `flags = parse(...)`,
/// `flags ||= ''`, `flags = flags.split(' ')`) were all flagged when only the
/// last unconditional assignment's value reaches examples. Added
/// `filter_dead_file_level_assignments` using `is_unconditional` tracking on
/// `VarAssign` to mark `||=`, `&&=`, and `+=`-style writes as conditional.
/// Earlier assignments with a later unconditional assignment (and no
/// describe-block reference between them) are filtered as dead.
///
/// **Remaining FP gaps (estimate ~20-25 remaining):**
/// - rswag/discourse FPs: variables inside `post`/`response` DSL blocks are
///   collected as group-level assignments. A full fix requires either
///   recognizing rswag DSL methods or implementing per-assignment reference
///   tracking like VariableForce.
/// - jruby `platform_is` conditional reassignment: variables conditionally
///   reassigned inside `platform_is :windows do ... end` blocks need
///   VariableForce-style branching analysis.
/// - 286 FNs from VariableForce scope tracking gaps.
///
/// ## Investigation (FP=41, FN=1059, 2026-03-20)
///
/// **FN fix: iterator block assignments (major FN source)**
/// `var_value_reaches_example_scope_in_stmts` walked top-level statements
/// linearly and, upon finding the statement containing the assignment offset,
/// `continue`d to the next sibling. When the assignment was inside a non-RSpec
/// block (e.g., `.each do |v| val = v; context ... do it ... end end`), both
/// the assignment and the example scopes were inside the same statement — so
/// skipping to the next sibling missed all references. Fix: fall through to
/// `stmt_example_scope_var_interaction` on the containing statement instead
/// of `continue`-ing past it. This addresses the dominant FN pattern (puppet,
/// datadog, sensu repos using `.each` for parameterized specs).
///
/// **Remaining gaps:**
/// - FP=~41: rswag DSL, platform_is conditional reassignment, file-level
///   variables reassigned conditionally in hooks. All require VariableForce-
///   level flow analysis.
/// - FN: VariableForce's comprehensive scope tracking across all Ruby scope
///   boundaries. Our AST-walking heuristics handle common patterns but can't
///   replicate VariableForce's full dataflow analysis. A complete fix would
///   require implementing VariableForce in Rust.
///
/// ## Investigation (FP=3, FN=77, 2026-03-21)
///
/// FP=3: All from jruby which lacks rubocop-rspec in Gemfile (same infra
/// issue as ScatteredLet/que-rb). RuboCop skips RSpec cops when the plugin
/// isn't installed; nitrocop runs them because they're compiled in.
/// Not a cop logic bug — nitrocop is correct for actual RSpec files.
///
/// FN fixes (3 root causes):
/// 1. **ConstantPathNode in `node_references_var`**: `result::Success` was
///    not handled. When a variable is used as the parent of a constant path
///    (e.g., `describe result::Success`), the `ConstantPathNode` fell through
///    to `false`. Fixed by recursing into `cp.parent()`.
/// 2. **If-condition assignments**: `if error = spec['error']` embeds a
///    `LocalVariableWriteNode` in the `IfNode.predicate()`, which
///    `collect_assignments_in_scope` and `collect_file_level_assignments`
///    did not check. Fixed by recursing into predicate for both functions.
/// 3. **`RSpec.describe` inside non-RSpec blocks**: `stmt_example_scope_var_interaction`
///    and `check_var_used_in_example_scopes` only matched `describe` with no
///    receiver (`no_recv`), missing `RSpec.describe` (receiver is `RSpec`).
///    Fixed by adding `is_rspec_recv` check alongside `no_recv`.
/// 4. **`LocalVariableWriteNode` in interaction/scope checks**: `spec = RSpec.describe ...`
///    wraps the example group call in an assignment node, which both
///    `stmt_example_scope_var_interaction` and `check_var_used_in_example_scopes`
///    did not recurse into. Fixed by handling `as_local_variable_write_node()`.
/// 5. **Example group arguments**: `describe result::Success do` uses a variable
///    as an argument to the example group call. Both `stmt_example_scope_var_interaction`
///    and `check_var_used_in_example_scopes` only recursed into the block body,
///    not the call arguments. Fixed by checking `call.arguments()`.
///
/// Remaining FN gap (~20): `def self.method` bodies containing `.each` with
/// `context`/`let` blocks (DataDog pattern). These create a separate Ruby
/// scope that our AST-walking approach doesn't enter. A full fix requires
/// VariableForce-level scope tracking.
///
/// ## Investigation (FP=34, FN=59, 2026-03-23)
///
/// **FP=34 fixed (two root causes):**
/// 1. **Describe/context arguments as example-scope reads** (~20 FPs):
///    `stmt_example_scope_var_interaction` and `check_var_used_in_example_scopes`
///    counted variable references in describe/context call arguments (e.g.,
///    `describe "#{v}" do`, `context "...", skip: flag do`) as example-scope
///    reads. Per RuboCop's source, `part_of_example_scope?` only matches
///    it/before/let/subject/include — NOT describe/context arguments, which
///    are evaluated at the group scope level. Repos affected: active_type (4),
///    chef (3), puppet (5), CocoaPods (2), vcr (2), elasticsearch (1),
///    imap-backup (1), activegraph (1), natalie (1), openproject (1).
/// 2. **File-level assignment collection entering shared groups** (~14 FPs):
///    `collect_file_level_assignments` recursed into shared_examples,
///    shared_examples_for, and shared_context blocks. Fixed by adding
///    `is_rspec_shared_group` check. Repos affected: forem (4), puppet (3),
///    vcr (2), openproject (2), natalie (1), activegraph (1), other (1).
///
/// **FN=59 remaining (multiple root causes, all require VariableForce):**
/// - `def self.method` with `.each`/`context` (DataDog: ~11 FN)
/// - Ruby 3.1 keyword shorthand `url:` not detected as variable reference
///   (stringer: ~4 FN)
/// - `def` methods inside describe blocks not entered by `check_def_level_vars`
///   (opal: ~3 FN)
/// - Conditional reassignment flow analysis gaps (fastlane: ~3 FN)
/// - Various other VariableForce scope-tracking gaps (~38 FN)
///
/// ## Fix (FN: ImplicitNode for Ruby 3.1 keyword shorthand, 2026-03-23)
///
/// `node_references_var` (and transitively `node_reads_var`) did not handle
/// `ImplicitNode`. In Prism, Ruby 3.1+ keyword shorthand `method(url:)`
/// (equivalent to `method(url: url)`) is represented as an `AssocNode`
/// where the value is an `ImplicitNode` wrapping a `LocalVariableReadNode`.
/// The AssocNode handler already recurses into key and value, but when the
/// value was an `ImplicitNode`, it fell through to `false` at the end of
/// `node_references_var`. Fix: added `as_implicit_node()` handler that
/// unwraps and recurses into the inner value. This fixes ~6 FNs (stringer,
/// gumroad, shoulda-matchers).
///
/// ## Remaining gaps (FP=3, FN=53 as of 2026-03-23)
///
/// **3 FP — infrastructure issues, not cop logic bugs:**
/// - SubjectStub FP=2: corpus oracle artifact. nitrocop is correct; RuboCop
///   1.85.1 + rubocop-rspec 3.9.0 also flags both lines (ubicloud). Oracle
///   missed them — will self-resolve on next corpus oracle run.
/// - ScatteredLet FP=1: que-rb uses minitest with minitest-hooks DSL that
///   looks like RSpec. RuboCop can't load rubocop-rspec from que-rb's bundle
///   so it skips RSpec cops. nitrocop has them compiled in. Fix requires
///   infrastructure: check project Gemfile.lock for plugin gem presence.
///
/// **53 FN — partially addressed:**
///
/// ## Fix (FN: def bodies inside example groups, 2026-03-24)
///
/// Added `check_defs_in_scope` to `check_scope_for_leaky_vars`. Previously,
/// `def`/`def self.method` nodes inside example group blocks were invisible:
/// `collect_assignments_in_scope` stops at def boundaries, and
/// `check_def_level_vars` only runs at file level. The new function
/// recursively finds def nodes inside example group block bodies and checks
/// their internal variables for leaks into example scopes (both via
/// `check_var_used_in_describe_blocks` for nested describe/context, and
/// `check_var_used_in_example_scopes` for direct it/before/let/subject).
///
/// Patterns fixed:
/// - `def self.define_cases` with `.each` + nested `context`/`it` (~11 FN,
///   DataDog/chef/yard/phony)
/// - `def run_test` with `RSpec.describe` inside wrapper blocks (~7 FN,
///   DataDog CI)
/// - `def self.it_is_correct_for` with direct `it` blocks (no wrapping
///   describe)
///
/// Remaining FN (~35):
/// - Conditional write kills (~3 FN, fastlane): `before` hook writes
///   variable inside `unless initialized` — flow analysis returns
///   `WriteBeforeRead` (killing the outer value), but the write is
///   conditional so the file-level value can still reach later `it` blocks.
/// - Block-local scoping edge cases (~32 FN): variables captured across
///   multiple nested blocks, rescue/ensure reassignment, lambda assignments,
///   etc. These require VariableForce-level dataflow analysis.
///
/// ## Investigation (FN: hook predicate reads before conditional writes, 2026-03-30)
///
/// Corpus cases from OpenVox initialized a variable to `nil` at group scope and
/// then used `if module_def.nil?; module_def = ...; end` inside a `before` hook.
/// Our deep-write heuristic treated that hook as `WriteBeforeRead`, killing the
/// outer assignment, because `node_reads_var_without_prior_write` checked the
/// branch body but ignored reads in the `if`/`unless` predicate. RuboCop's
/// VariableForce counts the predicate read as a reference to the outer value, so
/// the group-level `module_def = nil` is an offense. Fix: predicate reads now
/// count as reads-before-write in `node_reads_var_without_prior_write`.
///
/// ## Investigation (FN: embedded assignments in group/file expressions, 2026-03-30)
///
/// Corpus misses from cenit showed assignments nested inside group-level hash
/// literals on the RHS of another assignment:
/// `schema = { const: const_schema = {...}, (p_0 = :foo) => [p_1 = :bar] }`.
/// The old collectors only recorded assignments when the current node itself
/// was a write statement. They returned immediately for `schema = ...` and
/// never descended into the RHS/container expression tree, so `const_schema`,
/// `p_0`, and `p_1` were invisible. Fix: both assignment collectors now keep
/// walking assignment values plus common expression containers (hash/array/call
/// args, splats, interpolations, boolean/range nodes) while still stopping at
/// example-scope and Ruby-scope boundaries.
///
/// ## Investigation (FN: let/subject args and backtick interpolation, 2026-03-30)
///
/// Two remaining corpus clusters came from treating every "example scope" the
/// same:
/// - We skipped all arguments to `it`/`specify`/`before`/`let`/`subject`
///   together. RuboCop only exempts arguments to actual example methods like
///   `it` and `specify`; locals used in `let(html_options)` or `let(:foo, &bar)`
///   are still leaky references and must count.
/// - Backtick commands like `` `vagrant up #{insert_tee_log}` `` and
///   `` `tar -xf "#{test_tarball}"` `` are `InterpolatedXStringNode`s. Our
///   reference walker handled interpolated strings, symbols, and regexps, but
///   not xstrings, so reads inside command interpolation were invisible.
///
/// ## Investigation (FN: module/class wrapper scopes, 2026-03-31)
///
/// Corpus misses in `saml-idp` and `syntax_suggest` assign locals in a
/// `module` body before an `RSpec.describe` nested in that same Ruby scope:
/// `module X; payload = ...; RSpec.describe { it { payload } } end`.
/// `check_source` only analyzed file-level statements plus `def` bodies, so
/// locals in `module`/`class` bodies were invisible unless they lived inside a
/// method. Fix: reuse the existing outer-scope assignment analysis for
/// `module`/`class` statement lists before recursing into nested defs.
///
/// ## Fix (FP: dead group-scope assignments and operator-write kills, 2026-04-01)
///
/// Two root causes for 8 of 11 remaining FPs:
///
/// 1. **Group-scope reassignment not tracked in `var_value_reaches_example_scope_in_stmts`**:
///    When a variable is reassigned at the group scope level (unconditional write or
///    operator-writes like `x -= y`, `x += y`), the previous assignment's value is
///    replaced and should not be attributed to example-scope reads. Added
///    `stmt_reassigns_var` helper and a check before the example-scope interaction
///    analysis: if a later group-scope statement reassigns the variable, the tracked
///    value is dead. Fixes: leftovers (2 FP at lines 21-22), SlideHub (1 FP),
///    puppetlabs-docker line 43 (1 FP).
///
/// 2. **Non-RSpec block recursion lacking linear flow analysis**: The "other calls with
///    blocks" branch in `stmt_example_scope_var_interaction` combined all child statement
///    interactions without respecting that a later same-scope reassignment kills the
///    tracked value. When the block contains the tracked assignment, added linear flow
///    analysis: after the statement containing the assignment, a subsequent reassignment
///    (`stmt_reassigns_var`) means the tracked value is dead. Fixes: org-ruby (4 FP
///    where `textile_name = File.join(...)` was immediately overwritten by
///    `textile_name = File.expand_path(textile_name)` inside `.each` blocks).
///
/// Remaining FP (3):
/// - vets-api (1): rswag DSL `path/get` blocks collect variables at group scope.
///   Needs infrastructure fix to detect rswag DSL or VariableForce flow analysis.
/// - solargraph-rails (1): `filename = nil` at group scope, deep reassignment in
///   `it` block's nested `rails_workspace do |root| filename = ... end`. The per-
///   statement `has_outer_read` check doesn't propagate deep writes across
///   statements within an example scope. Would require cross-statement deep-write
///   tracking in the example-scope analysis.
/// - puppetlabs-docker run_spec (1): `facts = x; facts = facts.merge(y)` inside
///   an `if` branch within `.each`. Needs linear flow analysis within control flow
///   branches inside non-RSpec blocks (VariableForce territory).
pub struct LeakyLocalVariable;

impl Cop for LeakyLocalVariable {
    fn name(&self) -> &'static str {
        "RSpec/LeakyLocalVariable"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE]
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        check_file_level_vars(source, &parse_result.node(), diagnostics, self);
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        let is_example_group = if let Some(recv) = call.receiver() {
            util::constant_name(&recv).is_some_and(|n| n == b"RSpec")
                && is_rspec_example_group(method_name)
        } else {
            is_rspec_example_group(method_name)
        };

        if !is_example_group {
            return;
        }

        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        check_scope_for_leaky_vars(source, block_node, diagnostics, self);
    }
}

/// A local variable assignment found in the example-group scope.
struct VarAssign {
    name: Vec<u8>,
    offset: usize,
    /// Whether this is an unconditional write (`x = expr` or multi-write),
    /// as opposed to conditional/compound writes (`x ||= expr`, `x &&= expr`,
    /// `x += expr`). Used for dead assignment filtering.
    is_unconditional: bool,
    /// Whether this assignment is inside a non-RSpec block (e.g., `matcher`,
    /// `populate`, `.each`). Block-scoped assignments should only be checked
    /// against example scopes within the same containing statement.
    inside_block: bool,
}

/// Check for file-level variable assignments that leak into describe blocks.
/// This handles the case where variables are assigned before/outside the top-level
/// describe block and then referenced inside example scopes within it.
fn check_file_level_vars(
    source: &SourceFile,
    program: &ruby_prism::Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    cop: &LeakyLocalVariable,
) {
    let program_node = match program.as_program_node() {
        Some(p) => p,
        None => return,
    };
    let stmts = match program_node.statements().body().is_empty() {
        true => return,
        false => program_node.statements(),
    };

    // Collect file-level variable assignments (not inside describe blocks)
    let mut file_level_assigns: Vec<VarAssign> = Vec::new();
    for stmt in stmts.body().iter() {
        collect_file_level_assignments(&stmt, &mut file_level_assigns, false);
    }

    if !file_level_assigns.is_empty() {
        // Filter dead file-level assignments: if a variable is assigned multiple
        // times at file level and a later unconditional assignment exists with no
        // describe-block example-scope reference between them, the earlier
        // assignment is dead (its value never reaches any example).
        let live_assigns = filter_dead_file_level_assignments(&file_level_assigns, &stmts);

        // For each live file-level assignment, check if the variable is referenced
        // inside any example scope within any describe block in the file
        for assign in &live_assigns {
            let mut used = false;
            for stmt in stmts.body().iter() {
                if check_var_used_in_describe_blocks(&stmt, &assign.name) {
                    used = true;
                    break;
                }
            }
            if used {
                let (line, column) = source.offset_to_line_col(assign.offset);
                diagnostics.push(
                    cop.diagnostic(
                        source,
                        line,
                        column,
                        "Do not use local variables defined outside of examples inside of them."
                            .to_string(),
                    ),
                );
            }
        }
    }

    // Also check def/class/module bodies for variables that leak into describe blocks
    check_def_level_vars(source, &stmts, diagnostics, cop);
}

/// Check for variables inside `def` method bodies that leak into describe blocks.
/// RuboCop's VariableForce processes def scopes, finding variables assigned before
/// describe blocks that are referenced in example scopes within those describes.
fn check_def_level_vars(
    source: &SourceFile,
    stmts: &ruby_prism::StatementsNode<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    cop: &LeakyLocalVariable,
) {
    for stmt in stmts.body().iter() {
        check_def_level_vars_in_node(&stmt, source, diagnostics, cop);
    }
}

/// Recursively search for `def` nodes that contain describe blocks.
fn check_def_level_vars_in_node(
    node: &ruby_prism::Node<'_>,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    cop: &LeakyLocalVariable,
) {
    if let Some(def_node) = node.as_def_node() {
        if let Some(body) = def_node.body() {
            if let Some(stmts) = body.as_statements_node() {
                diagnose_outer_scope_var_leaks_in_stmts(source, &stmts, diagnostics, cop);
            }
        }
        return; // don't recurse into nested defs
    }

    // Class/module bodies are also Ruby local-variable scopes whose locals can
    // leak into nested describe/shared-context blocks.
    if let Some(class_node) = node.as_class_node() {
        if let Some(body) = class_node.body() {
            if let Some(stmts) = body.as_statements_node() {
                diagnose_outer_scope_var_leaks_in_stmts(source, &stmts, diagnostics, cop);
                for s in stmts.body().iter() {
                    check_def_level_vars_in_node(&s, source, diagnostics, cop);
                }
            }
        }
        return;
    }
    if let Some(module_node) = node.as_module_node() {
        if let Some(body) = module_node.body() {
            if let Some(stmts) = body.as_statements_node() {
                diagnose_outer_scope_var_leaks_in_stmts(source, &stmts, diagnostics, cop);
                for s in stmts.body().iter() {
                    check_def_level_vars_in_node(&s, source, diagnostics, cop);
                }
            }
        }
    }
}

/// Diagnose outer-scope local variables for a Ruby scope body (`def`, `module`,
/// or `class`) by reusing the same assignment collection and describe-block
/// reachability checks as file-level analysis.
fn diagnose_outer_scope_var_leaks_in_stmts(
    source: &SourceFile,
    stmts: &ruby_prism::StatementsNode<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    cop: &LeakyLocalVariable,
) {
    let mut assigns: Vec<VarAssign> = Vec::new();
    for s in stmts.body().iter() {
        collect_file_level_assignments(&s, &mut assigns, false);
    }
    if assigns.is_empty() {
        return;
    }

    let live = filter_dead_file_level_assignments(&assigns, stmts);
    for assign in &live {
        let mut used = false;
        for s in stmts.body().iter() {
            if check_var_used_in_describe_blocks(&s, &assign.name) {
                used = true;
                break;
            }
        }
        if used {
            let (line, column) = source.offset_to_line_col(assign.offset);
            diagnostics.push(
                cop.diagnostic(
                    source,
                    line,
                    column,
                    "Do not use local variables defined outside of examples inside of them."
                        .to_string(),
                ),
            );
        }
    }
}

/// Collect variable assignments at file level, stopping at describe blocks,
/// class/module definitions, and method definitions.
/// `in_conditional` is true when recursing inside if/elsif/else/unless branches,
/// making assignments there conditional (they may not execute at runtime).
fn collect_file_level_assignments(
    node: &ruby_prism::Node<'_>,
    assigns: &mut Vec<VarAssign>,
    in_conditional: bool,
) {
    // Direct assignment
    if let Some(lw) = node.as_local_variable_write_node() {
        assigns.push(VarAssign {
            name: lw.name().as_slice().to_vec(),
            offset: lw.location().start_offset(),
            is_unconditional: !in_conditional,
            inside_block: false,
        });
        collect_file_level_assignments(&lw.value(), assigns, in_conditional);
        return;
    }

    // or-write: `x ||= expr`
    if let Some(ow) = node.as_local_variable_or_write_node() {
        assigns.push(VarAssign {
            name: ow.name().as_slice().to_vec(),
            offset: ow.location().start_offset(),
            is_unconditional: false, // conditional write
            inside_block: false,
        });
        collect_file_level_assignments(&ow.value(), assigns, true);
        return;
    }

    // and-write: `x &&= expr`
    if let Some(aw) = node.as_local_variable_and_write_node() {
        assigns.push(VarAssign {
            name: aw.name().as_slice().to_vec(),
            offset: aw.location().start_offset(),
            is_unconditional: false, // conditional write
            inside_block: false,
        });
        collect_file_level_assignments(&aw.value(), assigns, true);
        return;
    }

    // operator-write: `x += expr`, `x -= expr`, etc.
    if let Some(ow) = node.as_local_variable_operator_write_node() {
        assigns.push(VarAssign {
            name: ow.name().as_slice().to_vec(),
            offset: ow.location().start_offset(),
            is_unconditional: false, // reads then writes
            inside_block: false,
        });
        collect_file_level_assignments(&ow.value(), assigns, true);
        return;
    }

    // Multi-write: `a, b = expr`
    if let Some(mw) = node.as_multi_write_node() {
        for target in mw.lefts().iter() {
            if let Some(lt) = target.as_local_variable_target_node() {
                assigns.push(VarAssign {
                    name: lt.name().as_slice().to_vec(),
                    offset: lt.location().start_offset(),
                    is_unconditional: !in_conditional,
                    inside_block: false,
                });
            }
        }
        collect_file_level_assignments(&mw.value(), assigns, in_conditional);
        return;
    }

    // Stop at describe blocks, example scopes, classes, modules, defs
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        let is_rspec_recv = call
            .receiver()
            .is_some_and(|r| util::constant_name(&r).is_some_and(|n| n == b"RSpec"));
        let no_recv = call.receiver().is_none() || is_rspec_recv;
        if no_recv && is_rspec_example_group(name) {
            return;
        }
        // Stop at shared example groups (shared_examples, shared_examples_for,
        // shared_context). Variables inside shared groups are scoped to the
        // shared group block, not file-level. RuboCop's VariableForce respects
        // block scope boundaries, so variables inside shared groups don't leak
        // to the file level.
        if no_recv && util::is_rspec_shared_group(name) {
            return;
        }
        // Stop at example scopes (it, before, let, subject, etc.)
        // Variables assigned inside example scopes are not file-level leaks.
        if call.receiver().is_none() && (is_example_scope(name) || is_includes_method(name)) {
            return;
        }
        if let Some(recv) = call.receiver() {
            collect_file_level_assignments(&recv, assigns, in_conditional);
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                collect_file_level_assignments(&arg, assigns, in_conditional);
            }
        }
        // For other calls (e.g., iterators), recurse into block body
        if let Some(blk) = call.block() {
            if let Some(bn) = blk.as_block_node() {
                if let Some(body) = bn.body() {
                    if let Some(stmts) = body.as_statements_node() {
                        for s in stmts.body().iter() {
                            collect_file_level_assignments(&s, assigns, in_conditional);
                        }
                    }
                }
            }
        }
        return;
    }

    if let Some(hash) = node.as_hash_node() {
        for elem in hash.elements().iter() {
            collect_file_level_assignments(&elem, assigns, in_conditional);
        }
        return;
    }

    if let Some(hash) = node.as_keyword_hash_node() {
        for elem in hash.elements().iter() {
            collect_file_level_assignments(&elem, assigns, in_conditional);
        }
        return;
    }

    if let Some(assoc) = node.as_assoc_node() {
        collect_file_level_assignments(&assoc.key(), assigns, in_conditional);
        collect_file_level_assignments(&assoc.value(), assigns, in_conditional);
        return;
    }

    if let Some(assoc_splat) = node.as_assoc_splat_node() {
        if let Some(expr) = assoc_splat.value() {
            collect_file_level_assignments(&expr, assigns, in_conditional);
        }
        return;
    }

    if let Some(arr) = node.as_array_node() {
        for elem in arr.elements().iter() {
            collect_file_level_assignments(&elem, assigns, in_conditional);
        }
        return;
    }

    if let Some(splat) = node.as_splat_node() {
        if let Some(expr) = splat.expression() {
            collect_file_level_assignments(&expr, assigns, in_conditional);
        }
        return;
    }

    if let Some(ret) = node.as_return_node() {
        if let Some(args) = ret.arguments() {
            for arg in args.arguments().iter() {
                collect_file_level_assignments(&arg, assigns, in_conditional);
            }
        }
        return;
    }

    if let Some(and_node) = node.as_and_node() {
        collect_file_level_assignments(&and_node.left(), assigns, true);
        collect_file_level_assignments(&and_node.right(), assigns, true);
        return;
    }

    if let Some(or_node) = node.as_or_node() {
        collect_file_level_assignments(&or_node.left(), assigns, true);
        collect_file_level_assignments(&or_node.right(), assigns, true);
        return;
    }

    if let Some(range) = node.as_range_node() {
        if let Some(left) = range.left() {
            collect_file_level_assignments(&left, assigns, in_conditional);
        }
        if let Some(right) = range.right() {
            collect_file_level_assignments(&right, assigns, in_conditional);
        }
        return;
    }

    if let Some(embedded) = node.as_embedded_statements_node() {
        if let Some(stmts) = embedded.statements() {
            for s in stmts.body().iter() {
                collect_file_level_assignments(&s, assigns, in_conditional);
            }
        }
        return;
    }

    if let Some(interp) = node.as_interpolated_string_node() {
        for part in interp.parts().iter() {
            collect_file_level_assignments(&part, assigns, in_conditional);
        }
        return;
    }

    if let Some(interp) = node.as_interpolated_symbol_node() {
        for part in interp.parts().iter() {
            collect_file_level_assignments(&part, assigns, in_conditional);
        }
        return;
    }

    if let Some(interp) = node.as_interpolated_regular_expression_node() {
        for part in interp.parts().iter() {
            collect_file_level_assignments(&part, assigns, in_conditional);
        }
        return;
    }

    // Recurse through control flow — assignments inside if/elsif/else are conditional
    // Also check predicate for embedded assignments (e.g., `if error = expr`)
    if let Some(if_node) = node.as_if_node() {
        collect_file_level_assignments(&if_node.predicate(), assigns, true);
        if let Some(stmts) = if_node.statements() {
            for s in stmts.body().iter() {
                collect_file_level_assignments(&s, assigns, true);
            }
        }
        if let Some(subsequent) = if_node.subsequent() {
            collect_file_level_assignments(&subsequent, assigns, true);
        }
        return;
    }

    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            for s in stmts.body().iter() {
                collect_file_level_assignments(&s, assigns, in_conditional);
            }
        }
    }

    // ElseNode (from if/elsif/else chain)
    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            for s in stmts.body().iter() {
                collect_file_level_assignments(&s, assigns, true);
            }
        }
        return;
    }

    // UnlessNode
    if let Some(unless_node) = node.as_unless_node() {
        collect_file_level_assignments(&unless_node.predicate(), assigns, true);
        if let Some(stmts) = unless_node.statements() {
            for s in stmts.body().iter() {
                collect_file_level_assignments(&s, assigns, true);
            }
        }
        if let Some(else_clause) = unless_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    collect_file_level_assignments(&s, assigns, true);
                }
            }
        }
        return;
    }

    // CaseNode
    if let Some(case_node) = node.as_case_node() {
        for cond in case_node.conditions().iter() {
            if let Some(when_node) = cond.as_when_node() {
                if let Some(stmts) = when_node.statements() {
                    for s in stmts.body().iter() {
                        collect_file_level_assignments(&s, assigns, true);
                    }
                }
            }
        }
        if let Some(else_clause) = case_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    collect_file_level_assignments(&s, assigns, true);
                }
            }
        }
    }

    // Stop at class/module/def
}

/// Check if a variable is referenced inside any example scope within describe
/// blocks found in the given node tree.
fn check_var_used_in_describe_blocks(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        let is_eg = if let Some(recv) = call.receiver() {
            util::constant_name(&recv).is_some_and(|n| n == b"RSpec")
                && is_rspec_example_group(name)
        } else {
            is_rspec_example_group(name)
        };

        if is_eg {
            // Found a describe block — use flow-aware analysis to check if the
            // file-level variable's value reaches any example scope.
            //
            // First check if the variable is unconditionally reassigned at the
            // group's top-level scope before any example scope reference (the
            // file-level value is dead). Then use linear flow analysis across
            // example scopes: if an example scope unconditionally reassigns the
            // variable before reading it (WriteBeforeRead), subsequent example
            // scopes' reads belong to that example-scope assignment, not the
            // file-level one (fastlane pattern: after(:all) kills the value).
            if let Some(blk) = call.block() {
                if let Some(bn) = blk.as_block_node() {
                    if let Some(body) = bn.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            if var_reassigned_before_example_ref_in_stmts(&stmts, var_name) {
                                return false;
                            }
                            // Flow-aware check: track whether the file-level
                            // value has been killed by an example-scope write.
                            let mut value_killed = false;
                            for s in stmts.body().iter() {
                                match stmt_example_scope_var_interaction(&s, var_name, 0) {
                                    VarInteraction::ReadOnly => {
                                        if !value_killed {
                                            return true;
                                        }
                                    }
                                    VarInteraction::WriteBeforeRead => {
                                        value_killed = true;
                                    }
                                    VarInteraction::WriteAndReadBeforeWrite => {
                                        if !value_killed {
                                            return true;
                                        }
                                        value_killed = true;
                                    }
                                    VarInteraction::None => {}
                                }
                            }
                        }
                    }
                }
            }
            return false;
        }

        // For other calls with blocks, recurse (respect block param shadowing)
        if let Some(blk) = call.block() {
            if let Some(bn) = blk.as_block_node() {
                if !block_has_param(&bn, var_name) {
                    if let Some(body) = bn.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            for s in stmts.body().iter() {
                                if check_var_used_in_describe_blocks(&s, var_name) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        return false;
    }

    // Recurse through control flow
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            for s in stmts.body().iter() {
                if check_var_used_in_describe_blocks(&s, var_name) {
                    return true;
                }
            }
        }
        if let Some(subsequent) = if_node.subsequent() {
            if check_var_used_in_describe_blocks(&subsequent, var_name) {
                return true;
            }
        }
        return false;
    }

    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            for s in stmts.body().iter() {
                if check_var_used_in_describe_blocks(&s, var_name) {
                    return true;
                }
            }
        }
        return false;
    }

    false
}

/// Check an example group block for leaky local variables.
fn check_scope_for_leaky_vars(
    source: &SourceFile,
    block: ruby_prism::BlockNode<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    cop: &LeakyLocalVariable,
) {
    let body = match block.body() {
        Some(b) => b,
        None => return,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return,
    };

    // Collect all local variable assignments in this scope (recursively through
    // non-scope-boundary nodes like if/unless/case/begin, but stopping at
    // example scopes and nested example groups).
    let mut assignments: Vec<VarAssign> = Vec::new();
    for stmt in stmts.body().iter() {
        collect_assignments_in_scope(&stmt, &mut assignments, false);
    }

    // Filter out dead assignments: if a variable is unconditionally reassigned
    // at the same scope level before any example scope reads it, the earlier
    // assignment is dead (its value is never observed by examples).
    let live_assignments = filter_dead_assignments(&assignments, &stmts);

    // For each live assignment, check if the variable is referenced inside any
    // example scope within this block. Use the scope-aware check that handles
    // reassignment in nested example groups. Also applies flow-aware dead-value
    // analysis: if the variable is unconditionally reassigned in an example scope
    // (e.g., a before hook or an it block), subsequent example-scope reads belong
    // to the example-scope assignment, not the group-level one.
    for assign in &live_assignments {
        let used_in_example_scope = if assign.inside_block {
            // For assignments inside non-RSpec blocks (matcher, populate, control),
            // only check if the variable leaks within the containing statement.
            var_value_reaches_example_scope_in_containing_stmt(&stmts, &assign.name, assign.offset)
        } else {
            var_value_reaches_example_scope_in_stmts(&stmts, &assign.name, assign.offset)
        };

        if used_in_example_scope {
            let (line, column) = source.offset_to_line_col(assign.offset);
            diagnostics.push(
                cop.diagnostic(
                    source,
                    line,
                    column,
                    "Do not use local variables defined outside of examples inside of them."
                        .to_string(),
                ),
            );
        }
    }

    // Also check def/def self.method bodies inside this example group block
    // for variables that leak into example scopes within the def body.
    check_defs_in_scope(source, &stmts, diagnostics, cop);
}

/// Find `def` and `def self.method` nodes inside an example group block and
/// check their bodies for variables that leak into example scopes.
///
/// This handles patterns like:
/// ```ruby
/// describe "dynamic" do
///   def self.define_cases(items)
///     items.each do |label, value|
///       result = value.upcase         # assignment inside def body
///       context label do
///         it { expect(x).to eq(result) }  # leaks into example scope
///       end
///     end
///   end
/// end
/// ```
fn check_defs_in_scope(
    source: &SourceFile,
    stmts: &ruby_prism::StatementsNode<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    cop: &LeakyLocalVariable,
) {
    for stmt in stmts.body().iter() {
        find_and_check_defs_in_node(&stmt, source, diagnostics, cop);
    }
}

/// Recursively search for `def` nodes within example group block bodies.
fn find_and_check_defs_in_node(
    node: &ruby_prism::Node<'_>,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    cop: &LeakyLocalVariable,
) {
    // Check for def node (handles both `def foo` and `def self.foo`)
    if let Some(def_node) = node.as_def_node() {
        check_def_body_for_leaky_vars(def_node, source, diagnostics, cop);
        return; // don't recurse into nested defs
    }

    // Recurse into call blocks (e.g., other method calls wrapping defs)
    if let Some(call) = node.as_call_node() {
        if let Some(blk) = call.block() {
            if let Some(bn) = blk.as_block_node() {
                if let Some(body) = bn.body() {
                    if let Some(stmts) = body.as_statements_node() {
                        for s in stmts.body().iter() {
                            find_and_check_defs_in_node(&s, source, diagnostics, cop);
                        }
                    }
                }
            }
        }
        return;
    }

    // Recurse into control flow
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            for s in stmts.body().iter() {
                find_and_check_defs_in_node(&s, source, diagnostics, cop);
            }
        }
        if let Some(subsequent) = if_node.subsequent() {
            find_and_check_defs_in_node(&subsequent, source, diagnostics, cop);
        }
        return;
    }

    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            for s in stmts.body().iter() {
                find_and_check_defs_in_node(&s, source, diagnostics, cop);
            }
        }
    }
}

/// Check a single def node's body for variables that leak into example scopes.
fn check_def_body_for_leaky_vars(
    def_node: ruby_prism::DefNode<'_>,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    cop: &LeakyLocalVariable,
) {
    let body = match def_node.body() {
        Some(b) => b,
        None => return,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return,
    };

    // Collect assignments in the def body (stopping at describe blocks)
    let mut assigns: Vec<VarAssign> = Vec::new();
    for s in stmts.body().iter() {
        collect_file_level_assignments(&s, &mut assigns, false);
    }
    if assigns.is_empty() {
        return;
    }

    let live = filter_dead_file_level_assignments(&assigns, &stmts);
    for assign in &live {
        // Check if the variable is used in describe blocks within the def body
        let mut used = false;
        for s in stmts.body().iter() {
            if check_var_used_in_describe_blocks(&s, &assign.name) {
                used = true;
                break;
            }
        }

        // Also check for direct example scopes (it, before, let, subject)
        // in the def body — handles cases like `def self.it_is_correct_for`
        // where example scopes appear directly without a wrapping describe block.
        if !used {
            for s in stmts.body().iter() {
                if check_var_used_in_example_scopes(&s, &assign.name) {
                    used = true;
                    break;
                }
            }
        }

        if used {
            let (line, column) = source.offset_to_line_col(assign.offset);
            diagnostics.push(
                cop.diagnostic(
                    source,
                    line,
                    column,
                    "Do not use local variables defined outside of examples inside of them."
                        .to_string(),
                ),
            );
        }
    }
}

/// Flow-aware check: does the group-level assignment's value actually reach any
/// example scope? Walks statements linearly starting after `assign_offset`,
/// tracking whether the value is still "live." The value becomes dead when the
/// variable is unconditionally reassigned in an example scope (before/after/
/// around/it/let/subject) — subsequent example-scope reads belong to that
/// example-scope assignment, not the group-level one.
///
/// This matches RuboCop's VariableForce behavior: it tracks per-assignment
/// references using linear dataflow, so if a hook reassigns the variable before
/// any example reads it, the group-level assignment has zero references in
/// example scopes.
fn var_value_reaches_example_scope_in_stmts(
    stmts: &ruby_prism::StatementsNode<'_>,
    var_name: &[u8],
    assign_offset: usize,
) -> bool {
    let mut past_assignment = false;
    // Track whether the group-level value has been "killed" by an example-scope
    // reassignment. Once killed, subsequent example-scope reads don't count.
    let mut value_killed = false;

    for stmt in stmts.body().iter() {
        if !past_assignment {
            if stmt_contains_offset(&stmt, assign_offset) {
                past_assignment = true;
                // Fall through to check this statement too — it may contain
                // example scopes after the assignment (e.g., .each blocks
                // with nested context/it inside the same block).
            } else {
                continue;
            }
        }

        // Group-scope reassignment: if a later statement at the same scope
        // level reassigns the variable (unconditional write, operator-write,
        // or-write, and-write), the tracked assignment's value is replaced.
        // The new value may or may not reach example scopes, but that's tracked
        // by a separate VarAssign entry — not this one.
        // Only check statements AFTER the one containing the assignment.
        if !stmt_contains_offset(&stmt, assign_offset) && stmt_reassigns_var(&stmt, var_name) {
            return false;
        }

        // Check this statement for example-scope interactions with the variable.
        // We need to distinguish between:
        // (a) example scopes that reassign the variable before reading it (kills value)
        // (b) example scopes that read the variable (uses value, if not killed)
        // (c) nested example groups (recurse with the same flow logic)
        match stmt_example_scope_var_interaction(&stmt, var_name, assign_offset) {
            VarInteraction::ReadOnly => {
                if !value_killed {
                    return true;
                }
            }
            VarInteraction::WriteBeforeRead => {
                // This example scope reassigns the variable before reading it.
                // In RuboCop's VariableForce linear flow, this kills the group-level
                // value for all subsequent scopes.
                value_killed = true;
            }
            VarInteraction::WriteAndReadBeforeWrite => {
                // The scope both reads then writes, or reads the group-level value.
                if !value_killed {
                    return true;
                }
                value_killed = true;
            }
            VarInteraction::None => {}
        }
    }

    false
}

/// For assignments inside non-RSpec blocks (e.g., `matcher`, `populate`, `control`),
/// check only the containing statement for example-scope references. Block-local
/// variables only leak if the containing block itself has example scopes that read
/// the variable.
fn var_value_reaches_example_scope_in_containing_stmt(
    stmts: &ruby_prism::StatementsNode<'_>,
    var_name: &[u8],
    assign_offset: usize,
) -> bool {
    for stmt in stmts.body().iter() {
        if stmt_contains_offset(&stmt, assign_offset) {
            match stmt_example_scope_var_interaction(&stmt, var_name, assign_offset) {
                VarInteraction::ReadOnly | VarInteraction::WriteAndReadBeforeWrite => return true,
                _ => return false,
            }
        }
    }
    false
}

/// How an example scope interacts with a variable.
enum VarInteraction {
    /// No reference to the variable in this statement's example scopes.
    None,
    /// The variable is read in an example scope without prior reassignment.
    ReadOnly,
    /// The variable is unconditionally reassigned in an example scope before
    /// being read (e.g., `before { result = compute() }`).
    WriteBeforeRead,
    /// The variable is read in an example scope AND also written, but the read
    /// happens before the write (or both happen).
    WriteAndReadBeforeWrite,
}

/// Analyze how a statement's example scopes interact with a variable.
/// Returns the combined interaction across all example scopes in the statement.
///
/// `assign_offset` is the byte offset of the assignment we're tracking. When
/// recursing into non-RSpec blocks, we skip blocks that don't contain the
/// assignment AND have their own local assignment to the same variable name,
/// because such blocks create a new local binding that shadows the outer one.
fn stmt_example_scope_var_interaction(
    node: &ruby_prism::Node<'_>,
    var_name: &[u8],
    assign_offset: usize,
) -> VarInteraction {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        let no_recv = call.receiver().is_none();
        let is_rspec_recv = call
            .receiver()
            .is_some_and(|r| util::constant_name(&r).is_some_and(|n| n == b"RSpec"));

        // Example scopes: it, before, let, subject, etc.
        if no_recv && is_example_scope(name) {
            // Only actual example methods (`it`, `specify`, etc.) get the
            // "arguments are allowed" exemption for descriptions/metadata.
            // Args to hooks/lets/subjects are part of the leaky reference.
            let mut result = if !is_rspec_example(name) {
                if let Some(args) = call.arguments() {
                    if args
                        .arguments()
                        .iter()
                        .any(|arg| node_references_var(&arg, var_name))
                    {
                        VarInteraction::ReadOnly
                    } else {
                        VarInteraction::None
                    }
                } else {
                    VarInteraction::None
                }
            } else {
                VarInteraction::None
            };

            if let Some(blk) = call.block() {
                if let Some(bn) = blk.as_block_node() {
                    if block_has_param(&bn, var_name) {
                        return result; // shadowed in body, args (if any) already counted
                    }
                    if let Some(body) = bn.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            if var_written_before_read_in_stmts(&stmts, var_name) {
                                return combine_var_interactions(
                                    result,
                                    VarInteraction::WriteBeforeRead,
                                );
                            }
                            // Deep write check: the block may write the
                            // variable inside a nested call (e.g., `expect do
                            // response = ... end`) or inside a conditional
                            // (e.g., `unless cond; x = new_val; use(x); end`).
                            // A write without a prior read of the outer value
                            // means the outer value is dead for this scope.
                            let has_deep_write = stmts
                                .body()
                                .iter()
                                .any(|s| node_writes_var_deep(&s, var_name));
                            if has_deep_write {
                                // Check if there are any reads that are NOT
                                // preceded by a write within the same branch.
                                // If all reads appear after writes in the same
                                // conditional, the outer value is dead.
                                let has_outer_read = stmts
                                    .body()
                                    .iter()
                                    .any(|s| node_reads_var_without_prior_write(&s, var_name));
                                if !has_outer_read {
                                    return combine_var_interactions(
                                        result,
                                        VarInteraction::WriteBeforeRead,
                                    );
                                }
                                return combine_var_interactions(
                                    result,
                                    VarInteraction::WriteAndReadBeforeWrite,
                                );
                            }
                            // Check if the variable is referenced at all
                            let has_read = stmts
                                .body()
                                .iter()
                                .any(|s| node_references_var(&s, var_name));
                            if has_read {
                                result = combine_var_interactions(result, VarInteraction::ReadOnly);
                            }
                        }
                    }
                }
            }
            return result;
        }

        // Includes methods: it_behaves_like, include_examples, etc.
        if no_recv && is_includes_method(name) {
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                for (i, arg) in arg_list.iter().enumerate() {
                    if i == 0 {
                        continue;
                    }
                    if is_interpolated_string_or_symbol(arg) {
                        continue;
                    }
                    if node_references_var(arg, var_name) {
                        return VarInteraction::ReadOnly;
                    }
                }
            }
            if let Some(blk) = call.block() {
                if let Some(bn) = blk.as_block_node() {
                    if !block_has_param(&bn, var_name) {
                        if let Some(body) = bn.body() {
                            if let Some(stmts) = body.as_statements_node() {
                                // Recurse with full example-scope analysis so
                                // that nested includes methods (include_context
                                // with first-arg exclusion) are handled properly.
                                let mut result = VarInteraction::None;
                                for s in stmts.body().iter() {
                                    let inner = stmt_example_scope_var_interaction(
                                        &s,
                                        var_name,
                                        assign_offset,
                                    );
                                    result = combine_var_interactions(result, inner);
                                }
                                if !matches!(result, VarInteraction::None) {
                                    return result;
                                }
                            }
                        }
                    }
                }
            }
            return VarInteraction::None;
        }

        // Nested example groups: recurse into their statements
        // Match both `describe` (no receiver) and `RSpec.describe` (receiver is RSpec)
        if (no_recv || is_rspec_recv) && is_rspec_example_group(name) {
            // Note: describe/context ARGUMENTS (e.g., `describe "#{v}" do`) are
            // evaluated at the group scope, not inside example scopes. RuboCop's
            // LeakyLocalVariable cop checks `part_of_example_scope?` which only
            // matches it/before/let/subject/include — NOT describe/context call
            // arguments. So we do NOT check call.arguments() here.
            if let Some(blk) = call.block() {
                if let Some(bn) = blk.as_block_node() {
                    if let Some(body) = bn.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            // Check if the variable is reassigned at the nested
                            // group's scope level before any example reads it
                            if var_reassigned_before_example_ref_in_stmts(&stmts, var_name) {
                                return VarInteraction::None;
                            }
                            // Recurse with flow-aware analysis: track whether
                            // a hook's write kills the outer value before any
                            // example reads it. This matches the flow analysis
                            // in check_var_used_in_describe_blocks.
                            let mut value_killed = false;
                            for s in stmts.body().iter() {
                                let inner =
                                    stmt_example_scope_var_interaction(&s, var_name, assign_offset);
                                match inner {
                                    VarInteraction::ReadOnly => {
                                        if !value_killed {
                                            return VarInteraction::ReadOnly;
                                        }
                                    }
                                    VarInteraction::WriteBeforeRead => {
                                        value_killed = true;
                                    }
                                    VarInteraction::WriteAndReadBeforeWrite => {
                                        if !value_killed {
                                            return VarInteraction::WriteAndReadBeforeWrite;
                                        }
                                        value_killed = true;
                                    }
                                    VarInteraction::None => {}
                                }
                            }
                            // If all reads were killed, report the write (if any).
                            return if value_killed {
                                VarInteraction::WriteBeforeRead
                            } else {
                                VarInteraction::None
                            };
                        }
                    }
                }
            }
            return VarInteraction::None;
        }

        // Other calls with blocks: recurse into block body, respecting block param shadowing
        // and Ruby's block-local variable scoping.
        if let Some(blk) = call.block() {
            if let Some(bn) = blk.as_block_node() {
                let block_contains_assignment = stmt_contains_offset(node, assign_offset);
                if block_has_param(&bn, var_name) {
                    // If the tracked assignment is inside this block (i.e., a
                    // reassignment of the block param like `k = k.to_s`), we
                    // must still recurse to find example-scope references.
                    // If the assignment is NOT inside this block, the block
                    // param shadows the outer variable — skip it.
                    if !block_contains_assignment {
                        return VarInteraction::None; // shadowed by block param
                    }
                }
                // Ruby block scoping: if this block does NOT contain the
                // assignment we're tracking but has its own local assignment
                // to the same variable name, then references inside this
                // block refer to the block-local variable, not ours. Skip it.
                // This handles the discourse/rswag pattern where sibling
                // blocks (get/post/put) each have their own local copy of
                // a variable like `expected_request_schema`.
                if !block_contains_assignment && block_body_assigns_var(&bn, var_name) {
                    return VarInteraction::None;
                }
                if let Some(body) = bn.body() {
                    if let Some(stmts) = body.as_statements_node() {
                        if block_contains_assignment {
                            // When this block contains the tracked assignment,
                            // do linear flow analysis: a later reassignment of
                            // the same variable at the same block scope level
                            // kills the tracked value, so subsequent example-
                            // scope reads reference the new value, not ours.
                            // This handles the org-ruby pattern:
                            //   files.each do |file|
                            //     name = File.join(...)    # tracked
                            //     name = File.expand_path(name)  # kills tracked
                            //     it { IO.read(name) }     # reads new value
                            //   end
                            let mut past_assign = false;
                            let mut result = VarInteraction::None;
                            for s in stmts.body().iter() {
                                if !past_assign {
                                    if stmt_contains_offset(&s, assign_offset) {
                                        past_assign = true;
                                        // Check this statement itself for example
                                        // scope interactions (the assignment and
                                        // example scopes might be in the same stmt)
                                        let inner = stmt_example_scope_var_interaction(
                                            &s,
                                            var_name,
                                            assign_offset,
                                        );
                                        result = combine_var_interactions(result, inner);
                                    }
                                    continue;
                                }
                                // After the tracked assignment: if this statement
                                // reassigns the variable (unconditional write or
                                // operator-write), the tracked value is dead.
                                if stmt_reassigns_var(&s, var_name) {
                                    return result;
                                }
                                let inner =
                                    stmt_example_scope_var_interaction(&s, var_name, assign_offset);
                                result = combine_var_interactions(result, inner);
                            }
                            return result;
                        } else {
                            let mut result = VarInteraction::None;
                            for s in stmts.body().iter() {
                                let inner =
                                    stmt_example_scope_var_interaction(&s, var_name, assign_offset);
                                result = combine_var_interactions(result, inner);
                            }
                            return result;
                        }
                    }
                }
            }
        }
        return VarInteraction::None;
    }

    // Recurse through control flow
    if let Some(if_node) = node.as_if_node() {
        let mut result = VarInteraction::None;
        if let Some(stmts) = if_node.statements() {
            for s in stmts.body().iter() {
                let inner = stmt_example_scope_var_interaction(&s, var_name, assign_offset);
                result = combine_var_interactions(result, inner);
            }
        }
        if let Some(subsequent) = if_node.subsequent() {
            let inner = stmt_example_scope_var_interaction(&subsequent, var_name, assign_offset);
            result = combine_var_interactions(result, inner);
        }
        return result;
    }
    if let Some(unless_node) = node.as_unless_node() {
        let mut result = VarInteraction::None;
        if let Some(stmts) = unless_node.statements() {
            for s in stmts.body().iter() {
                let inner = stmt_example_scope_var_interaction(&s, var_name, assign_offset);
                result = combine_var_interactions(result, inner);
            }
        }
        if let Some(else_clause) = unless_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    let inner = stmt_example_scope_var_interaction(&s, var_name, assign_offset);
                    result = combine_var_interactions(result, inner);
                }
            }
        }
        return result;
    }
    if let Some(else_node) = node.as_else_node() {
        let mut result = VarInteraction::None;
        if let Some(stmts) = else_node.statements() {
            for s in stmts.body().iter() {
                let inner = stmt_example_scope_var_interaction(&s, var_name, assign_offset);
                result = combine_var_interactions(result, inner);
            }
        }
        return result;
    }
    if let Some(case_node) = node.as_case_node() {
        let mut result = VarInteraction::None;
        for cond in case_node.conditions().iter() {
            if let Some(when_node) = cond.as_when_node() {
                if let Some(stmts) = when_node.statements() {
                    for s in stmts.body().iter() {
                        let inner = stmt_example_scope_var_interaction(&s, var_name, assign_offset);
                        result = combine_var_interactions(result, inner);
                    }
                }
            }
        }
        if let Some(else_clause) = case_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    let inner = stmt_example_scope_var_interaction(&s, var_name, assign_offset);
                    result = combine_var_interactions(result, inner);
                }
            }
        }
        return result;
    }
    if let Some(begin_node) = node.as_begin_node() {
        let mut result = VarInteraction::None;
        if let Some(stmts) = begin_node.statements() {
            for s in stmts.body().iter() {
                let inner = stmt_example_scope_var_interaction(&s, var_name, assign_offset);
                result = combine_var_interactions(result, inner);
            }
        }
        if let Some(rescue_clause) = begin_node.rescue_clause() {
            let inner = rescue_var_interaction(&rescue_clause, var_name, assign_offset);
            result = combine_var_interactions(result, inner);
        }
        if let Some(else_clause) = begin_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    let inner = stmt_example_scope_var_interaction(&s, var_name, assign_offset);
                    result = combine_var_interactions(result, inner);
                }
            }
        }
        if let Some(ensure_clause) = begin_node.ensure_clause() {
            if let Some(stmts) = ensure_clause.statements() {
                for s in stmts.body().iter() {
                    let inner = stmt_example_scope_var_interaction(&s, var_name, assign_offset);
                    result = combine_var_interactions(result, inner);
                }
            }
        }
        return result;
    }
    if let Some(paren) = node.as_parentheses_node() {
        if let Some(body) = paren.body() {
            return stmt_example_scope_var_interaction(&body, var_name, assign_offset);
        }
    }

    // Local variable write: the RHS may contain example scopes
    // e.g., `spec = RSpec.describe "SomeTest" do ... end`
    if let Some(lw) = node.as_local_variable_write_node() {
        return stmt_example_scope_var_interaction(&lw.value(), var_name, assign_offset);
    }

    VarInteraction::None
}

/// Check rescue chain for variable interaction.
fn rescue_var_interaction(
    rescue_node: &ruby_prism::RescueNode<'_>,
    var_name: &[u8],
    assign_offset: usize,
) -> VarInteraction {
    let mut result = VarInteraction::None;
    if let Some(stmts) = rescue_node.statements() {
        for s in stmts.body().iter() {
            let inner = stmt_example_scope_var_interaction(&s, var_name, assign_offset);
            result = combine_var_interactions(result, inner);
        }
    }
    if let Some(subsequent) = rescue_node.subsequent() {
        let inner = rescue_var_interaction(&subsequent, var_name, assign_offset);
        result = combine_var_interactions(result, inner);
    }
    result
}

/// Combine two VarInteraction values. A read anywhere dominates.
fn combine_var_interactions(a: VarInteraction, b: VarInteraction) -> VarInteraction {
    match (&a, &b) {
        // Any read makes the combined result a read
        (VarInteraction::ReadOnly, _) | (_, VarInteraction::ReadOnly) => VarInteraction::ReadOnly,
        (VarInteraction::WriteAndReadBeforeWrite, _)
        | (_, VarInteraction::WriteAndReadBeforeWrite) => VarInteraction::WriteAndReadBeforeWrite,
        // Write-before-read in at least one scope
        (VarInteraction::WriteBeforeRead, _) | (_, VarInteraction::WriteBeforeRead) => {
            VarInteraction::WriteBeforeRead
        }
        // Neither
        _ => VarInteraction::None,
    }
}

/// Filter out dead assignments. An assignment to variable X is dead if there's
/// a later unconditional assignment to X at the top-level statement list, and
/// no example-scope reference to X exists between the two assignments.
///
/// This implements a simplified version of RuboCop's VariableForce flow analysis
/// for the common case of sequential reassignment.
fn filter_dead_assignments<'a>(
    assignments: &'a [VarAssign],
    stmts: &ruby_prism::StatementsNode<'_>,
) -> Vec<&'a VarAssign> {
    if assignments.is_empty() {
        return Vec::new();
    }

    let mut live: Vec<&VarAssign> = Vec::new();

    for assign in assignments {
        if assign.inside_block {
            // Block-scoped assignments skip dead assignment filtering here.
            // Different blocks create separate Ruby variable scopes, making
            // offset-based dead assignment comparison unreliable. Instead,
            // the per-containing-statement check in check_scope_for_leaky_vars
            // handles block-local scoping correctly.
            live.push(assign);
        } else {
            // For top-level assignments, use the existing statement-tree-based check.
            if is_dead_assignment(assign, stmts) {
                continue;
            }
            live.push(assign);
        }
    }

    live
}

/// Check if an assignment is dead — overwritten by a later unconditional assignment
/// at the top-level statement list with no intervening example-scope reference.
fn is_dead_assignment(assign: &VarAssign, stmts: &ruby_prism::StatementsNode<'_>) -> bool {
    let mut past_current = false;
    let mut seen_example_ref = false;

    for stmt in stmts.body().iter() {
        // First, find the current assignment's position
        if !past_current {
            if stmt_contains_offset(&stmt, assign.offset) {
                past_current = true;
            }
            continue;
        }

        // After the current assignment, check for example-scope references
        // and later unconditional assignments
        if check_var_used_in_example_scopes(&stmt, &assign.name) {
            seen_example_ref = true;
        }

        if !seen_example_ref && stmt_is_unconditional_assign_to(&stmt, &assign.name) {
            // Found a later unconditional assignment with no example reference between
            return true;
        }
    }

    false
}

/// Filter dead file-level assignments using assignment-list-based analysis.
/// An assignment to variable X is dead if there's a later collected assignment
/// to X (by byte offset) that is an unconditional write (`x = expr` but not
/// `x ||= expr` or `x += expr`), and no describe-block example-scope reference
/// to X exists between the two assignments' byte offsets in the source.
///
/// This works for assignments nested inside non-RSpec blocks (e.g.,
/// `control do ... end`) because it compares byte offsets rather than walking
/// the statement tree.
fn filter_dead_file_level_assignments<'a>(
    assignments: &'a [VarAssign],
    stmts: &ruby_prism::StatementsNode<'_>,
) -> Vec<&'a VarAssign> {
    if assignments.is_empty() {
        return Vec::new();
    }

    let mut live: Vec<&VarAssign> = Vec::new();

    for (i, assign) in assignments.iter().enumerate() {
        // Check if there's a later collected unconditional assignment to the
        // same variable. We use is_unconditional based on the assignment type.
        let has_later_unconditional = assignments[i + 1..].iter().any(|later| {
            later.name == assign.name && later.is_unconditional && later.offset > assign.offset
        });

        if has_later_unconditional {
            // Check that no describe-block example-scope reference exists
            // between this assignment and the next unconditional one.
            // For simplicity, we just check if the assignment is dead at
            // the file level using the stmts tree.
            let next_unconditional_offset = assignments[i + 1..]
                .iter()
                .find(|a| a.name == assign.name && a.is_unconditional && a.offset > assign.offset)
                .map(|a| a.offset);

            if let Some(next_offset) = next_unconditional_offset {
                // Check if any describe block whose own start offset falls
                // between assign.offset and next_offset references the variable
                // in an example scope. Uses offset-aware recursive search so
                // that describe blocks AFTER both assignments aren't counted.
                if !describe_ref_in_node_between_offsets(
                    stmts,
                    &assign.name,
                    assign.offset,
                    next_offset,
                ) {
                    continue; // dead assignment
                }
            }
        }

        live.push(assign);
    }

    live
}

/// Check if any describe block between two byte offsets (within any node subtree)
/// references the variable in an example scope. Uses offset-aware recursive search
/// so that only describe blocks whose own start offset falls in the range are checked
/// (e.g., `control do ... flags = ...; describe ... end`).
fn describe_ref_in_node_between_offsets(
    stmts: &ruby_prism::StatementsNode<'_>,
    var_name: &[u8],
    start_offset: usize,
    end_offset: usize,
) -> bool {
    for stmt in stmts.body().iter() {
        let loc = stmt.location();
        // Skip nodes entirely before start_offset
        if loc.end_offset() <= start_offset {
            continue;
        }
        // Skip nodes entirely after end_offset
        if loc.start_offset() >= end_offset {
            continue;
        }
        // This node overlaps the range — recursively search for describe blocks
        if describe_ref_in_node_recursive(&stmt, var_name, start_offset, end_offset) {
            return true;
        }
    }
    false
}

/// Recursively search for describe blocks whose start offset falls in range,
/// and check if they reference the variable in example scopes.
fn describe_ref_in_node_recursive(
    node: &ruby_prism::Node<'_>,
    var_name: &[u8],
    start_offset: usize,
    end_offset: usize,
) -> bool {
    let loc = node.location();
    if loc.end_offset() <= start_offset || loc.start_offset() >= end_offset {
        return false;
    }

    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        let is_eg = if let Some(recv) = call.receiver() {
            util::constant_name(&recv).is_some_and(|n| n == b"RSpec")
                && is_rspec_example_group(name)
        } else {
            call.receiver().is_none() && is_rspec_example_group(name)
        };

        if is_eg && loc.start_offset() >= start_offset {
            // Found a describe block in range — check for example-scope refs
            if check_var_used_in_example_scopes(node, var_name) {
                return true;
            }
        }

        // Recurse into block body
        if let Some(blk) = call.block() {
            if let Some(bn) = blk.as_block_node() {
                if let Some(body) = bn.body() {
                    if let Some(stmts) = body.as_statements_node() {
                        for s in stmts.body().iter() {
                            if describe_ref_in_node_recursive(
                                &s,
                                var_name,
                                start_offset,
                                end_offset,
                            ) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }

    // Recurse through control flow
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            for s in stmts.body().iter() {
                if describe_ref_in_node_recursive(&s, var_name, start_offset, end_offset) {
                    return true;
                }
            }
        }
        if let Some(sub) = if_node.subsequent() {
            if describe_ref_in_node_recursive(&sub, var_name, start_offset, end_offset) {
                return true;
            }
        }
    }

    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            for s in stmts.body().iter() {
                if describe_ref_in_node_recursive(&s, var_name, start_offset, end_offset) {
                    return true;
                }
            }
        }
    }

    if let Some(stmts) = node.as_statements_node() {
        for s in stmts.body().iter() {
            if describe_ref_in_node_recursive(&s, var_name, start_offset, end_offset) {
                return true;
            }
        }
    }

    false
}

/// Check if a statement contains a byte offset (for locating an assignment in the stmt list).
fn stmt_contains_offset(node: &ruby_prism::Node<'_>, offset: usize) -> bool {
    let loc = node.location();
    offset >= loc.start_offset() && offset < loc.end_offset()
}

/// Check if a top-level statement unconditionally assigns to the given variable.
fn stmt_is_unconditional_assign_to(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    // Direct assignment: `var = expr`
    if let Some(lw) = node.as_local_variable_write_node() {
        return lw.name().as_slice() == var_name;
    }
    // Multi-write: `a, b = expr`
    if let Some(mw) = node.as_multi_write_node() {
        for target in mw.lefts().iter() {
            if let Some(lt) = target.as_local_variable_target_node() {
                if lt.name().as_slice() == var_name {
                    return true;
                }
            }
        }
        return false;
    }
    false
}

/// Check if a statement reassigns a variable — either unconditionally (`x = expr`)
/// or via operator-write (`x += expr`, `x -= expr`, `x ||= expr`, `x &&= expr`).
/// Any of these kill the previous assignment's value at the same scope level,
/// because the resulting value is a new binding (the old value is consumed or
/// replaced). This is used for linear flow analysis inside non-RSpec blocks and
/// at group scope.
fn stmt_reassigns_var(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    // Only unconditional writes (`x = ...`) kill the tracked value.
    // Operator-writes (`x += ...`, `x ||= ...`, `x &&= ...`) READ the old
    // value first, so the previous assignment is NOT dead.
    if let Some(lw) = node.as_local_variable_write_node() {
        return lw.name().as_slice() == var_name;
    }
    if let Some(mw) = node.as_multi_write_node() {
        for target in mw.lefts().iter() {
            if let Some(lt) = target.as_local_variable_target_node() {
                if lt.name().as_slice() == var_name {
                    return true;
                }
            }
        }
        return false;
    }
    false
}

/// Recursively collect local variable assignments within a node, stopping at
/// scope boundaries (examples, hooks, let, subject, nested example groups,
/// method definitions, class/module definitions).
fn collect_assignments_in_scope(
    node: &ruby_prism::Node<'_>,
    assigns: &mut Vec<VarAssign>,
    inside_block: bool,
) {
    // Direct assignment
    if let Some(lw) = node.as_local_variable_write_node() {
        assigns.push(VarAssign {
            name: lw.name().as_slice().to_vec(),
            offset: lw.location().start_offset(),
            is_unconditional: true,
            inside_block,
        });
        collect_assignments_in_scope(&lw.value(), assigns, inside_block);
        return;
    }

    // or-write: `x ||= expr`
    if let Some(ow) = node.as_local_variable_or_write_node() {
        assigns.push(VarAssign {
            name: ow.name().as_slice().to_vec(),
            offset: ow.location().start_offset(),
            is_unconditional: false,
            inside_block,
        });
        collect_assignments_in_scope(&ow.value(), assigns, inside_block);
        return;
    }

    // and-write: `x &&= expr`
    if let Some(aw) = node.as_local_variable_and_write_node() {
        assigns.push(VarAssign {
            name: aw.name().as_slice().to_vec(),
            offset: aw.location().start_offset(),
            is_unconditional: false,
            inside_block,
        });
        collect_assignments_in_scope(&aw.value(), assigns, inside_block);
        return;
    }

    // operator-write: `x += expr`, `x -= expr`, etc.
    if let Some(ow) = node.as_local_variable_operator_write_node() {
        assigns.push(VarAssign {
            name: ow.name().as_slice().to_vec(),
            offset: ow.location().start_offset(),
            is_unconditional: false,
            inside_block,
        });
        collect_assignments_in_scope(&ow.value(), assigns, inside_block);
        return;
    }

    // Multi-write: `a, b = expr` -- collect targets
    if let Some(mw) = node.as_multi_write_node() {
        for target in mw.lefts().iter() {
            if let Some(lt) = target.as_local_variable_target_node() {
                assigns.push(VarAssign {
                    name: lt.name().as_slice().to_vec(),
                    offset: lt.location().start_offset(),
                    is_unconditional: true,
                    inside_block,
                });
            }
        }
        if let Some(rest) = mw.rest() {
            if let Some(sr) = rest.as_splat_node() {
                if let Some(expr) = sr.expression() {
                    if let Some(lt) = expr.as_local_variable_target_node() {
                        assigns.push(VarAssign {
                            name: lt.name().as_slice().to_vec(),
                            offset: lt.location().start_offset(),
                            is_unconditional: true,
                            inside_block,
                        });
                    }
                }
            }
        }
        for target in mw.rights().iter() {
            if let Some(lt) = target.as_local_variable_target_node() {
                assigns.push(VarAssign {
                    name: lt.name().as_slice().to_vec(),
                    offset: lt.location().start_offset(),
                    is_unconditional: true,
                    inside_block,
                });
            }
        }
        collect_assignments_in_scope(&mw.value(), assigns, inside_block);
        return;
    }

    // Call nodes: stop at scope boundaries, recurse into non-scope calls
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        let no_recv = call.receiver().is_none();
        let is_rspec_recv = call
            .receiver()
            .is_some_and(|r| util::constant_name(&r).is_some_and(|n| n == b"RSpec"));

        // Stop at example scopes, nested example groups, includes methods
        if (no_recv && (is_example_scope(name) || is_includes_method(name)))
            || ((no_recv || is_rspec_recv) && is_rspec_example_group(name))
        {
            return;
        }

        if let Some(recv) = call.receiver() {
            collect_assignments_in_scope(&recv, assigns, inside_block);
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                collect_assignments_in_scope(&arg, assigns, inside_block);
            }
        }

        // For other calls (e.g., `each do ... end`), recurse into the block body.
        // Assignments inside these blocks are marked `inside_block: true` because
        // Ruby blocks create a local variable scope — variables first assigned
        // inside a block are block-local and don't leak to the enclosing scope.
        if let Some(blk) = call.block() {
            if let Some(bn) = blk.as_block_node() {
                if let Some(body) = bn.body() {
                    if let Some(stmts) = body.as_statements_node() {
                        for s in stmts.body().iter() {
                            collect_assignments_in_scope(&s, assigns, true);
                        }
                    }
                }
            }
        }
        return;
    }

    if let Some(hash) = node.as_hash_node() {
        for elem in hash.elements().iter() {
            collect_assignments_in_scope(&elem, assigns, inside_block);
        }
        return;
    }

    if let Some(hash) = node.as_keyword_hash_node() {
        for elem in hash.elements().iter() {
            collect_assignments_in_scope(&elem, assigns, inside_block);
        }
        return;
    }

    if let Some(assoc) = node.as_assoc_node() {
        collect_assignments_in_scope(&assoc.key(), assigns, inside_block);
        collect_assignments_in_scope(&assoc.value(), assigns, inside_block);
        return;
    }

    if let Some(assoc_splat) = node.as_assoc_splat_node() {
        if let Some(expr) = assoc_splat.value() {
            collect_assignments_in_scope(&expr, assigns, inside_block);
        }
        return;
    }

    if let Some(arr) = node.as_array_node() {
        for elem in arr.elements().iter() {
            collect_assignments_in_scope(&elem, assigns, inside_block);
        }
        return;
    }

    if let Some(splat) = node.as_splat_node() {
        if let Some(expr) = splat.expression() {
            collect_assignments_in_scope(&expr, assigns, inside_block);
        }
        return;
    }

    if let Some(ret) = node.as_return_node() {
        if let Some(args) = ret.arguments() {
            for arg in args.arguments().iter() {
                collect_assignments_in_scope(&arg, assigns, inside_block);
            }
        }
        return;
    }

    if let Some(and_node) = node.as_and_node() {
        collect_assignments_in_scope(&and_node.left(), assigns, inside_block);
        collect_assignments_in_scope(&and_node.right(), assigns, inside_block);
        return;
    }

    if let Some(or_node) = node.as_or_node() {
        collect_assignments_in_scope(&or_node.left(), assigns, inside_block);
        collect_assignments_in_scope(&or_node.right(), assigns, inside_block);
        return;
    }

    if let Some(range) = node.as_range_node() {
        if let Some(left) = range.left() {
            collect_assignments_in_scope(&left, assigns, inside_block);
        }
        if let Some(right) = range.right() {
            collect_assignments_in_scope(&right, assigns, inside_block);
        }
        return;
    }

    if let Some(embedded) = node.as_embedded_statements_node() {
        if let Some(stmts) = embedded.statements() {
            for s in stmts.body().iter() {
                collect_assignments_in_scope(&s, assigns, inside_block);
            }
        }
        return;
    }

    if let Some(interp) = node.as_interpolated_string_node() {
        for part in interp.parts().iter() {
            collect_assignments_in_scope(&part, assigns, inside_block);
        }
        return;
    }

    if let Some(interp) = node.as_interpolated_symbol_node() {
        for part in interp.parts().iter() {
            collect_assignments_in_scope(&part, assigns, inside_block);
        }
        return;
    }

    if let Some(interp) = node.as_interpolated_regular_expression_node() {
        for part in interp.parts().iter() {
            collect_assignments_in_scope(&part, assigns, inside_block);
        }
        return;
    }

    // If/Unless: recurse into predicate (for embedded assignments like `if error = expr`)
    // and branches
    if let Some(if_node) = node.as_if_node() {
        // Check predicate for embedded assignments (e.g., `if error = spec['error']`)
        collect_assignments_in_scope(&if_node.predicate(), assigns, inside_block);
        if let Some(stmts) = if_node.statements() {
            for s in stmts.body().iter() {
                collect_assignments_in_scope(&s, assigns, inside_block);
            }
        }
        if let Some(subsequent) = if_node.subsequent() {
            collect_assignments_in_scope(&subsequent, assigns, inside_block);
        }
        return;
    }
    if let Some(unless_node) = node.as_unless_node() {
        // Check predicate for embedded assignments
        collect_assignments_in_scope(&unless_node.predicate(), assigns, inside_block);
        if let Some(stmts) = unless_node.statements() {
            for s in stmts.body().iter() {
                collect_assignments_in_scope(&s, assigns, inside_block);
            }
        }
        if let Some(else_clause) = unless_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    collect_assignments_in_scope(&s, assigns, inside_block);
                }
            }
        }
        return;
    }

    // Else node
    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            for s in stmts.body().iter() {
                collect_assignments_in_scope(&s, assigns, inside_block);
            }
        }
        return;
    }

    // Case/When/In
    if let Some(case_node) = node.as_case_node() {
        for cond in case_node.conditions().iter() {
            if let Some(when_node) = cond.as_when_node() {
                if let Some(stmts) = when_node.statements() {
                    for s in stmts.body().iter() {
                        collect_assignments_in_scope(&s, assigns, inside_block);
                    }
                }
            }
            if let Some(in_node) = cond.as_in_node() {
                if let Some(stmts) = in_node.statements() {
                    for s in stmts.body().iter() {
                        collect_assignments_in_scope(&s, assigns, inside_block);
                    }
                }
            }
        }
        if let Some(else_clause) = case_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    collect_assignments_in_scope(&s, assigns, inside_block);
                }
            }
        }
        return;
    }

    // CaseMatch (pattern matching)
    if let Some(cm) = node.as_case_match_node() {
        for cond in cm.conditions().iter() {
            if let Some(in_node) = cond.as_in_node() {
                if let Some(stmts) = in_node.statements() {
                    for s in stmts.body().iter() {
                        collect_assignments_in_scope(&s, assigns, inside_block);
                    }
                }
            }
        }
        if let Some(else_clause) = cm.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    collect_assignments_in_scope(&s, assigns, inside_block);
                }
            }
        }
        return;
    }

    // Begin/Rescue/Ensure
    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            for s in stmts.body().iter() {
                collect_assignments_in_scope(&s, assigns, inside_block);
            }
        }
        if let Some(rescue_clause) = begin_node.rescue_clause() {
            collect_assignments_in_rescue_node(&rescue_clause, assigns, inside_block);
        }
        if let Some(else_clause) = begin_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    collect_assignments_in_scope(&s, assigns, inside_block);
                }
            }
        }
        if let Some(ensure_clause) = begin_node.ensure_clause() {
            if let Some(stmts) = ensure_clause.statements() {
                for s in stmts.body().iter() {
                    collect_assignments_in_scope(&s, assigns, inside_block);
                }
            }
        }
        return;
    }

    // Parentheses
    if let Some(paren) = node.as_parentheses_node() {
        if let Some(body) = paren.body() {
            collect_assignments_in_scope(&body, assigns, inside_block);
        }
        return;
    }

    // Statements node
    if let Some(stmts) = node.as_statements_node() {
        for s in stmts.body().iter() {
            collect_assignments_in_scope(&s, assigns, inside_block);
        }
        return;
    }

    // While/Until loops
    if let Some(while_node) = node.as_while_node() {
        if let Some(stmts) = while_node.statements() {
            for s in stmts.body().iter() {
                collect_assignments_in_scope(&s, assigns, inside_block);
            }
        }
        return;
    }
    if let Some(until_node) = node.as_until_node() {
        if let Some(stmts) = until_node.statements() {
            for s in stmts.body().iter() {
                collect_assignments_in_scope(&s, assigns, inside_block);
            }
        }
        return;
    }

    // For loop
    if let Some(for_node) = node.as_for_node() {
        if let Some(stmts) = for_node.statements() {
            for s in stmts.body().iter() {
                collect_assignments_in_scope(&s, assigns, inside_block);
            }
        }
    }

    // Stop at class/module/def -- these are Ruby scope boundaries
}

/// Recurse through rescue clause chain.
fn collect_assignments_in_rescue_node(
    rescue_node: &ruby_prism::RescueNode<'_>,
    assigns: &mut Vec<VarAssign>,
    inside_block: bool,
) {
    if let Some(stmts) = rescue_node.statements() {
        for s in stmts.body().iter() {
            collect_assignments_in_scope(&s, assigns, inside_block);
        }
    }
    if let Some(subsequent) = rescue_node.subsequent() {
        collect_assignments_in_rescue_node(&subsequent, assigns, inside_block);
    }
}

/// Check if a variable is reassigned at the top level of a statement list
/// (in a nested example group) before any example scope references it.
/// Returns true if the variable is unconditionally written before any
/// example scope reads it, meaning the outer scope's value is dead.
fn var_reassigned_before_example_ref_in_stmts(
    stmts: &ruby_prism::StatementsNode<'_>,
    var_name: &[u8],
) -> bool {
    for stmt in stmts.body().iter() {
        // Check if this statement unconditionally assigns the variable
        if stmt_is_unconditional_assign_to(&stmt, var_name) {
            return true;
        }
        // Check if this statement references the variable in an example scope
        if check_var_used_in_example_scopes(&stmt, var_name) {
            return false;
        }
    }
    false
}

/// Check if a variable is referenced inside any example scope within the given
/// node tree. Walks through the example group body looking for example scopes
/// and checks if the variable is referenced inside them.
fn check_var_used_in_example_scopes(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        let no_recv = call.receiver().is_none();
        let is_rspec_recv = call
            .receiver()
            .is_some_and(|r| util::constant_name(&r).is_some_and(|n| n == b"RSpec"));

        // Example scopes: it, before, let, subject, etc.
        if no_recv && is_example_scope(name) {
            // `it`/`specify` descriptions and metadata are allowed, but
            // args to lets/subjects/hooks are not.
            if !is_rspec_example(name) {
                if let Some(args) = call.arguments() {
                    for arg in args.arguments().iter() {
                        if node_references_var(&arg, var_name) {
                            return true;
                        }
                    }
                }
            }
            // Check if the block body references the variable
            if let Some(blk) = call.block() {
                if let Some(bn) = blk.as_block_node() {
                    if block_body_references_var(bn, var_name) {
                        return true;
                    }
                }
            }
            // If the var is only in args (description, metadata), it's allowed
            return false;
        }

        // Includes methods: it_behaves_like, include_examples, etc.
        if no_recv && is_includes_method(name) {
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                for (i, arg) in arg_list.iter().enumerate() {
                    if i == 0 {
                        // First arg (shared example name) is allowed
                        continue;
                    }
                    // Subsequent args in interpolated string/symbol are allowed
                    if is_interpolated_string_or_symbol(arg) {
                        continue;
                    }
                    if node_references_var(arg, var_name) {
                        return true;
                    }
                }
            }
            // Check block body of includes method — recurse with full
            // example-scope analysis so nested includes methods respect
            // first-arg exclusion.
            if let Some(blk) = call.block() {
                if let Some(bn) = blk.as_block_node() {
                    if !block_has_param(&bn, var_name) {
                        if let Some(body) = bn.body() {
                            if let Some(stmts) = body.as_statements_node() {
                                for s in stmts.body().iter() {
                                    if check_var_used_in_example_scopes(&s, var_name) {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            return false;
        }

        // Nested example groups: recurse into their body
        // Match both `describe` (no receiver) and `RSpec.describe` (receiver is RSpec)
        if (no_recv || is_rspec_recv) && is_rspec_example_group(name) {
            // Note: describe/context ARGUMENTS are evaluated at the group scope,
            // not inside example scopes. RuboCop doesn't flag these. See the
            // matching comment in stmt_example_scope_var_interaction.
            if let Some(blk) = call.block() {
                if let Some(bn) = blk.as_block_node() {
                    if let Some(body) = bn.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            for s in stmts.body().iter() {
                                if check_var_used_in_example_scopes(&s, var_name) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
            return false;
        }

        // For other calls with blocks (e.g., `each do ... end`), recurse
        // but respect block parameter shadowing
        if let Some(blk) = call.block() {
            if let Some(bn) = blk.as_block_node() {
                if !block_has_param(&bn, var_name) {
                    if let Some(body) = bn.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            for s in stmts.body().iter() {
                                if check_var_used_in_example_scopes(&s, var_name) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        return false;
    }

    // Recurse through control flow structures
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            for s in stmts.body().iter() {
                if check_var_used_in_example_scopes(&s, var_name) {
                    return true;
                }
            }
        }
        if let Some(subsequent) = if_node.subsequent() {
            if check_var_used_in_example_scopes(&subsequent, var_name) {
                return true;
            }
        }
        return false;
    }
    if let Some(unless_node) = node.as_unless_node() {
        if let Some(stmts) = unless_node.statements() {
            for s in stmts.body().iter() {
                if check_var_used_in_example_scopes(&s, var_name) {
                    return true;
                }
            }
        }
        if let Some(else_clause) = unless_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    if check_var_used_in_example_scopes(&s, var_name) {
                        return true;
                    }
                }
            }
        }
        return false;
    }
    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            for s in stmts.body().iter() {
                if check_var_used_in_example_scopes(&s, var_name) {
                    return true;
                }
            }
        }
        return false;
    }

    // Case/When
    if let Some(case_node) = node.as_case_node() {
        for cond in case_node.conditions().iter() {
            if let Some(when_node) = cond.as_when_node() {
                if let Some(stmts) = when_node.statements() {
                    for s in stmts.body().iter() {
                        if check_var_used_in_example_scopes(&s, var_name) {
                            return true;
                        }
                    }
                }
            }
        }
        if let Some(else_clause) = case_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    if check_var_used_in_example_scopes(&s, var_name) {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    // Begin/Rescue
    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            for s in stmts.body().iter() {
                if check_var_used_in_example_scopes(&s, var_name) {
                    return true;
                }
            }
        }
        if let Some(rescue_clause) = begin_node.rescue_clause() {
            if check_var_in_rescue_scopes_inner(&rescue_clause, var_name) {
                return true;
            }
        }
        if let Some(else_clause) = begin_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    if check_var_used_in_example_scopes(&s, var_name) {
                        return true;
                    }
                }
            }
        }
        if let Some(ensure_clause) = begin_node.ensure_clause() {
            if let Some(stmts) = ensure_clause.statements() {
                for s in stmts.body().iter() {
                    if check_var_used_in_example_scopes(&s, var_name) {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    // Parentheses
    if let Some(paren) = node.as_parentheses_node() {
        if let Some(body) = paren.body() {
            if check_var_used_in_example_scopes(&body, var_name) {
                return true;
            }
        }
        return false;
    }

    // Local variable write: the RHS may contain example scopes
    // e.g., `spec = RSpec.describe "SomeTest" do ... end`
    if let Some(lw) = node.as_local_variable_write_node() {
        if check_var_used_in_example_scopes(&lw.value(), var_name) {
            return true;
        }
        return false;
    }

    false
}

/// Check rescue chain for example scope references.
fn check_var_in_rescue_scopes_inner(
    rescue_node: &ruby_prism::RescueNode<'_>,
    var_name: &[u8],
) -> bool {
    if let Some(stmts) = rescue_node.statements() {
        for s in stmts.body().iter() {
            if check_var_used_in_example_scopes(&s, var_name) {
                return true;
            }
        }
    }
    if let Some(subsequent) = rescue_node.subsequent() {
        if check_var_in_rescue_scopes_inner(&subsequent, var_name) {
            return true;
        }
    }
    false
}

/// Deep recursive check: does any node in the subtree WRITE to the variable?
/// Unlike `node_references_var` (which only checks reads/RHS), this checks for
/// write nodes (`LocalVariableWriteNode`, operator-write, or-write, multi-write).
/// Recurses through all node types including call receivers and block bodies.
/// Used to detect writes nested inside `expect do ... end` and similar constructs.
fn node_writes_var_deep(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    if let Some(lw) = node.as_local_variable_write_node() {
        if lw.name().as_slice() == var_name {
            return true;
        }
        return node_writes_var_deep(&lw.value(), var_name);
    }
    if let Some(ow) = node.as_local_variable_or_write_node() {
        if ow.name().as_slice() == var_name {
            return true;
        }
        return node_writes_var_deep(&ow.value(), var_name);
    }
    if let Some(aw) = node.as_local_variable_and_write_node() {
        if aw.name().as_slice() == var_name {
            return true;
        }
        return node_writes_var_deep(&aw.value(), var_name);
    }
    if let Some(opw) = node.as_local_variable_operator_write_node() {
        if opw.name().as_slice() == var_name {
            return true;
        }
        return node_writes_var_deep(&opw.value(), var_name);
    }
    if let Some(mw) = node.as_multi_write_node() {
        for target in mw.lefts().iter() {
            if let Some(lt) = target.as_local_variable_target_node() {
                if lt.name().as_slice() == var_name {
                    return true;
                }
            }
        }
        return node_writes_var_deep(&mw.value(), var_name);
    }
    // For call nodes, check receiver, args, and block body
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            if node_writes_var_deep(&recv, var_name) {
                return true;
            }
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if node_writes_var_deep(&arg, var_name) {
                    return true;
                }
            }
        }
        if let Some(block) = call.block() {
            if let Some(bn) = block.as_block_node() {
                if !block_has_param(&bn, var_name) {
                    if let Some(body) = bn.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            for s in stmts.body().iter() {
                                if node_writes_var_deep(&s, var_name) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        return false;
    }
    // Recurse through common node types
    if let Some(stmts) = node.as_statements_node() {
        return stmts
            .body()
            .iter()
            .any(|s| node_writes_var_deep(&s, var_name));
    }
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            if stmts
                .body()
                .iter()
                .any(|s| node_writes_var_deep(&s, var_name))
            {
                return true;
            }
        }
        if let Some(sub) = if_node.subsequent() {
            return node_writes_var_deep(&sub, var_name);
        }
        return false;
    }
    if let Some(unless_node) = node.as_unless_node() {
        if let Some(stmts) = unless_node.statements() {
            if stmts
                .body()
                .iter()
                .any(|s| node_writes_var_deep(&s, var_name))
            {
                return true;
            }
        }
        if let Some(else_clause) = unless_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                if stmts
                    .body()
                    .iter()
                    .any(|s| node_writes_var_deep(&s, var_name))
                {
                    return true;
                }
            }
        }
        return false;
    }
    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            return stmts
                .body()
                .iter()
                .any(|s| node_writes_var_deep(&s, var_name));
        }
        return false;
    }
    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            return stmts
                .body()
                .iter()
                .any(|s| node_writes_var_deep(&s, var_name));
        }
        return false;
    }
    if let Some(paren) = node.as_parentheses_node() {
        if let Some(body) = paren.body() {
            return node_writes_var_deep(&body, var_name);
        }
    }
    false
}

/// Check if a node is an interpolated string or symbol.
fn is_interpolated_string_or_symbol(node: &ruby_prism::Node<'_>) -> bool {
    node.as_interpolated_string_node().is_some() || node.as_interpolated_symbol_node().is_some()
}

/// Check if a block body contains any assignment to the given variable name.
/// This is a shallow check at the block body's top-level statements only
/// (including recursing through control flow but NOT into nested blocks).
/// Used for block-local variable scoping: if a block assigns a variable that
/// wasn't defined in its enclosing scope, Ruby creates a block-local binding.
fn block_body_assigns_var(block: &ruby_prism::BlockNode<'_>, var_name: &[u8]) -> bool {
    let body = match block.body() {
        Some(b) => b,
        None => return false,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return false,
    };
    let mut assigns = Vec::new();
    for s in stmts.body().iter() {
        collect_assignments_in_scope(&s, &mut assigns, false);
    }
    assigns.iter().any(|a| a.name == var_name)
}

/// Check if the body of a block references a variable. Does a deep recursive
/// search through all node types. Respects block parameter shadowing and
/// reassignment-before-use (if the variable is unconditionally written before
/// any read in the block, the outer variable is not actually referenced).
fn block_body_references_var(block: ruby_prism::BlockNode<'_>, var_name: &[u8]) -> bool {
    // If the block has a parameter with the same name, it shadows the outer var
    if block_has_param(&block, var_name) {
        return false;
    }

    let body = match block.body() {
        Some(b) => b,
        None => return false,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return false,
    };

    // Check if the variable is reassigned before any read in the block.
    // If the first mention of the variable is an unconditional write (not a read),
    // then the outer variable is never actually referenced.
    if var_written_before_read_in_stmts(&stmts, var_name) {
        return false;
    }

    for stmt in stmts.body().iter() {
        if node_references_var(&stmt, var_name) {
            return true;
        }
    }
    false
}

/// Check if a variable is unconditionally written before being read in a
/// sequence of statements. Returns true if the variable is guaranteed to be
/// assigned before any read occurs, meaning the outer scope's value is never
/// used. This matches RuboCop's VariableForce flow-sensitive analysis for the
/// common case of reassignment at the beginning of a block.
fn var_written_before_read_in_stmts(
    stmts: &ruby_prism::StatementsNode<'_>,
    var_name: &[u8],
) -> bool {
    var_written_before_read_in_body(stmts, var_name)
}

/// Check if a node is an unconditional write to the given variable.
/// Matches direct `var = expr` assignments and multi-writes, but not
/// `var ||= expr` or conditional assignments (those might not execute).
/// Also recurses into `begin` blocks and parentheses, since those always
/// execute their contents.
fn is_unconditional_var_write(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    if let Some(lw) = node.as_local_variable_write_node() {
        return lw.name().as_slice() == var_name;
    }
    if let Some(mw) = node.as_multi_write_node() {
        for target in mw.lefts().iter() {
            if let Some(lt) = target.as_local_variable_target_node() {
                if lt.name().as_slice() == var_name {
                    return true;
                }
            }
        }
        return false;
    }
    // `begin ... end` always executes — check if the first statement in the
    // begin body is an unconditional write (recursively).
    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            return var_written_before_read_in_body(&stmts, var_name);
        }
    }
    // Parenthesized expressions always execute
    if let Some(paren) = node.as_parentheses_node() {
        if let Some(body) = paren.body() {
            if let Some(stmts) = body.as_statements_node() {
                return var_written_before_read_in_body(&stmts, var_name);
            }
        }
    }
    false
}

/// Check if a variable is written before read in a sequence of statements.
/// Extracted from `var_written_before_read_in_stmts` for reuse.
fn var_written_before_read_in_body(
    stmts: &ruby_prism::StatementsNode<'_>,
    var_name: &[u8],
) -> bool {
    for stmt in stmts.body().iter() {
        if is_unconditional_var_write(&stmt, var_name) {
            return true;
        }
        if node_reads_var(&stmt, var_name) {
            return false;
        }
    }
    false
}

/// Check if a node reads (but doesn't write) the given variable.
/// Returns true if the variable name appears as a read anywhere in the node.
fn node_reads_var(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    if let Some(lv) = node.as_local_variable_read_node() {
        return lv.name().as_slice() == var_name;
    }
    // For writes, check if the RHS reads the variable
    if let Some(lw) = node.as_local_variable_write_node() {
        if lw.name().as_slice() == var_name {
            // The write itself doesn't read, but the RHS might
            return node_reads_var(&lw.value(), var_name);
        }
    }
    // Operator-write (`x += expr`) always reads the variable first
    if let Some(opw) = node.as_local_variable_operator_write_node() {
        if opw.name().as_slice() == var_name {
            return true;
        }
        return node_reads_var(&opw.value(), var_name);
    }
    // For all other node types, delegate to the full reference checker
    // (this is a conservative check - any reference counts as a read)
    node_references_var(node, var_name)
}

/// Check if a node contains a read of the variable that is NOT preceded by a
/// write to the same variable within the same execution path. This handles
/// the pattern where a conditional block writes then reads the variable:
///   `unless cond; x = new_val; use(x); end`
/// In this case, the read of `x` is preceded by the write, so this returns false.
/// But for `use(x); x = new_val;` it returns true (read before write).
fn node_reads_var_without_prior_write(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    // For conditional nodes (if/unless), check within each branch
    if let Some(if_node) = node.as_if_node() {
        if node_reads_var(&if_node.predicate(), var_name) {
            return true;
        }
        if let Some(stmts) = if_node.statements() {
            if stmts_read_var_without_prior_write(&stmts, var_name) {
                return true;
            }
        }
        if let Some(sub) = if_node.subsequent() {
            if node_reads_var_without_prior_write(&sub, var_name) {
                return true;
            }
        }
        return false;
    }
    if let Some(unless_node) = node.as_unless_node() {
        if node_reads_var(&unless_node.predicate(), var_name) {
            return true;
        }
        if let Some(stmts) = unless_node.statements() {
            if stmts_read_var_without_prior_write(&stmts, var_name) {
                return true;
            }
        }
        if let Some(else_clause) = unless_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                if stmts_read_var_without_prior_write(&stmts, var_name) {
                    return true;
                }
            }
        }
        return false;
    }
    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            return stmts_read_var_without_prior_write(&stmts, var_name);
        }
        return false;
    }
    // For non-conditional nodes, fall back to checking for any read
    node_reads_var(node, var_name)
}

/// Check if a statement list contains a read of the variable that is NOT
/// preceded by a write. Walks statements linearly: if a write is encountered
/// first, subsequent reads in that linear flow are "covered" by the write.
fn stmts_read_var_without_prior_write(
    stmts: &ruby_prism::StatementsNode<'_>,
    var_name: &[u8],
) -> bool {
    for stmt in stmts.body().iter() {
        // Check if this statement writes the variable (direct or deep inside
        // a nested block/call). Deep writes inside blocks still create a new
        // value that subsequent reads in the same scope refer to, matching
        // RuboCop's VariableForce behavior.
        if is_unconditional_var_write(&stmt, var_name) || node_writes_var_deep(&stmt, var_name) {
            return false; // write precedes any subsequent reads
        }
        if node_reads_var_without_prior_write(&stmt, var_name) {
            return true; // found a read before any write
        }
    }
    false
}

/// Check if a block has a parameter with the given name (for shadowing).
fn block_has_param(block: &ruby_prism::BlockNode<'_>, var_name: &[u8]) -> bool {
    let params = match block.parameters() {
        Some(p) => p,
        None => return false,
    };
    let params_node = match params.as_block_parameters_node() {
        Some(p) => p,
        None => return false,
    };
    let inner = match params_node.parameters() {
        Some(p) => p,
        None => return false,
    };
    for p in inner.requireds().iter() {
        if let Some(rp) = p.as_required_parameter_node() {
            if rp.name().as_slice() == var_name {
                return true;
            }
        }
    }
    for p in inner.optionals().iter() {
        if let Some(op) = p.as_optional_parameter_node() {
            if op.name().as_slice() == var_name {
                return true;
            }
        }
    }
    if let Some(rest) = inner.rest() {
        if let Some(rp) = rest.as_rest_parameter_node() {
            if let Some(name) = rp.name() {
                if name.as_slice() == var_name {
                    return true;
                }
            }
        }
    }
    for p in inner.keywords().iter() {
        if let Some(kp) = p.as_required_keyword_parameter_node() {
            if kp.name().as_slice() == var_name {
                return true;
            }
        }
        if let Some(kp) = p.as_optional_keyword_parameter_node() {
            if kp.name().as_slice() == var_name {
                return true;
            }
        }
    }
    false
}

/// Deep recursive check: does any node in the subtree reference the variable?
fn node_references_var(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    if let Some(lv) = node.as_local_variable_read_node() {
        if lv.name().as_slice() == var_name {
            return true;
        }
        return false;
    }

    // Local variable write: only check the RHS for references
    if let Some(lw) = node.as_local_variable_write_node() {
        return node_references_var(&lw.value(), var_name);
    }

    // For call nodes with blocks, check if block params shadow the variable
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            if node_references_var(&recv, var_name) {
                return true;
            }
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if node_references_var(&arg, var_name) {
                    return true;
                }
            }
        }
        if let Some(block) = call.block() {
            if let Some(bn) = block.as_block_node() {
                if !block_has_param(&bn, var_name) {
                    if let Some(body) = bn.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            for s in stmts.body().iter() {
                                if node_references_var(&s, var_name) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        return false;
    }

    // Instance variable write: check RHS
    if let Some(iw) = node.as_instance_variable_write_node() {
        return node_references_var(&iw.value(), var_name);
    }

    // Local variable or-write / and-write
    if let Some(ow) = node.as_local_variable_or_write_node() {
        return node_references_var(&ow.value(), var_name);
    }
    if let Some(aw) = node.as_local_variable_and_write_node() {
        return node_references_var(&aw.value(), var_name);
    }

    // Local variable operator-write: `x += expr`, `x -= expr`, etc.
    // These implicitly read the variable AND write to it. If the variable
    // name matches, it's a reference. Also check the RHS value.
    if let Some(opw) = node.as_local_variable_operator_write_node() {
        if opw.name().as_slice() == var_name {
            return true;
        }
        return node_references_var(&opw.value(), var_name);
    }

    // Multi-write
    if let Some(mw) = node.as_multi_write_node() {
        return node_references_var(&mw.value(), var_name);
    }

    // If/Unless nodes
    if let Some(if_node) = node.as_if_node() {
        if node_references_var(&if_node.predicate(), var_name) {
            return true;
        }
        if let Some(stmts) = if_node.statements() {
            for s in stmts.body().iter() {
                if node_references_var(&s, var_name) {
                    return true;
                }
            }
        }
        if let Some(subsequent) = if_node.subsequent() {
            if node_references_var(&subsequent, var_name) {
                return true;
            }
        }
        return false;
    }

    if let Some(unless_node) = node.as_unless_node() {
        if node_references_var(&unless_node.predicate(), var_name) {
            return true;
        }
        if let Some(stmts) = unless_node.statements() {
            for s in stmts.body().iter() {
                if node_references_var(&s, var_name) {
                    return true;
                }
            }
        }
        if let Some(else_clause) = unless_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    if node_references_var(&s, var_name) {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    // ElseNode
    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            for s in stmts.body().iter() {
                if node_references_var(&s, var_name) {
                    return true;
                }
            }
        }
        return false;
    }

    // Return node
    if let Some(ret) = node.as_return_node() {
        if let Some(args) = ret.arguments() {
            for arg in args.arguments().iter() {
                if node_references_var(&arg, var_name) {
                    return true;
                }
            }
        }
        return false;
    }

    // Parentheses node
    if let Some(paren) = node.as_parentheses_node() {
        if let Some(body) = paren.body() {
            return node_references_var(&body, var_name);
        }
        return false;
    }

    // And/Or nodes
    if let Some(and_node) = node.as_and_node() {
        return node_references_var(&and_node.left(), var_name)
            || node_references_var(&and_node.right(), var_name);
    }
    if let Some(or_node) = node.as_or_node() {
        return node_references_var(&or_node.left(), var_name)
            || node_references_var(&or_node.right(), var_name);
    }

    // Interpolated strings/symbols
    if let Some(interp) = node.as_interpolated_string_node() {
        for part in interp.parts().iter() {
            if let Some(embedded) = part.as_embedded_statements_node() {
                if let Some(stmts) = embedded.statements() {
                    for s in stmts.body().iter() {
                        if node_references_var(&s, var_name) {
                            return true;
                        }
                    }
                }
            }
        }
        return false;
    }
    if let Some(interp) = node.as_interpolated_symbol_node() {
        for part in interp.parts().iter() {
            if let Some(embedded) = part.as_embedded_statements_node() {
                if let Some(stmts) = embedded.statements() {
                    for s in stmts.body().iter() {
                        if node_references_var(&s, var_name) {
                            return true;
                        }
                    }
                }
            }
        }
        return false;
    }

    // Interpolated regular expressions: /#{x}/
    if let Some(interp) = node.as_interpolated_regular_expression_node() {
        for part in interp.parts().iter() {
            if let Some(embedded) = part.as_embedded_statements_node() {
                if let Some(stmts) = embedded.statements() {
                    for s in stmts.body().iter() {
                        if node_references_var(&s, var_name) {
                            return true;
                        }
                    }
                }
            }
        }
        return false;
    }

    // Interpolated backtick commands: `cmd #{var}`
    if let Some(interp) = node.as_interpolated_x_string_node() {
        for part in interp.parts().iter() {
            if let Some(embedded) = part.as_embedded_statements_node() {
                if let Some(stmts) = embedded.statements() {
                    for s in stmts.body().iter() {
                        if node_references_var(&s, var_name) {
                            return true;
                        }
                    }
                }
            }
        }
        return false;
    }

    // Array
    if let Some(arr) = node.as_array_node() {
        for elem in arr.elements().iter() {
            if node_references_var(&elem, var_name) {
                return true;
            }
        }
        return false;
    }

    // Hash / KeywordHash
    if let Some(hash) = node.as_hash_node() {
        for elem in hash.elements().iter() {
            if let Some(assoc) = elem.as_assoc_node() {
                if node_references_var(&assoc.key(), var_name)
                    || node_references_var(&assoc.value(), var_name)
                {
                    return true;
                }
            }
            if let Some(splat) = elem.as_assoc_splat_node() {
                if let Some(expr) = splat.value() {
                    if node_references_var(&expr, var_name) {
                        return true;
                    }
                }
            }
        }
        return false;
    }
    if let Some(kw) = node.as_keyword_hash_node() {
        for elem in kw.elements().iter() {
            if let Some(assoc) = elem.as_assoc_node() {
                if node_references_var(&assoc.key(), var_name)
                    || node_references_var(&assoc.value(), var_name)
                {
                    return true;
                }
            }
            if let Some(splat) = elem.as_assoc_splat_node() {
                if let Some(expr) = splat.value() {
                    if node_references_var(&expr, var_name) {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    // Splat / AssocSplat
    if let Some(splat) = node.as_splat_node() {
        if let Some(expr) = splat.expression() {
            return node_references_var(&expr, var_name);
        }
        return false;
    }
    if let Some(assoc_splat) = node.as_assoc_splat_node() {
        if let Some(expr) = assoc_splat.value() {
            return node_references_var(&expr, var_name);
        }
        return false;
    }

    // Embedded statements
    if let Some(embedded) = node.as_embedded_statements_node() {
        if let Some(stmts) = embedded.statements() {
            for s in stmts.body().iter() {
                if node_references_var(&s, var_name) {
                    return true;
                }
            }
        }
        return false;
    }

    // Case/When
    if let Some(case_node) = node.as_case_node() {
        if let Some(pred) = case_node.predicate() {
            if node_references_var(&pred, var_name) {
                return true;
            }
        }
        for cond in case_node.conditions().iter() {
            if let Some(when_node) = cond.as_when_node() {
                for c in when_node.conditions().iter() {
                    if node_references_var(&c, var_name) {
                        return true;
                    }
                }
                if let Some(stmts) = when_node.statements() {
                    for s in stmts.body().iter() {
                        if node_references_var(&s, var_name) {
                            return true;
                        }
                    }
                }
            }
        }
        if let Some(else_clause) = case_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    if node_references_var(&s, var_name) {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    // Begin/Rescue/Ensure
    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            for s in stmts.body().iter() {
                if node_references_var(&s, var_name) {
                    return true;
                }
            }
        }
        if let Some(rescue_clause) = begin_node.rescue_clause() {
            if node_references_var_in_rescue_inner(&rescue_clause, var_name) {
                return true;
            }
        }
        if let Some(else_clause) = begin_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    if node_references_var(&s, var_name) {
                        return true;
                    }
                }
            }
        }
        if let Some(ensure_clause) = begin_node.ensure_clause() {
            if let Some(stmts) = ensure_clause.statements() {
                for s in stmts.body().iter() {
                    if node_references_var(&s, var_name) {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    // Rescue node
    if let Some(rescue_node) = node.as_rescue_node() {
        return node_references_var_in_rescue_inner(&rescue_node, var_name);
    }

    // While/Until
    if let Some(while_node) = node.as_while_node() {
        if node_references_var(&while_node.predicate(), var_name) {
            return true;
        }
        if let Some(stmts) = while_node.statements() {
            for s in stmts.body().iter() {
                if node_references_var(&s, var_name) {
                    return true;
                }
            }
        }
        return false;
    }
    if let Some(until_node) = node.as_until_node() {
        if node_references_var(&until_node.predicate(), var_name) {
            return true;
        }
        if let Some(stmts) = until_node.statements() {
            for s in stmts.body().iter() {
                if node_references_var(&s, var_name) {
                    return true;
                }
            }
        }
        return false;
    }

    // Range
    if let Some(range) = node.as_range_node() {
        if let Some(left) = range.left() {
            if node_references_var(&left, var_name) {
                return true;
            }
        }
        if let Some(right) = range.right() {
            if node_references_var(&right, var_name) {
                return true;
            }
        }
        return false;
    }

    // Lambda
    if let Some(lambda) = node.as_lambda_node() {
        if let Some(body) = lambda.body() {
            if let Some(stmts) = body.as_statements_node() {
                for s in stmts.body().iter() {
                    if node_references_var(&s, var_name) {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    // Defined?
    if let Some(def) = node.as_defined_node() {
        return node_references_var(&def.value(), var_name);
    }

    // StatementsNode
    if let Some(stmts) = node.as_statements_node() {
        for s in stmts.body().iter() {
            if node_references_var(&s, var_name) {
                return true;
            }
        }
        return false;
    }

    // Yield
    if let Some(yield_node) = node.as_yield_node() {
        if let Some(args) = yield_node.arguments() {
            for arg in args.arguments().iter() {
                if node_references_var(&arg, var_name) {
                    return true;
                }
            }
        }
        return false;
    }

    // For loop: `for x in items do ... end`
    if let Some(for_node) = node.as_for_node() {
        if node_references_var(&for_node.collection(), var_name) {
            return true;
        }
        if let Some(stmts) = for_node.statements() {
            for s in stmts.body().iter() {
                if node_references_var(&s, var_name) {
                    return true;
                }
            }
        }
        return false;
    }

    // Ternary / inline conditionals (same node type as if in Prism, already handled above)

    // ConstantPathNode: `result::Success` — check the parent (e.g., `result`)
    if let Some(cp) = node.as_constant_path_node() {
        if let Some(parent) = cp.parent() {
            return node_references_var(&parent, var_name);
        }
        return false;
    }

    // ImplicitNode: Ruby 3.1+ keyword shorthand `method(url:)` wraps the
    // value in an ImplicitNode containing a LocalVariableReadNode (or CallNode).
    // Unwrap and check the inner node.
    if let Some(implicit) = node.as_implicit_node() {
        return node_references_var(&implicit.value(), var_name);
    }

    false
}

/// Check rescue chain for variable references.
fn node_references_var_in_rescue_inner(
    rescue_node: &ruby_prism::RescueNode<'_>,
    var_name: &[u8],
) -> bool {
    if let Some(stmts) = rescue_node.statements() {
        for s in stmts.body().iter() {
            if node_references_var(&s, var_name) {
                return true;
            }
        }
    }
    if let Some(subsequent) = rescue_node.subsequent() {
        if node_references_var_in_rescue_inner(&subsequent, var_name) {
            return true;
        }
    }
    false
}

/// Check if a method name represents an example scope
fn is_example_scope(name: &[u8]) -> bool {
    let s = std::str::from_utf8(name).unwrap_or("");
    crate::cop::util::RSPEC_EXAMPLES.contains(&s)
        || crate::cop::util::RSPEC_HOOKS.contains(&s)
        || crate::cop::util::RSPEC_LETS.contains(&s)
        || crate::cop::util::RSPEC_SUBJECTS.contains(&s)
}

/// Check if a method name is an RSpec includes method
fn is_includes_method(name: &[u8]) -> bool {
    matches!(
        name,
        b"it_behaves_like" | b"it_should_behave_like" | b"include_examples" | b"include_context"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(LeakyLocalVariable, "cops/rspec/leaky_local_variable");

    #[test]
    fn test_no_fp_iterator_var_only_in_description() {
        // jruby pattern: format = "%" + f inside .each, used only in it description
        let source = br#"describe SomeClass do
  %w(d i).each do |f|
    format = "%" + f

    it "supports integer formats using #{format}" do
      ("%#{f}" % 10).should == "10"
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable, source);
        assert_eq!(
            diags.len(),
            0,
            "Expected 0 offenses for var used only in description, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_no_fp_sibling_block_scope() {
        // discourse rswag pattern: variable assigned in one block, reference in sibling block
        let source = br#"describe SomeClass do
  path "/api" do
    get "List" do
      expected_schema = nil
      response "200" do
        it_behaves_like "endpoint" do
          let(:schema) { expected_schema }
        end
      end
    end

    post "Create" do
      expected_schema = load_schema("create")
      parameter name: :params, schema: expected_schema
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable, source);
        // Only the get block's expected_schema should be flagged (it leaks into it_behaves_like).
        // The post block's expected_schema should NOT be flagged (used only at DSL level, not in example scopes).
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense (get block only), got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_no_fp_var_reassigned_in_nested_expect_block() {
        // excon pattern: response = nil at group scope, reassigned inside it > expect do end
        let source = br#"describe SomeClass do
  response = nil

  it 'returns a response' do
    expect do
      response = make_request()
    end.to_not raise_error
  end

  it 'has status' do
    expect(response.status).to eq(200)
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable, source);
        // RuboCop doesn't flag this because the first it block's write kills the group-level value.
        // The second it block reads from the first it block's assignment (linear flow).
        // The deep write check detects the nested write inside expect do end.
        assert_eq!(
            diags.len(),
            0,
            "Expected 0 offenses (nested write kills value), got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fn_file_level_var_in_if_elsif() {
        // inspec pattern: variable assigned at file level, conditionally reassigned,
        // then used in describe block inside an if.
        // RuboCop flags all 4 assignments (initial + 3 conditional).
        let source = br#"root_group = 'root'

if os == 'aix'
  root_group = 'system'
elsif os == 'freebsd'
  root_group = 'wheel'
elsif os == 'suse'
  root_group = 'sfcb'
end

if true
  describe SomeClass do
    its('groups') { should include root_group }
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable, source);
        // RuboCop flags ALL assignments because any one of them could be the value
        // that reaches the example scope (it depends on the runtime `os` value).
        assert_eq!(
            diags.len(),
            4,
            "Expected 4 offenses (initial + 3 conditional), got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fp_file_level_var_reassigned_at_group_scope() {
        // FP: File-level `records = fetch_records()` should NOT fire when the variable
        // is unconditionally reassigned at the group scope before any example reference.
        // The reference in the `it` block belongs to the group-level assignment, not the file-level one.
        let source = br#"records = fetch_records()

describe SomeClass do
  records = limited_records()

  it 'works' do
    expect(records).to be_empty
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable, source);
        // Should get exactly 1 offense (the group-level `records = limited_records()`)
        // NOT 2 offenses (file-level + group-level)
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense (group-level only), got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
        // The offense should be at line 4 (group-level assignment), not line 1 (file-level)
        assert_eq!(
            diags[0].location.line, 4,
            "Offense should be on group-level assignment (line 4)"
        );
    }

    #[test]
    fn test_fp_webmachine_matcher_block() {
        // webmachine pattern: variable assigned inside matcher block, referenced
        // in it blocks that are siblings (not inside the matcher). The matcher block
        // creates a block scope — the route variable inside it is block-local.
        let source = br#"describe SomeClass do
  matcher :match_route do |*expected|
    route = SomeClass.new(expected[0], Class.new(Resource), expected[1] || {})
    match do |actual|
      route.match?(actual)
    end
  end

  it 'warns' do
    [['*'], ['foo', '*']].each do |path|
      route = described_class.allocate
      expect(route).to receive(:warn)
      route.send :initialize, path, resource, {}
    end
  end

  context 'matching' do
    subject { '/' }
    it { is_expected.to match_route([]) }
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable, source);
        assert_eq!(
            diags.len(),
            0,
            "Expected 0 offenses (matcher block variable is block-local), got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fp_otwcode_populate_block() {
        // otwcode pattern: variable assigned inside populate block (non-RSpec DSL).
        let source = br#"describe SomeClass do
  context 'n plus one' do
    populate do |n|
      create_list(:subscription, n, subscribable: work)
      email = UserMailer.batch_subscription_notification(Subscription.first.id, entries)
      expect(email).to have_html_part_content("posted a new chapter")
    end

    it 'generates queries per mail' do
      expect do
        Subscription.ids.each do |id|
          email = UserMailer.batch_subscription_notification(id, entries)
          expect(email).to have_html_part_content("posted a new chapter")
        end
      end.to perform_linear_number_of_queries(slope: 10)
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable, source);
        assert_eq!(
            diags.len(),
            0,
            "Expected 0 offenses (populate block variable), got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fp_datadog_shared_context_include_context() {
        // DataDog pattern: file-level variable used only in include_context first arg
        let source = br#"major_version = 7

RSpec.shared_context 'Rails test application' do
  include_context 'Rails base application' do
    include_context "Rails #{major_version} test application"
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable, source);
        assert_eq!(
            diags.len(),
            0,
            "Expected 0 offenses (include_context first arg in interpolation), got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fp_fastlane_file_level_nil_before_hook_reassign() {
        // fastlane pattern: file-level nil-initialized variables reassigned in hooks.
        let source = br#"test_ui = nil
generator = nil

describe SomeClass do
  describe '#generate' do
    before(:each) do
      unless initialized
        test_ui = PluginGeneratorUI.new
        generator = PluginGenerator.new(ui: test_ui)
      end
    end

    after(:all) do
      test_ui = nil
      generator = nil
    end

    it 'generates plugin' do
      expect(generator).not_to be_nil
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable, source);
        assert_eq!(
            diags.len(),
            0,
            "Expected 0 offenses (file-level nil vars reassigned in hooks), got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fn_nil_initialized_var_read_in_hook_predicate_before_write() {
        let source = br#"describe 'Puppet Ruby Generator' do
  context 'when generating static code' do
    module_def = nil

    before(:each) do
      if module_def.nil?
        module_def = build_module
      end
    end

    it 'keeps the generated module' do
      expect(module_def).not_to be_nil
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for nil init read in hook predicate, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
        assert_eq!(diags[0].location.line, 3, "Offense should be on line 3");
    }

    #[test]
    fn test_fp_devsec_control_block() {
        // dev-sec pattern: variables inside InSpec's `control` block with a describe inside.
        // RuboCop flags only the LAST unconditional assignment (line 4: flags = flags.split(' '))
        // because earlier assignments are dead (overwritten before any example reference).
        let source = br#"control 'sysctl-33' do
  flags = parse_config_file('/proc/cpuinfo').flags
  flags ||= ''
  flags = flags.split(' ')

  describe '/proc/cpuinfo' do
    it 'Flags should include NX' do
      expect(flags).to include('nx')
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable, source);
        // RuboCop flags only the last assignment (line 4) — earlier ones are dead.
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense (last unconditional assignment only), got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
        assert_eq!(diags[0].location.line, 4, "Offense should be on line 4");
    }

    #[test]
    fn test_no_fp_shared_context_vars() {
        // shared_context IS in RSPEC_EXAMPLE_GROUPS, so check_node DOES process it.
        // Variables inside shared_context used in example scopes ARE correctly flagged.
        // This test verifies that file-level collection stops at shared_context.
        //
        // When only the shared_context exists in the file (no surrounding describe),
        // check_node processes it and finds sc_opts used in the before hook — offense.
        // This is CORRECT per RuboCop's vendor spec.
        let source_with_describe = br#"sc_extra = "file level"
RSpec.shared_context "test setup" do
  sc_opts = { timeout: 30 }
  before { setup(sc_opts) }
end
describe SomeClass do
  it "test" do
    expect(true).to be true
  end
end
"#;
        // sc_extra at file level should NOT be flagged (not used in example scopes).
        // sc_opts inside shared_context IS correctly flagged by check_node (as a
        // group-level assignment used in the before hook), but should NOT be
        // double-flagged by check_source as a file-level assignment.
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable, source_with_describe);
        let offenses_at_line1: Vec<_> = diags.iter().filter(|d| d.location.line == 1).collect();
        assert!(
            offenses_at_line1.is_empty(),
            "sc_extra at file level should not be flagged (not used in examples)"
        );
    }

    #[test]
    fn test_fn_def_body_vars_leak_into_describe() {
        // chef pattern: variables assigned inside def body, then used in describe/let/it blocks
        let source = br#"def static_provider_resolution(opts = {})
  action         = opts[:action]
  provider_class = opts[:provider]
  resource_class = opts[:resource]

  describe resource_class, "static provider" do
    let(:node) do
      node = Object.new
      node
    end

    it "resolves the provider" do
      expect(provider_class).to eq(action)
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable, source);
        // All 3 variables should be flagged: action, provider_class, resource_class
        // resource_class is used in describe args AND it description is allowed,
        // but provider_class and action are used inside example scopes (let and it blocks)
        assert!(
            diags.len() >= 2,
            "Expected at least 2 offenses for def body vars, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fn_module_body_vars_leak_into_describe() {
        let source = br#"module SamlIdp
  metadata_1 = <<-eos
<md:EntityDescriptor></md:EntityDescriptor>
  eos

  RSpec.describe 'incoming metadata' do
    it 'parses the metadata' do
      expect(metadata_1).to include('EntityDescriptor')
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for module body var leak, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
        assert_eq!(diags[0].location.line, 2, "Offense should be on line 2");
    }

    #[test]
    fn test_fn_each_block_param_reassignment() {
        // arachni/ManageIQ pattern: block param reassigned then used in example
        let source = br#"describe SomeClass do
  items.each do |k|
    k = k.to_s

    it "includes the '#{k}' group" do
      expect(data[k]).to eq(subject.send(k))
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable, source);
        // k = k.to_s should be flagged since the reassignment creates a new local
        // that leaks into the example scope
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for block param reassignment, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fn_var_used_in_describe_argument() {
        // Describe/context arguments are evaluated at the group scope, not inside
        // example scopes. RuboCop's `part_of_example_scope?` doesn't match
        // describe/context, so variables used only in describe arguments should
        // NOT be flagged.
        let source = br#"RSpec.describe(SomeClass) do
  result = described_class

  describe result::Success do
    it "works" do
      expect(true).to be true
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable, source);
        assert!(
            diags.is_empty(),
            "Expected no offense for var used only in describe arg, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fn_var_used_in_if_condition_with_let() {
        // DataDog pattern: variable assigned in if condition, used in let/it blocks
        // `if error = spec['error']; let(:expected_error) { error }; it ... end`
        let source = br#"describe SomeClass do
  specs.each do |spec|
    context spec['name'] do
      if error = spec['error']
        let(:expected_error) { error }

        it 'fails' do
          expect { run }.to raise_error(expected_error)
        end
      end
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable, source);
        assert!(
            diags.len() >= 1,
            "Expected at least 1 offense for var in if-condition with let, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fn_var_before_non_rspec_block_with_describe() {
        // DataDog CI pattern: variables assigned before a non-RSpec block containing RSpec.describe
        let source = br#"describe SomeClass do
  max_failures = 4
  failure_count = 0

  with_new_environment do
    spec = RSpec.describe "SomeTest" do
      it "test" do
        failure_count += 1
        if failure_count >= max_failures
          raise "too many failures"
        end
      end
    end

    spec.run
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable, source);
        assert!(
            diags.len() >= 2,
            "Expected at least 2 offenses for vars before non-RSpec block with describe, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fn_var_used_in_let_name_argument() {
        let source = br#"RSpec.shared_examples 'a form field' do |field, html_options|
  html_options ||= :options

  include_context 'form', field

  context 'when class/id/data attributes are provided' do
    let(html_options) { { class: 'custom-field' } }

    it 'sets the attributes on the field' do
      expect(true).to eq(true)
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for let-name arg leak, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
        assert_eq!(diags[0].location.line, 2, "Offense should be on line 2");
    }

    #[test]
    fn test_fn_var_used_in_interpolated_xstring() {
        let source = br#"def which(cmd)
  cmd
end

insert_tee_log = '  2>&1 | tee -a vagrant.log ' if which('tee')

describe 'VM Life Cycle' do
  it 'starts Linux and Windows VM' do
    expect(`vagrant up  #{insert_tee_log}`).to include('tee')
  end

  it 'destroys Linux and Windows VM' do
    expect(`vagrant destroy --force  #{insert_tee_log}`).to include('Done removing resources')
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for interpolated xstring leak, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
        assert_eq!(diags[0].location.line, 5, "Offense should be on line 5");
    }

    // Note: def self.method with .each containing context/let (DataDog pattern)
    // is not yet handled. Variables inside `def self.define_cases` are in a separate
    // Ruby scope. Implementing this requires VariableForce-level scope tracking.
    // This accounts for ~15-20 of the corpus FNs.
}
