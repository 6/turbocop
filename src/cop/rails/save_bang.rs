use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Rails/SaveBang - flags ActiveRecord persist methods (save, update, destroy, create, etc.)
/// whose return value is not checked, suggesting bang variants instead.
///
/// ## Investigation findings (2026-03-08)
///
/// **Root cause of massive FN (24,736):** `visit_call_node` did not visit `BlockNode`
/// children of CallNodes. It only handled `block_argument_node` (e.g., `&block`) but
/// not actual block bodies (e.g., `items.each { |i| i.save }`). Since `visit_block_node`
/// was never invoked for blocks attached to calls, persist calls inside any block body
/// were invisible to the cop.
///
/// **Fix:** Added `block.as_block_node()` handling in `visit_call_node` to invoke
/// `visit_block_node` for block bodies attached to call nodes.
///
/// **FP cause (558):** `persisted?` follow-up checks were not recognized. When a create
/// method result was assigned to a variable and `persisted?` was called on that variable
/// in the next statement (e.g., `user = User.create; if user.persisted?`), the cop
/// incorrectly flagged the create call. Also, inline patterns like
/// `(user = User.create).persisted?` were not suppressed.
///
/// **Fix:** Added lookahead in statement visitors to detect `persisted?` checks on
/// assigned variables. Added suppression when `persisted?` is called directly on a
/// receiver containing a create assignment.
///
/// **Remaining gaps:** Large FN count likely has additional causes beyond block traversal,
/// such as unhandled control flow patterns or context-tracking gaps. The block fix
/// addresses the primary structural issue.
///
/// ## Investigation findings (2026-03-10)
///
/// **FP cause: receiver context suppression.** `visit_call_node` was pushing `Argument`
/// context for ALL receivers, including non-persisted? method chains. This meant
/// `object.save.to_s`, `object.save.inspect`, etc. were incorrectly suppressed
/// because `save` (as receiver of `to_s`) got Argument context. In RuboCop, only
/// `.persisted?` receivers are suppressed via `checked_immediately?`.
///
/// **Fix:** Only push Argument context for receivers when the call is `persisted?`.
/// For other methods, the receiver inherits parent context, allowing persist calls
/// to be flagged when chained with non-persisted? methods.
///
/// **FP cause: negation not treated as condition.** `!object.save` / `not object.save`
/// was not recognized as condition context. RuboCop's `single_negative?` check treats
/// unary `!`/`not` as part of `operator_or_single_negative?`, exempting modify methods.
///
/// **Fix:** Added special handling for `!` CallNodes (no arguments) to push Condition
/// context for the receiver.
///
/// **FP cause: yield/super arguments not recognized.** `yield object.save` and
/// `super(object.save)` were not treated as argument context because `visit_yield_node`
/// and `visit_super_node` were not overridden.
///
/// **Fix:** Added `visit_yield_node` and `visit_super_node` to push Argument context.
///
/// **FN cause: string interpolation treated as argument.** `"#{object.save}"` was
/// suppressed because `visit_embedded_statements_node` pushed Argument context.
/// RuboCop does NOT treat interpolation as "using" the return value.
///
/// **Fix:** Removed Argument context push from `visit_embedded_statements_node`.
///
/// ## Investigation findings (2026-03-18)
///
/// **FP cause: persisted? lookahead was limited to next statement only.** When a create
/// method result was assigned to a variable and `persisted?` was called several statements
/// later (not immediately), the cop incorrectly flagged the assignment. RuboCop uses
/// VariableForce to track ALL references to the assigned variable across the entire scope,
/// including calls inside nested conditionals, method calls, and other expressions.
///
/// Examples that were FPs:
/// - `user = User.create; logger.info; if user.persisted?` — intervening statement
/// - `@user = User.create; render json: @user, status: @user.persisted? ? :ok : :err` — non-adjacent
/// - `record = find_or_create_by(name:); log("id=#{record.id}"); raise unless record.persisted?`
///
/// **Fix:** Changed `should_suppress_create` to scan ALL subsequent statements (not just
/// the next one) using `subtree_checks_persisted`, a visitor-based recursive search.
/// Added `PersistedFinder` visitor struct that searches any subtree for `var.persisted?`.
///
/// **Scope:** The scan is bounded to the current `StatementsNode` body (same method/block
/// scope), matching RuboCop's per-scope VariableForce tracking. Cross-method references
/// are correctly not suppressed.
///
/// ## Corpus investigation (2026-03-19)
///
/// Corpus oracle reported FP=1437, FN=4678 (82% match).
///
/// **FP root cause 1: Non-local variable create-in-assignment flagged.**
/// RuboCop's VariableForce only tracks local variables. Instance/class/global variable
/// create assignments (e.g., `@user = User.create(...)`) are skipped by
/// `return_value_assigned?` in `on_send` and never checked by VariableForce's
/// `check_assignment`. Nitrocop was flagging all create-in-assignment regardless.
/// **Fix:** Added `in_local_assignment` flag; only flag create-in-assignment for locals.
///
/// **FN root cause 1: Receiver chain context propagation.**
/// When a persist call is the receiver of a method chain (e.g., `log(object.save.to_s)`),
/// nitrocop was inheriting the outer Argument/Assignment context down to the persist call,
/// incorrectly exempting it. RuboCop evaluates each persist call independently — the
/// immediate parent being a chained method doesn't exempt it.
/// **Fix:** Push VoidStatement context when visiting non-persisted? receiver chains.
///
/// **FN root cause 2: Multi-statement body ImplicitReturn.**
/// RuboCop's `implicit_return?` only recognizes single-statement method/block bodies.
/// In a multi-statement body, the last statement's parent is a `begin` node (not def/block
/// directly), so `implicit_return?` returns false. Nitrocop was marking the last statement
/// as ImplicitReturn for ALL method/block bodies regardless of statement count.
/// **Fix:** Only grant ImplicitReturn when the body has exactly one statement (len == 1).
///
/// ## Corpus investigation (2026-03-19, batch 2)
///
/// Oracle: FP=240, FN=183 (98.6% match rate on 31,933 offenses).
///
/// **FP root cause 1: Narrow literal check in expected_signature (100+ FP).**
/// Only checked StringNode/IntegerNode/SymbolNode. Missed InterpolatedStringNode,
/// InterpolatedSymbolNode, ArrayNode, TrueNode, FalseNode, etc. RuboCop's
/// `expected_signature?` uses `!node.first_argument.literal?` which covers all literals.
/// **Fix:** Added `is_literal_node()` helper covering all Prism literal types.
///
/// **FP root cause 2: Array/hash Collection context too permissive (56+ FP).**
/// Pushed Collection context for array/hash elements, exempting persist calls inside
/// arrays. RuboCop's `assignable_node` climbs through arrays/hashes — elements inherit
/// the enclosing context. `[save]` in void context IS flagged.
/// **Fix:** Made arrays/hashes transparent (inherit parent context).
///
/// **FP root cause 3: Setter receiver not recognized as assignment (22 FP).**
/// `create.multipart = true` — RuboCop's `return_value_assigned?` treats setter calls
/// (`method=`) as assignments via `SendNode#assignment?` (alias for `setter_method?`).
/// **Fix:** Detect setter methods in `visit_call_node` and push Assignment context.
///
/// **FP root cause 4: Missing assignment node visitors (10+ FP).**
/// Missing visitors for operator-write (`+=`), or-write (`||=`), and-write (`&&=`),
/// constant or-write, index or-write, call or-write, etc. Persist calls in these
/// contexts got void context instead of assignment.
/// **Fix:** Added visitors for all Prism write/operator-write node types.
///
/// **FP root cause 5: Parenthesized conditions lost context (6 FP).**
/// `if(@result.save)` — ParenthesesNode body is a StatementsNode, and our
/// `visit_statements_node` pushed VoidStatement, overriding the Condition context.
/// **Fix:** `visit_parentheses_node` unwraps StatementsNode, visiting children directly.
///
/// **FP root cause 6: ||= and &&= flagging create (4+ FP).**
/// RuboCop's VariableForce `check_assignment` returns early for or_asgn/and_asgn
/// because `right_assignment_node` gets the lvasgn target, not the RHS.
/// **Fix:** Don't set `in_local_assignment` for ||= and &&= write nodes.
///
/// **FP root cause 7: Block argument not counted in expected_signature (5+ FP).**
/// `create(hash, &block)` has 2 args in RuboCop (block_pass counts), but Prism
/// separates block from arguments. Our count was 1.
/// **Fix:** Count BlockArgumentNode in total argument count.
///
/// **FN root cause 1: Array/hash Collection context suppressed offenses (30+ FN).**
/// Same fix as FP root cause 2 — making arrays/hashes transparent both fixes FPs
/// (arrays in void context now flag) and FNs (arrays in assignment context now exempt).
///
/// **FN root cause 2: Singleton method implicit return (14+ FN).**
/// `def self.method; create(name: 'x'); end` — RuboCop's `implicit_return?` only
/// matches `def`, not `defs`. Nitrocop gave ImplicitReturn for all DefNode.
/// **Fix:** Only grant ImplicitReturn when DefNode has no receiver (instance method).
///
/// **Remaining (6 FP, 34 FN):** Edge cases including block_node unwrapping in
/// `assignable_node` (RuboCop unwraps `create { }` to the block_node before checking
/// parent context), and complex control flow patterns. These represent <0.13% of total
/// offenses and are documented for future investigation.
///
/// ## Corpus investigation (2026-03-19, batch 3)
///
/// Oracle: FP=18, FN=37 (99.8% match rate on 32,661 offenses).
///
/// **FP root cause: create inside array/hash in local variable assignment (18 FP).**
/// `x = [Model.create(...), Model.create(...)]` — RuboCop's VariableForce
/// `check_assignment` checks `if rhs_node.send_type?` on the RHS. ArrayNode/HashNode
/// doesn't match, so create calls inside arrays in local assignments are never flagged.
/// Nitrocop was propagating `in_local_assignment` through transparent array/hash nodes.
/// **Fix:** Save and reset `in_local_assignment` to false in `visit_array_node`,
/// `visit_hash_node`, `visit_keyword_hash_node`.
///
/// **FN root cause: block-wrapped create in argument context (7 FN).**
/// `Subscription.create { cleanup }` as an array element inside a method argument.
/// In RuboCop's Parser gem AST, `create { }` becomes `Block(Send, Args, Body)`.
/// `argument?` on the Send walks: Send→Block(parent)→array, and Block.parent is not
/// `send_type?`, so `argument?` returns false — RuboCop flags it. In Prism, the block
/// is part of the CallNode, so the CallNode inherits Argument context from the enclosing
/// expression.
/// **Fix:** In `process_persist_call`, when context is Argument and the call has a
/// block body (BlockNode, not BlockArgumentNode), treat as VoidStatement.
///
/// **Remaining (~30 FN):** Various patterns in discourse, galetahub, natalie-lang etc.
/// including Hash#update on hash literal in splat args (4 FN, hard to fix without
/// parent tracking) and unknown patterns across ~16 repos.
///
/// ## Corpus investigation (2026-03-19, batch 4)
///
/// Targeted fix for remaining ~24 FN from batch 3. Four root causes:
///
/// **FN root cause 1: Create in compound boolean (16 FN).**
/// RuboCop checks `compound_boolean?` for create methods — if the create call is inside
/// `||`/`&&`/`or`/`and`, it's ALWAYS flagged as conditional regardless of enclosing
/// assignment/argument context. Only `implicit_return?` and `explicit_return?` walk
/// through or_type? parents to exempt.
/// **Fix:** Added `in_compound_boolean` flag set inside `visit_or_node`/`visit_and_node`.
///
/// **FN root cause 2: Rescue modifier breaks context chains (5 FN).**
/// `@post.destroy rescue nil` — RuboCop's `implicit_return?` doesn't walk through
/// rescue_modifier, and `assignable_node` stops at rescue_mod.
/// **Fix:** Added `visit_rescue_modifier_node` pushing VoidStatement for the expression.
///
/// **FN root cause 3: Yield/super arguments not argument context (2+ FN).**
/// RuboCop's `argument?` only checks `parent.send_type?`. Yield/super are not send_type?.
/// **Fix:** Changed `visit_yield_node`/`visit_super_node` to push VoidStatement.
///
/// **FN root cause 4: Splat breaks argument context (1 FN).**
/// `execute *builder.create` — `assignable_node` stops at splat, `argument?` returns false.
/// **Fix:** Added `visit_splat_node` pushing VoidStatement for the expression.
///
/// ## Corpus investigation (2026-03-19, batch 5)
///
/// Location-level verification: FP=18→6, FN=37→1 (48 of 55 known mismatches fixed).
///
/// **FN root cause 5: Or/And node inherited Assignment context (3 FN).**
/// `@dir = get || create(...)` — RuboCop's `return_value_assigned?` uses `assignable_node`
/// which only walks through hash/array parents, NOT through or/and nodes. So create inside
/// `||` inside an assignment is NOT considered assigned — compound_boolean takes priority.
/// **Fix:** `visit_or_node` no longer inherits Assignment context from parent. When the or
/// is inside an assignment, children get Condition context (compound_boolean path).
///
/// **FN root cause 6: ImplicitReturn leaked through if/else branches (1 FN).**
/// `def m; if cond; self.x = find || create; end; end` — the if node is the single
/// statement in the method body (ImplicitReturn), but RuboCop's `implicit_return?` does NOT
/// walk through if/case/begin nodes — it only walks through or_type? parents.
/// **Fix:** `visit_if_node` and `visit_unless_node` push VoidStatement for body/else clauses
/// to prevent ImplicitReturn from the outer scope leaking into branches.
///
/// **FN root cause 7: Or node ImplicitReturn applied to both children (1 FN).**
/// `items.map { |v| Gem::Version.create(v) or raise }` — create is the LEFT child of `or`
/// in a single-statement block body. RuboCop's `implicit_return?` uses `sibling_index` +
/// `find_method_with_sibling_index` where walking through an or_type? increments the index.
/// The formula `method.children.size == node.sibling_index + sibling_index` only holds for
/// the RIGHT child (index 1), not the LEFT child (index 0) of the or expression.
/// **Fix:** `visit_or_node` only grants ImplicitReturn to the right child; left child gets
/// Condition context.
///
/// **FN root cause 8: csend `&.persisted?` incorrectly suppressed create (1 FN).**
/// `s = DomainSetup.create(...); s if s&.persisted?` — RuboCop's `call_to_persisted?`
/// checks `node.send_type?` which excludes csend (safe navigation). So `s&.persisted?`
/// does NOT count as a persisted? check. Our PersistedFinder and checked_immediately path
/// were matching both send and csend.
/// **Fix:** Added csend detection (call_operator length == 2) to exclude `&.persisted?`.
///
/// **FP root cause: create in `&&` with direct assignment (discourse topic.rb).**
/// `if (new_post = creator.create) && new_post.present?` — create is directly assigned
/// inside a `&&` expression. RuboCop's `return_value_assigned?` sees the assignment parent
/// and returns true (priority over compound_boolean). Our compound_boolean check was
/// overriding the Assignment context.
/// **Fix:** When `in_compound_boolean` is true but current context is Assignment (meaning
/// the create was directly assigned inside the boolean), let the assignment path handle it.
///
/// **Remaining (6 FP, 1 FN):**
/// - 6 FP: Require VariableForce-level tracking (non-adjacent persisted? checks in
///   complex control flow, multi-write assignments). Examples: discourse topic.rb:1112
///   (`(x=create)&&x.present?` with `x.persisted?` in if-body), discourse
///   export_csv_file.rb:85 (create in local var, value used non-adjacent), redmine
///   project.rb (save in if-body), taps operation.rb (create-with-block in multi-write).
///   Some may also be oracle artifacts (config differences or stale data).
/// - 1 FN: neo4j association_proxy_spec.rb:225 (`Lesson.create` inside `[x, create]`
///   inside `+=` operator — may be an oracle artifact since `+=` RHS array isn't tracked
///   by VariableForce's `right_assignment_node`).
///
/// ## Corpus investigation (2026-03-20)
///
/// Oracle: FP=5, FN=1 (99.98% match rate on 32,755 offenses).
///
/// **FP fix: cross-scope persisted? detection (1 FP fixed).**
/// discourse topic.rb:1112: `if (new_post = creator.create) && new_post.present?` with
/// `new_post.persisted?` in the if body. The `should_suppress_create` mechanism only
/// scanned direct statement children, missing assignments nested in if predicates.
/// **Fix:** Added pre-scan (`CreateVarFinder` visitor + `suppressed_create_vars` set)
/// that searches entire scope bodies for local variable create assignments with persisted?
/// references anywhere in the same scope, including inside compound expressions.
///
/// **FN fix: operator_write void context (1 FN fixed).**
/// neo4j association_proxy_spec.rb:225: `Lesson.create` inside array inside `+=`.
/// RuboCop's `assignable_node` doesn't handle `op_asgn`, so persist calls in
/// operator-write values are void context, not assignment.
/// **Fix:** Changed all `*OperatorWriteNode` visitors from Assignment to VoidStatement.
/// Also added receiver visiting in `CallOperatorWriteNode` to flag persist calls in
/// receiver chains (e.g., `Student.create.lessons += [...]`).
///
/// **Remaining (4 FP, 0 FN):**
/// - discourse export_csv_file.rb:85: create in local var, variable referenced but no
///   persisted? check. RuboCop behavior unclear — may be corpus config artifact.
/// - redmine project.rb:928/953: bare `save` in if body. Both tools should flag this
///   based on source analysis; likely corpus config artifact.
/// - taps operation.rb:510: `Taps::Multipart.create` in multi_write. Appears exempted
///   by Assignment context (in_local_assignment=false); likely corpus config artifact.
///
/// ## Corpus investigation (2026-03-20, batch 6)
///
/// Oracle: FP=9, FN=0.
///
/// **FP root cause: operator-write (+=, &=, etc.) incorrectly pushed VoidStatement (5 FP).**
/// Examples: `success &= pref.update(v: ...)` (openstreetmap, 3 FP),
/// `packet += @chacha_main.update(data)` (net-ssh, 1 FP),
/// `decrypted += cipher.update(buffer)` (roo, 1 FP).
/// RuboCop's `return_value_assigned?` checks `assignable_node(node).parent.assignment?`.
/// `op_asgn` is in `SHORTHAND_ASSIGNMENTS` (`rubocop-ast` node.rb), so `.assignment?`
/// returns true — persist calls in operator-write values ARE exempt.
/// The previous batch 4 fix incorrectly changed operator-write from Assignment to
/// VoidStatement based on a wrong analysis that `assignable_node` doesn't handle `op_asgn`.
/// **Fix:** Changed all `*OperatorWriteNode` visitors back to Assignment context.
///
/// **Remaining (4 FP, 0 FN):** Same 4 corpus config artifacts from batch 5.
///
/// ## Corpus investigation (2026-03-20, batch 7)
///
/// Oracle: FP=4, FN=0 (99.99% match rate on 32,756 offenses).
///
/// Deep investigation revealed these are NOT config artifacts — they are genuine
/// RuboCop quirks that must be replicated:
///
/// **FP fix 1: RuboCop value equality in condition check (2 FP fixed).**
/// redmine project.rb:928/953: `if save; ...; save; end` — the second `save` in the
/// if-body is structurally identical (by Parser AST ==) to the if-condition `save`.
/// RuboCop's `in_condition_or_compound_boolean?` uses `node == deparenthesize(parent.condition)`
/// which is VALUE equality, not identity. Both `save` nodes are `(send nil :save)` with
/// identical structure, so == returns true. RuboCop exempts the body statement as
/// if it were in condition context.
/// **Fix:** In `visit_if_node`, compare each body statement's source bytes against the
/// predicate's source bytes. Statements matching the predicate get Condition context
/// instead of VoidStatement.
///
/// **FP fix 2: RuboCop call_to_persisted? if-condition substitution (1 FP fixed).**
/// discourse export_csv_file.rb:85: `user_export = UserExport.create(...)` with
/// `user_export.update_columns(...)` inside an `if upload.persisted?` body. RuboCop's
/// `call_to_persisted?` replaces node with `node.parent.condition` when
/// `node.parenthesized_call? && node.parent.if_type?`. This means a variable reference
/// in a parenthesized call inside an if-body triggers persisted? suppression if the
/// if-condition is ANY `.persisted?` call, even on a different variable.
/// **Fix:** Extended `PersistedFinder` to detect if-nodes where the condition is
/// `.persisted?` and the if-body references the target variable.
///
/// **FP fix 3: in_local_assignment leaked through block boundaries (1 FP fixed).**
/// taps operation.rb:510: `d3 = c.time_delta do; content, ct = X.create do ... end; end` —
/// the outer `d3 = ...` set `in_local_assignment = true`, which leaked through the
/// block boundary into the multi-write's create call.
/// **Fix:** Reset `in_local_assignment` to false when entering block/lambda/def bodies,
/// since these create new scope boundaries in RuboCop's VariableForce.
///
/// **Remaining (0 FP, 0 FN).** All known FP/FN locations are fixed.
///
/// ## Extended corpus investigation (2026-03-23)
///
/// Extended oracle: FP=1, FN=6.
///
/// **FP fix: in_transparent_container leaked through block boundaries (1 FP).**
/// foodcoops foodsoft supplier.rb:227: `Tempfile.create do |temp_file| ... end` inside
/// `{ articles: files.collect_concat do |file| ... end }`. The hash `{ articles: ... }`
/// set `in_transparent_container = true`, which leaked through the `collect_concat` block
/// into the `Tempfile.create` call. Since `Tempfile.create` has a block body,
/// `process_persist_call` saw `has_block_body && in_transparent_container` and forced
/// VoidStatement, incorrectly flagging it.
/// **Fix:** Reset `in_transparent_container` to false when entering block/lambda/def bodies,
/// matching the existing `in_local_assignment` reset at scope boundaries.
///
/// **FN fix 1: ExplicitReturn inherited through or_node (3 FN).**
/// WikiEduDashboard category.rb:61: `return record || create(wiki:, ...)`. RuboCop's
/// `explicit_return?` uses `assignable_node(node).parent`, which only walks hash/array
/// parents, NOT or/and nodes. For create inside `||` inside `return`, the parent of
/// the create_node is the or_node (not the return), so `explicit_return?` returns false.
/// Compound_boolean takes priority and flags with conditional message.
/// **Fix:** In `visit_or_node`, ExplicitReturn no longer inherits to children. Both
/// children get Condition context (compound_boolean path), matching RuboCop behavior.
///
/// **FN fix 2: Parenthesized persist call in argument context (2 FN).**
/// felipediesel auto_increment_spec.rb:45: `@accounts << (@account.users.create ...)`
/// noosfero cnpj_test.rb:58: `assert ( cnpj_valido.save ), "msg"`.
/// RuboCop's `argument?` checks `node.parent.send_type?`. Parentheses create a `begin`
/// wrapper which is not `send_type?`, so `argument?` returns false. Our transparent
/// `visit_parentheses_node` incorrectly passed Argument context through.
/// **Fix:** `visit_parentheses_node` pushes VoidStatement for body when current context
/// is Argument or Assignment, since RuboCop's argument?/return_value_assigned? checks
/// are blocked by parentheses (`begin` node).
///
/// **FN fix 3: Condition/compound_boolean context leaked through arrays (1 FN).**
/// rsim ruby-plsql oci_connection.rb:281: `version.update` inside array inside `&&`.
/// RuboCop's `in_condition_or_compound_boolean?` checks the FIRST non-begin ancestor.
/// Array is not `operator_keyword?` or `conditional?`, so the check returns false.
/// Our code propagated `in_compound_boolean` and Condition context through arrays.
/// **Fix:** Reset `in_compound_boolean` in array/hash/keyword_hash visitors. Override
/// Condition context to VoidStatement when entering arrays (matching RuboCop behavior
/// where arrays break the condition/boolean context chain).
///
/// **Remaining FN (1):** chargify examples/metafields.rb:31: `field = Chargify::...create
/// name: 'internal info'` — already correctly detected by nitrocop. This FN may be a
/// secondary effect of one of the above fixes or a corpus oracle artifact.
pub struct SaveBang;

/// Modify-type persistence methods whose return value indicates success/failure.
const MODIFY_PERSIST_METHODS: &[&[u8]] = &[b"save", b"update", b"update_attributes", b"destroy"];

/// Create-type persistence methods that always return a model (truthy).
const CREATE_PERSIST_METHODS: &[&[u8]] = &[
    b"create",
    b"create_or_find_by",
    b"first_or_create",
    b"find_or_create_by",
];

const MSG: &str = "Use `%prefer%` instead of `%current%` if the return value is not checked.";
const CREATE_MSG: &str = "Use `%prefer%` instead of `%current%` if the return value is not checked. Or check `persisted?` on model returned from `%current%`.";
const CREATE_CONDITIONAL_MSG: &str = "`%current%` returns a model which is always truthy.";

/// The context in which a node appears, as tracked by the visitor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Context {
    /// Statement in a method/block body, not the last one (void context).
    VoidStatement,
    /// Last statement in a method/block body (implicit return).
    ImplicitReturn,
    /// Right-hand side of an assignment.
    Assignment,
    /// Used as a condition in if/unless/case/ternary or in a boolean expression.
    Condition,
    /// Used as an argument to a method call.
    Argument,
    /// Used in an explicit return or next statement.
    ExplicitReturn,
}

/// Check if a method name is a setter method (ends with `=` but not a comparison operator).
/// Matches RuboCop's `MethodDispatchNode#setter_method?` / `assignment?`.
fn is_setter_method(name: &[u8]) -> bool {
    name.ends_with(b"=")
        && !matches!(
            name,
            b"==" | b"!=" | b"===" | b"<=>" | b"<=" | b">=" | b"=~"
        )
}

/// Check if a Prism node is a literal type (matches RuboCop's `Node#literal?`).
/// Literal types include strings, symbols, numbers, arrays, booleans, regexps, etc.
/// Hash is technically a literal but is handled separately (allowed in expected_signature).
fn is_literal_node(node: &ruby_prism::Node<'_>) -> bool {
    node.as_string_node().is_some()
        || node.as_interpolated_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_interpolated_symbol_node().is_some()
        || node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_array_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_regular_expression_node().is_some()
        || node.as_interpolated_regular_expression_node().is_some()
        || node.as_x_string_node().is_some()
        || node.as_interpolated_x_string_node().is_some()
        || node.as_range_node().is_some()
        || node.as_source_file_node().is_some()
        || node.as_source_line_node().is_some()
        || node.as_source_encoding_node().is_some()
}

impl Cop for SaveBang {
    fn name(&self) -> &'static str {
        "Rails/SaveBang"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        let allow_implicit_return = config.get_bool("AllowImplicitReturn", true);
        let allowed_receivers = config
            .get_string_array("AllowedReceivers")
            .unwrap_or_default();

        let mut visitor = SaveBangVisitor {
            cop: self,
            source,
            allow_implicit_return,
            allowed_receivers,
            diagnostics: Vec::new(),
            context_stack: Vec::new(),
            suppress_create_assignment: false,
            in_local_assignment: false,
            in_compound_boolean: false,
            in_transparent_container: false,
            suppressed_create_vars: Vec::new(),
            current_local_var_name: None,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct SaveBangVisitor<'a, 'src> {
    cop: &'a SaveBang,
    source: &'src SourceFile,
    allow_implicit_return: bool,
    allowed_receivers: Vec<String>,
    diagnostics: Vec<Diagnostic>,
    context_stack: Vec<Context>,
    /// When true, suppress create-in-assignment offenses because a persisted? check follows.
    suppress_create_assignment: bool,
    /// When true, the current Assignment context is for a local variable write.
    /// Only local variable create-in-assignment generates offenses; instance/class/global/constant
    /// assignments are treated as "return value used" (RuboCop's VariableForce only tracks locals).
    in_local_assignment: bool,
    /// When true, we are inside an `||` or `&&` (compound boolean) expression.
    /// RuboCop checks `compound_boolean?` for create methods BEFORE `argument?`,
    /// so create inside `||`/`&&` is ALWAYS flagged as conditional regardless of
    /// enclosing context. Only affects CREATE methods, not MODIFY methods.
    in_compound_boolean: bool,
    /// When true, the current context was inherited through a transparent container
    /// (hash/array/keyword_hash). In RuboCop's `assignable_node`, a block-bearing call
    /// inside a hash/array breaks the chain — the block node's parent is the pair/array,
    /// not the enclosing assignment/argument. So block-bearing calls inside transparent
    /// containers lose their parent's exemption and are treated as void context.
    in_transparent_container: bool,
    /// Variable names where create-in-assignment is suppressed because persisted? is checked
    /// somewhere in the same scope. Pre-computed at scope entry to handle cases where the
    /// create is nested inside compound expressions (if predicates, and/or nodes) and the
    /// persisted? check is in a different branch of the same statement.
    suppressed_create_vars: Vec<Vec<u8>>,
    /// The name of the local variable currently being assigned (set inside
    /// visit_local_variable_write_node). Used by process_persist_call to check
    /// the suppressed_create_vars set.
    current_local_var_name: Option<Vec<u8>>,
}

impl SaveBangVisitor<'_, '_> {
    fn current_context(&self) -> Option<Context> {
        self.context_stack.last().copied()
    }

    /// Check if a CallNode is a persistence method. Returns (is_persist, is_create) tuple.
    fn classify_persist_call(&self, call: &ruby_prism::CallNode<'_>) -> Option<bool> {
        let method_name = call.name().as_slice();

        let is_modify = MODIFY_PERSIST_METHODS.contains(&method_name);
        let is_create = CREATE_PERSIST_METHODS.contains(&method_name);

        if !is_modify && !is_create {
            return None;
        }

        // Check expected_signature: no arguments, or one hash/non-literal argument.
        // In RuboCop, &block_arg counts as an argument (part of node.arguments).
        // In Prism, it's separate (call.block()). Count it for parity.
        let has_block_arg = call
            .block()
            .is_some_and(|b| b.as_block_argument_node().is_some());

        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            let total_args = arg_list.len() + usize::from(has_block_arg);

            // destroy with any arguments is not a persistence method
            if method_name == b"destroy" {
                return None;
            }

            // More than one argument: not a persistence call (e.g., Model.save(1, name: 'Tom'))
            if total_args >= 2 {
                return None;
            }

            // Single argument: must be a hash or non-literal.
            // Matches RuboCop's: node.first_argument.hash_type? || !node.first_argument.literal?
            if arg_list.len() == 1 {
                let arg = &arg_list[0];
                // Hash and keyword hash arguments are valid (expected persistence signature)
                if arg.as_hash_node().is_some() || arg.as_keyword_hash_node().is_some() {
                    // Valid: create(name: 'Joe'), save(validate: false)
                } else if is_literal_node(arg) {
                    // Any other literal type is NOT a valid persistence call signature
                    return None;
                }
                // Non-literals (variables, method calls, splats, etc.) are valid
            }
        } else if has_block_arg {
            // Only a &block argument and no other args — still valid (1 argument)
            // RuboCop: expected_signature? returns true (1 arg, not literal)
            // This is fine — persist method with just a block
        }

        // Check allowed receivers
        if self.is_allowed_receiver(call) {
            return None;
        }

        Some(is_create)
    }

    /// Check if the receiver is in the AllowedReceivers list or is ENV.
    fn is_allowed_receiver(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return false,
        };

        // ENV is always exempt (it has an `update` method that isn't ActiveRecord)
        if let Some(cr) = receiver.as_constant_read_node() {
            if cr.name().as_slice() == b"ENV" {
                return true;
            }
        }
        if let Some(cp) = receiver.as_constant_path_node() {
            if let Some(name) = cp.name() {
                if name.as_slice() == b"ENV" && cp.parent().is_none() {
                    return true;
                }
            }
        }

        if self.allowed_receivers.is_empty() {
            return false;
        }

        let recv_src = &self.source.as_bytes()
            [receiver.location().start_offset()..receiver.location().end_offset()];
        let recv_str = std::str::from_utf8(recv_src).unwrap_or("");

        // Check each allowed receiver pattern
        for allowed in &self.allowed_receivers {
            if self.receiver_chain_matches(call, allowed) {
                return true;
            }
            // Direct match on receiver source
            if recv_str == allowed.as_str() {
                return true;
            }
        }

        false
    }

    /// Check if the receiver chain of a call matches an allowed receiver pattern.
    /// E.g., for `merchant.gateway.save`, checking against "merchant.gateway" should match.
    fn receiver_chain_matches(&self, call: &ruby_prism::CallNode<'_>, allowed: &str) -> bool {
        let parts: Vec<&str> = allowed.split('.').collect();
        let mut current_receiver = call.receiver();

        for part in parts.iter().rev() {
            match current_receiver {
                None => return false,
                Some(node) => {
                    if let Some(call_node) = node.as_call_node() {
                        let name = std::str::from_utf8(call_node.name().as_slice()).unwrap_or("");
                        if name != *part {
                            return false;
                        }
                        current_receiver = call_node.receiver();
                    } else if let Some(cr) = node.as_constant_read_node() {
                        let name = std::str::from_utf8(cr.name().as_slice()).unwrap_or("");
                        if !self.const_matches(name, part) {
                            return false;
                        }
                        current_receiver = None;
                    } else if let Some(cp) = node.as_constant_path_node() {
                        let const_name = self.constant_path_name(&cp);
                        if !self.const_matches(&const_name, part) {
                            return false;
                        }
                        current_receiver = None;
                    } else if let Some(lv) = node.as_local_variable_read_node() {
                        let name = std::str::from_utf8(lv.name().as_slice()).unwrap_or("");
                        if name != *part {
                            return false;
                        }
                        current_receiver = None;
                    } else {
                        return false;
                    }
                }
            }
        }

        true
    }

    fn constant_path_name(&self, cp: &ruby_prism::ConstantPathNode<'_>) -> String {
        let src = &self.source.as_bytes()[cp.location().start_offset()..cp.location().end_offset()];
        std::str::from_utf8(src).unwrap_or("").to_string()
    }

    /// Match const names following RuboCop rules:
    /// Const == Const, ::Const == ::Const, ::Const == Const,
    /// NameSpace::Const == Const, NameSpace::Const != ::Const
    fn const_matches(&self, const_name: &str, allowed: &str) -> bool {
        if allowed.starts_with("::") {
            // Absolute match: must match exactly or with leading ::
            const_name == allowed
                || format!("::{const_name}") == allowed
                || const_name == &allowed[2..]
        } else {
            // Relative match: allowed can match the tail of const_name
            let parts: Vec<&str> = allowed.split("::").collect();
            let const_parts: Vec<&str> = const_name.trim_start_matches("::").split("::").collect();
            if parts.len() > const_parts.len() {
                return false;
            }
            parts
                .iter()
                .rev()
                .zip(const_parts.iter().rev())
                .all(|(a, c)| a == c)
        }
    }

    /// Extract the variable name from an assignment node (local, instance, global, class, multi,
    /// or conditional assignment). Returns the variable name bytes and whether the RHS contains
    /// a create-type persist call.
    fn assignment_var_name<'n>(node: &'n ruby_prism::Node<'n>) -> Option<Vec<u8>> {
        if let Some(lv) = node.as_local_variable_write_node() {
            return Some(lv.name().as_slice().to_vec());
        }
        if let Some(iv) = node.as_instance_variable_write_node() {
            return Some(iv.name().as_slice().to_vec());
        }
        if let Some(gv) = node.as_global_variable_write_node() {
            return Some(gv.name().as_slice().to_vec());
        }
        if let Some(cv) = node.as_class_variable_write_node() {
            return Some(cv.name().as_slice().to_vec());
        }
        // local_variable_or_write (||=)
        if let Some(lov) = node.as_local_variable_or_write_node() {
            return Some(lov.name().as_slice().to_vec());
        }
        // multi-write: use first target if it's a local variable
        if let Some(mw) = node.as_multi_write_node() {
            let lefts: Vec<_> = mw.lefts().iter().collect();
            if let Some(first) = lefts.first() {
                if let Some(lt) = first.as_local_variable_target_node() {
                    return Some(lt.name().as_slice().to_vec());
                }
            }
        }
        // CallNode wrapping an assignment: `assert version = create(...)`.
        // In RuboCop, VariableForce tracks the assignment regardless of nesting.
        // Check the first argument of the call for a nested local variable write.
        if let Some(call) = node.as_call_node() {
            if let Some(args) = call.arguments() {
                for arg in args.arguments().iter() {
                    if let Some(name) = Self::assignment_var_name(&arg) {
                        return Some(name);
                    }
                }
            }
        }
        None
    }

    /// Check if a node is a variable read matching the given name.
    fn node_is_var(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
        if let Some(lv) = node.as_local_variable_read_node() {
            return lv.name().as_slice() == var_name;
        }
        if let Some(iv) = node.as_instance_variable_read_node() {
            return iv.name().as_slice() == var_name;
        }
        if let Some(gv) = node.as_global_variable_read_node() {
            return gv.name().as_slice() == var_name;
        }
        if let Some(cv) = node.as_class_variable_read_node() {
            return cv.name().as_slice() == var_name;
        }
        false
    }

    /// Check if a node's subtree contains a create-type persist call.
    fn subtree_has_create_call(&self, node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            if self.classify_persist_call(&call) == Some(true) {
                return true;
            }
            // Check arguments recursively
            if let Some(args) = call.arguments() {
                for arg in args.arguments().iter() {
                    if self.subtree_has_create_call(&arg) {
                        return true;
                    }
                }
            }
        }
        // Check assignment value
        if let Some(lv) = node.as_local_variable_write_node() {
            return self.subtree_has_create_call(&lv.value());
        }
        false
    }

    /// Check if a statement is a create-type assignment where the next statement
    /// checks persisted? on the assigned variable.
    fn should_suppress_create(
        &self,
        stmt: &ruby_prism::Node<'_>,
        body: &[ruby_prism::Node<'_>],
        idx: usize,
    ) -> bool {
        // Extract variable name from assignment (handles nested assignments like `assert x = create`)
        let var_name = match Self::assignment_var_name(stmt) {
            Some(name) => name,
            None => return false,
        };

        // Check if the statement's subtree contains a create-type call
        if !self.subtree_has_create_call(stmt) {
            return false;
        }

        // Scan ALL subsequent statements for any persisted? check on the variable.
        // RuboCop uses VariableForce to track all references across the entire scope,
        // so we need to search beyond just the immediately following statement.
        for next_stmt in body.iter().skip(idx + 1) {
            if Self::subtree_checks_persisted(next_stmt, &var_name) {
                return true;
            }
        }

        false
    }

    /// Recursively search a subtree for any `var.persisted?` call.
    /// This matches RuboCop's VariableForce approach of checking ALL references
    /// to the assigned variable anywhere in the scope, including inside nested
    /// conditionals, method calls, and other expressions.
    fn subtree_checks_persisted(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
        let mut finder = PersistedFinder {
            var_name,
            found: false,
        };
        finder.visit(node);
        finder.found
    }

    fn flag_void_context(&mut self, call: &ruby_prism::CallNode<'_>) {
        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("save");
        let msg_loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = self.source.offset_to_line_col(msg_loc.start_offset());
        let message = MSG
            .replace("%prefer%", &format!("{method_name}!"))
            .replace("%current%", method_name);
        self.diagnostics
            .push(self.cop.diagnostic(self.source, line, column, message));
    }

    fn flag_create_conditional(&mut self, call: &ruby_prism::CallNode<'_>) {
        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("create");
        let msg_loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = self.source.offset_to_line_col(msg_loc.start_offset());
        let message = CREATE_CONDITIONAL_MSG.replace("%current%", method_name);
        self.diagnostics
            .push(self.cop.diagnostic(self.source, line, column, message));
    }

    fn flag_create_assignment(&mut self, call: &ruby_prism::CallNode<'_>) {
        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("create");
        let msg_loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = self.source.offset_to_line_col(msg_loc.start_offset());
        let message = CREATE_MSG
            .replace("%prefer%", &format!("{method_name}!"))
            .replace("%current%", method_name);
        self.diagnostics
            .push(self.cop.diagnostic(self.source, line, column, message));
    }

    /// Process a call node that has been identified as a persist method.
    /// `is_create` indicates whether this is a create-type method.
    fn process_persist_call(&mut self, call: &ruby_prism::CallNode<'_>, is_create: bool) {
        // Check if .persisted? is called directly on the result
        // This is the checked_immediately? case from RuboCop
        // We can't check this in the visitor, so we skip it for now
        // (it would require looking at the parent, which we don't have)

        // RuboCop checks compound_boolean? for create methods. In RuboCop, `argument?`
        // and `return_value_assigned?` check the direct parent node — and the direct parent
        // of a create call inside `||` is the OrNode, not the enclosing call/assignment.
        // So argument? and assigned? return false. Only implicit_return? and explicit_return?
        // walk through or_type? parents, so only those exempt create in compound boolean.
        //
        if is_create && self.in_compound_boolean {
            // RuboCop checks return_value_assigned? BEFORE compound_boolean?.
            // When create is directly assigned (e.g., `(x = create) && x.present?`),
            // the assignment takes priority. We detect this when the immediate context
            // is Assignment — meaning a LocalVariableWriteNode (or similar) pushed it
            // INSIDE the or/and expression. For `@x = get || create`, the or_node
            // pushes Condition (not inheriting Assignment), so we correctly flag it.
            //
            let current = self.current_context();
            if matches!(current, Some(Context::Assignment)) {
                // Let the assignment path handle it below
            } else {
                // Check if ImplicitReturn or ExplicitReturn exist in the relevant
                // portion of the stack. RuboCop's implicit_return? walks UP through
                // or_type? parents to find the def/block, but does NOT walk through
                // begin/if/case nodes. The or_node only inherits ImplicitReturn when
                // it's a direct child of a method/block body (not nested in if/else).
                // We detect this by looking above VoidStatement barriers — a VoidStatement
                // pushed by visit_if_node/visit_unless_node blocks ImplicitReturn inheritance.
                // Only check contexts from the most recent or_node inheritance point upward,
                // stopping at VoidStatement boundaries.
                let has_return_exempt = self
                    .context_stack
                    .iter()
                    .rev()
                    .take_while(|c| !matches!(c, Context::VoidStatement))
                    .any(|c| matches!(c, Context::ImplicitReturn | Context::ExplicitReturn));
                if !has_return_exempt {
                    self.flag_create_conditional(call);
                    return;
                }
            }
        }

        // Block-bearing persist calls inside transparent containers (hash/array):
        // In RuboCop's `assignable_node`, the block node breaks the walk chain.
        // `create { }` inside a hash/array means assignable_node returns block_node
        // (not the hash/array), and block_node.parent is pair/array (not the enclosing
        // assignment/argument). So none of the exemptions (return_value_assigned?,
        // argument?, explicit_return?) apply — RuboCop flags with MSG.
        let has_block_body = call.block().is_some_and(|b| b.as_block_node().is_some());
        let effective_context = if has_block_body && self.in_transparent_container {
            Some(Context::VoidStatement)
        } else {
            self.current_context()
        };

        match effective_context {
            Some(Context::VoidStatement) => {
                // Void context: always flag with MSG
                self.flag_void_context(call);
            }
            Some(Context::Assignment) => {
                // Assignment: exempt for modify methods, flag create methods
                // unless persisted? is checked on the assigned variable.
                // Only flag for LOCAL variable assignments — RuboCop's VariableForce
                // only tracks locals; ivar/cvar/gvar assignments are treated as
                // "return value used" by return_value_assigned? in on_send.
                if is_create && !self.suppress_create_assignment && self.in_local_assignment {
                    // Also check the pre-scanned suppressed_create_vars set.
                    // This handles nested patterns where the assignment is inside
                    // compound expressions (if predicates, and/or nodes).
                    let suppressed_by_scope = self
                        .current_local_var_name
                        .as_ref()
                        .is_some_and(|name| self.suppressed_create_vars.contains(name));
                    if !suppressed_by_scope {
                        self.flag_create_assignment(call);
                    }
                }
            }
            Some(Context::Condition) => {
                // Condition/boolean: exempt for modify methods, flag create methods
                if is_create {
                    self.flag_create_conditional(call);
                }
            }
            Some(Context::ImplicitReturn) => {
                // Implicit return: exempt if AllowImplicitReturn is true
                // (already handled by not pushing VoidStatement for last stmt)
                // This context means AllowImplicitReturn is true, so skip.
            }
            Some(Context::Argument) | Some(Context::ExplicitReturn) => {
                // These contexts mean the return value is used: no offense
            }
            None => {
                // No context tracked (e.g., top-level expression outside any method).
                // Treat as void context.
                self.flag_void_context(call);
            }
        }
    }

    /// Visit children of a StatementsNode with proper context tracking.
    /// For each child statement, determines whether it's in void context or
    /// implicit return position, and sets context accordingly.
    fn visit_statements_with_context(
        &mut self,
        node: &ruby_prism::StatementsNode<'_>,
        in_method_or_block: bool,
    ) {
        let body: Vec<_> = node.body().iter().collect();
        let len = body.len();

        // Pre-scan: find local variable create assignments where persisted? is checked
        // anywhere in this scope. This handles nested patterns like:
        //   if (record = Creator.create(opts)) && record.present?
        //     increment if record.persisted?
        //   end
        // where the create is inside an if predicate and persisted? is in the body.
        let saved_suppressed = std::mem::take(&mut self.suppressed_create_vars);
        for stmt in body.iter() {
            let mut finder = CreateVarFinder { found: Vec::new() };
            finder.visit(stmt);
            for var_name in finder.found {
                // Search ALL statements in this scope for persisted? on this variable
                let has_persisted = body
                    .iter()
                    .any(|s| Self::subtree_checks_persisted(s, &var_name));
                if has_persisted {
                    self.suppressed_create_vars.push(var_name);
                }
            }
        }

        for (i, stmt) in body.iter().enumerate() {
            let is_last = i == len - 1;

            // RuboCop's implicit_return? only recognizes single-statement method/block
            // bodies. In a multi-statement body, the last statement's parent is a `begin`
            // node, not the def/block directly, so implicit_return? returns false.
            let ctx = if is_last && in_method_or_block && self.allow_implicit_return && len == 1 {
                Context::ImplicitReturn
            } else {
                Context::VoidStatement
            };

            // Check if this assignment's create call has persisted? checked in the next statement
            let suppress = self.should_suppress_create(stmt, &body, i);
            if suppress {
                self.suppress_create_assignment = true;
            }

            self.context_stack.push(ctx);
            self.visit(stmt);
            self.context_stack.pop();

            if suppress {
                self.suppress_create_assignment = false;
            }
        }

        self.suppressed_create_vars = saved_suppressed;
    }
}

/// A simple visitor that searches a subtree for `var.persisted?` calls.
/// Used by `subtree_checks_persisted` to match RuboCop's VariableForce behavior
/// of finding persisted? references anywhere in a scope, not just the next statement.
/// NOTE: Only matches `.persisted?` (send), NOT `&.persisted?` (csend).
/// RuboCop's call_to_persisted? checks `node.send_type?` which excludes csend.
///
/// Also matches RuboCop's `call_to_persisted?` quirk: when a reference to the variable
/// is a parenthesized call inside an if-body whose condition is ANY `.persisted?` call,
/// the reference is treated as having a persisted? check. This is because RuboCop's
/// `call_to_persisted?` replaces the node with `node.parent.condition` when
/// `node.parenthesized_call? && node.parent.if_type?`, effectively checking the
/// if-condition instead of the reference itself.
struct PersistedFinder<'v> {
    var_name: &'v [u8],
    found: bool,
}

impl PersistedFinder<'_> {
    /// Check if a node is a `.persisted?` call (regular send, not csend).
    fn is_persisted_call(node: &ruby_prism::CallNode<'_>) -> bool {
        let is_csend = node
            .call_operator_loc()
            .is_some_and(|loc| loc.end_offset() - loc.start_offset() == 2);
        !is_csend && node.name().as_slice() == b"persisted?"
    }

    /// Check if a subtree contains any reference to the target variable.
    fn subtree_has_var_ref(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
        let mut finder = VarRefFinder {
            var_name,
            found: false,
        };
        finder.visit(node);
        finder.found
    }
}

impl<'pr> Visit<'pr> for PersistedFinder<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if self.found {
            return;
        }
        if Self::is_persisted_call(node) {
            if let Some(recv) = node.receiver() {
                if SaveBangVisitor::node_is_var(&recv, self.var_name) {
                    self.found = true;
                    return;
                }
            }
        }
        // Continue visiting children
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        if self.found {
            return;
        }
        // RuboCop's call_to_persisted? quirk: when a variable reference is a parenthesized
        // call inside an if-body, and the if-condition is ANY `.persisted?` call, the
        // reference is treated as having a persisted? check. Check if the if-condition
        // contains `.persisted?` and the if-body references our target variable.
        let condition_has_persisted = {
            let pred = node.predicate();
            if let Some(call) = pred.as_call_node() {
                Self::is_persisted_call(&call)
            } else {
                false
            }
        };
        if condition_has_persisted {
            if let Some(stmts) = node.statements() {
                for stmt in stmts.body().iter() {
                    if Self::subtree_has_var_ref(&stmt, self.var_name) {
                        self.found = true;
                        return;
                    }
                }
            }
        }
        // Continue visiting children (visit condition, body, else)
        ruby_prism::visit_if_node(self, node);
    }
}

/// Simple visitor that checks if a subtree contains a reference to a variable name.
struct VarRefFinder<'v> {
    var_name: &'v [u8],
    found: bool,
}

impl<'pr> Visit<'pr> for VarRefFinder<'_> {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        if node.name().as_slice() == self.var_name {
            self.found = true;
        }
    }
}

/// A visitor that finds local variable names assigned to create call results.
/// Used for pre-scanning scope bodies to identify variables that may have
/// persisted? checked later, even when the assignment is nested inside compound
/// expressions (if predicates, and/or nodes, etc.).
struct CreateVarFinder {
    found: Vec<Vec<u8>>,
}

impl CreateVarFinder {
    fn value_is_create(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            let name = call.name().as_slice();
            return CREATE_PERSIST_METHODS.contains(&name);
        }
        false
    }
}

impl<'pr> Visit<'pr> for CreateVarFinder {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        if Self::value_is_create(&node.value()) {
            self.found.push(node.name().as_slice().to_vec());
        }
        // Continue visiting children for nested assignments
        ruby_prism::visit_local_variable_write_node(self, node);
    }
}

impl<'pr> Visit<'pr> for SaveBangVisitor<'_, '_> {
    // ── CallNode: check if this is a persist method ──────────────────────

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if let Some(is_create) = self.classify_persist_call(node) {
            self.process_persist_call(node, is_create);
        }

        // Continue visiting children (e.g., receiver, arguments, block)
        // Receivers do NOT get Argument context — in RuboCop, a persist call
        // that is the receiver of another method (e.g., `object.save.to_s`)
        // is still flagged because the return value is not meaningfully checked.
        // Exceptions:
        // - `.persisted?` counts as checking the result (checked_immediately?)
        // - `!` / `not` operator counts as condition/compound boolean (single_negative?)
        if let Some(recv) = node.receiver() {
            let method_name = node.name().as_slice();
            // checked_immediately?: only `.persisted?` (send), not `&.persisted?` (csend).
            // RuboCop's call_to_persisted? checks node.send_type? which excludes csend.
            let is_csend_call = node
                .call_operator_loc()
                .is_some_and(|loc| loc.end_offset() - loc.start_offset() == 2);
            let is_persisted_check = method_name == b"persisted?" && !is_csend_call;
            let is_negation = method_name == b"!" && node.arguments().is_none();
            let is_setter = is_setter_method(method_name);

            if is_persisted_check {
                // persisted? on the result means the return value IS checked
                self.suppress_create_assignment = true;
                self.context_stack.push(Context::Argument);
                self.visit(&recv);
                self.context_stack.pop();
                self.suppress_create_assignment = false;
            } else if is_negation {
                // `!object.save` / `not object.save` — RuboCop treats this as
                // single_negative? which is part of condition/compound boolean.
                self.context_stack.push(Context::Condition);
                self.visit(&recv);
                self.context_stack.pop();
            } else if is_setter {
                // Setter method (e.g., `create.multipart = true`): RuboCop's
                // return_value_assigned? treats setter calls as assignments via
                // SendNode#assignment? (alias for setter_method?). The persist
                // call's return value is used to set an attribute, so it's exempt.
                self.context_stack.push(Context::Assignment);
                self.visit(&recv);
                self.context_stack.pop();
            } else {
                // Non-persisted? receiver: push VoidStatement so persist calls as receivers
                // of method chains are always flagged, regardless of outer context.
                // RuboCop evaluates each persist call independently — being a receiver of
                // another method (e.g., `save.to_s`, `create.one`) doesn't exempt it.
                self.context_stack.push(Context::VoidStatement);
                self.visit(&recv);
                self.context_stack.pop();
            }
        }

        if let Some(args) = node.arguments() {
            // Clear in_compound_boolean when visiting arguments of a non-boolean
            // method call. In RuboCop, compound_boolean? checks the node's FIRST
            // non-begin ancestor. When create is inside a method call's arguments
            // within a boolean (e.g., `x || version != create(arg)`), the first
            // ancestor is the `!=` send, not the or — so compound_boolean doesn't
            // apply. But when create is a direct operand of the boolean (e.g.,
            // `log(find || create)`), the first ancestor IS the or node.
            // Clearing the flag at the method argument boundary ensures only
            // direct boolean operands see compound_boolean.
            let saved_compound = self.in_compound_boolean;
            self.in_compound_boolean = false;
            self.context_stack.push(Context::Argument);
            self.visit_arguments_node(&args);
            self.context_stack.pop();
            self.in_compound_boolean = saved_compound;
        }

        if let Some(block) = node.block() {
            if let Some(block_arg) = block.as_block_argument_node() {
                self.visit_block_argument_node(&block_arg);
            } else if let Some(block_node) = block.as_block_node() {
                self.visit_block_node(&block_node);
            }
        }
    }

    // ── BlockNode / LambdaNode: body has implicit return semantics ───────

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        if let Some(params) = node.parameters() {
            self.visit(&params);
        }
        // Reset in_local_assignment and in_transparent_container at block boundaries.
        // Blocks create new scopes in RuboCop's VariableForce. An outer `d3 = expr do ... end`
        // sets in_local_assignment for `d3`, but the block body's statements are in a
        // different scope — multi-write or other assignments inside the block should
        // not inherit the outer in_local_assignment flag.
        // Similarly, in_transparent_container (set by enclosing hash/array) should not
        // leak through block boundaries. E.g., `{ articles: files.map { Tempfile.create { } } }`
        // — the Tempfile.create block is a separate scope, not inside the hash.
        let saved_local = self.in_local_assignment;
        let saved_transparent = self.in_transparent_container;
        self.in_local_assignment = false;
        self.in_transparent_container = false;
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                self.visit_statements_with_context(&stmts, true);
            } else {
                self.visit(&body);
            }
        }
        self.in_local_assignment = saved_local;
        self.in_transparent_container = saved_transparent;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        if let Some(params) = node.parameters() {
            self.visit(&params);
        }
        let saved_local = self.in_local_assignment;
        let saved_transparent = self.in_transparent_container;
        self.in_local_assignment = false;
        self.in_transparent_container = false;
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                self.visit_statements_with_context(&stmts, true);
            } else {
                self.visit(&body);
            }
        }
        self.in_local_assignment = saved_local;
        self.in_transparent_container = saved_transparent;
    }

    // ── DefNode: body has implicit return semantics ──────────────────────

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if let Some(params) = node.parameters() {
            self.visit_parameters_node(&params);
        }
        // RuboCop's implicit_return? only matches `def` (instance methods), not `defs`
        // (singleton methods like `def self.foo`). In Prism, singleton methods are DefNode
        // with a receiver. Only instance methods (no receiver) get implicit return semantics.
        let is_instance_method = node.receiver().is_none();
        let saved_local = self.in_local_assignment;
        let saved_transparent = self.in_transparent_container;
        self.in_local_assignment = false;
        self.in_transparent_container = false;
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                self.visit_statements_with_context(&stmts, is_instance_method);
            } else {
                self.visit(&body);
            }
        }
        self.in_local_assignment = saved_local;
        self.in_transparent_container = saved_transparent;
    }

    // ── StatementsNode: default (not in method/block) ────────────────────
    // This handles StatementsNode that appears as a child of nodes other
    // than def/block/lambda (e.g., if body, begin body, class body).

    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        // For StatementsNode not inside method/block, all children are void.
        // But def/block/lambda override this to use visit_statements_with_context.
        let body: Vec<_> = node.body().iter().collect();

        for (i, stmt) in body.iter().enumerate() {
            let suppress = self.should_suppress_create(stmt, &body, i);
            if suppress {
                self.suppress_create_assignment = true;
            }

            self.context_stack.push(Context::VoidStatement);
            self.visit(stmt);
            self.context_stack.pop();

            if suppress {
                self.suppress_create_assignment = false;
            }
        }
    }

    // ── IfNode / UnlessNode: predicate is condition context ──────────────

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        // The predicate is in condition context
        let predicate = node.predicate();
        self.context_stack.push(Context::Condition);
        self.visit(&predicate);
        self.context_stack.pop();

        // Get predicate source bytes for RuboCop value-equality comparison.
        // RuboCop's in_condition_or_compound_boolean? uses `node == deparenthesize(parent.condition)`
        // which compares by structural equality (==), not identity (equal?). When a persist call
        // in the if-body is structurally identical to the if-condition (e.g., both are bare `save`),
        // RuboCop treats the body statement as being in condition context, exempting modify methods.
        let pred_src = &self.source.as_bytes()
            [predicate.location().start_offset()..predicate.location().end_offset()];

        // The then-body and else-body do NOT inherit ImplicitReturn from
        // outer scopes. RuboCop's implicit_return? only recognizes statements
        // that are direct children of def/block bodies — not statements nested
        // inside if/else branches. Push VoidStatement to prevent leakage.
        // Exception: statements matching the predicate source get Condition context.
        if let Some(stmts) = node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            for (i, stmt) in body.iter().enumerate() {
                let stmt_src = &self.source.as_bytes()
                    [stmt.location().start_offset()..stmt.location().end_offset()];
                let ctx = if stmt_src == pred_src {
                    Context::Condition
                } else {
                    Context::VoidStatement
                };

                let suppress = self.should_suppress_create(stmt, &body, i);
                if suppress {
                    self.suppress_create_assignment = true;
                }

                self.context_stack.push(ctx);
                self.visit(stmt);
                self.context_stack.pop();

                if suppress {
                    self.suppress_create_assignment = false;
                }
            }
        }

        if let Some(subsequent) = node.subsequent() {
            self.context_stack.push(Context::VoidStatement);
            self.visit(&subsequent);
            self.context_stack.pop();
        }
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        // The predicate is in condition context
        let predicate = node.predicate();
        self.context_stack.push(Context::Condition);
        self.visit(&predicate);
        self.context_stack.pop();

        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }

        if let Some(else_clause) = node.else_clause() {
            self.visit_else_node(&else_clause);
        }
    }

    // ── CaseNode: predicate is condition context ─────────────────────────

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        if let Some(predicate) = node.predicate() {
            self.context_stack.push(Context::Condition);
            self.visit(&predicate);
            self.context_stack.pop();
        }

        for condition in node.conditions().iter() {
            self.visit(&condition);
        }

        if let Some(else_clause) = node.else_clause() {
            self.visit_else_node(&else_clause);
        }
    }

    // ── Assignment nodes: RHS is assignment context ──────────────────────

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        self.in_local_assignment = true;
        let saved_var = self.current_local_var_name.take();
        self.current_local_var_name = Some(node.name().as_slice().to_vec());
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
        self.current_local_var_name = saved_var;
        self.in_local_assignment = false;
    }

    fn visit_instance_variable_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableWriteNode<'pr>,
    ) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_class_variable_write_node(&mut self, node: &ruby_prism::ClassVariableWriteNode<'pr>) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_global_variable_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableWriteNode<'pr>,
    ) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        // Don't set in_local_assignment for ||= — RuboCop's VariableForce
        // check_assignment returns early for or_asgn because right_assignment_node
        // gets the lvasgn target, not the RHS value. So create-in-||= is exempt.
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        // Same as ||= — don't flag create in &&= assignments.
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_instance_variable_or_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableOrWriteNode<'pr>,
    ) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_instance_variable_and_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableAndWriteNode<'pr>,
    ) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_multi_write_node(&mut self, node: &ruby_prism::MultiWriteNode<'pr>) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_constant_path_write_node(&mut self, node: &ruby_prism::ConstantPathWriteNode<'pr>) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    // ── Missing or/and write nodes: push Assignment context ──

    fn visit_class_variable_or_write_node(
        &mut self,
        node: &ruby_prism::ClassVariableOrWriteNode<'pr>,
    ) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_class_variable_and_write_node(
        &mut self,
        node: &ruby_prism::ClassVariableAndWriteNode<'pr>,
    ) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_global_variable_or_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableOrWriteNode<'pr>,
    ) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_global_variable_and_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableAndWriteNode<'pr>,
    ) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_constant_or_write_node(&mut self, node: &ruby_prism::ConstantOrWriteNode<'pr>) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_constant_and_write_node(&mut self, node: &ruby_prism::ConstantAndWriteNode<'pr>) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_constant_path_or_write_node(
        &mut self,
        node: &ruby_prism::ConstantPathOrWriteNode<'pr>,
    ) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_constant_path_and_write_node(
        &mut self,
        node: &ruby_prism::ConstantPathAndWriteNode<'pr>,
    ) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_constant_path_operator_write_node(
        &mut self,
        node: &ruby_prism::ConstantPathOperatorWriteNode<'pr>,
    ) {
        // op_asgn is in SHORTHAND_ASSIGNMENTS, so assignment? returns true
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_index_or_write_node(&mut self, node: &ruby_prism::IndexOrWriteNode<'pr>) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_index_and_write_node(&mut self, node: &ruby_prism::IndexAndWriteNode<'pr>) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_index_operator_write_node(&mut self, node: &ruby_prism::IndexOperatorWriteNode<'pr>) {
        // op_asgn is in SHORTHAND_ASSIGNMENTS, so assignment? returns true
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_call_operator_write_node(&mut self, node: &ruby_prism::CallOperatorWriteNode<'pr>) {
        // Visit receiver (e.g., Student.create in Student.create.lessons += [...])
        // Receiver is in void context — the persist call's return value is used as
        // a receiver of the read method, not meaningfully checked.
        if let Some(receiver) = node.receiver() {
            self.context_stack.push(Context::VoidStatement);
            self.visit(&receiver);
            self.context_stack.pop();
        }
        // op_asgn is in SHORTHAND_ASSIGNMENTS, so assignment? returns true
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_call_or_write_node(&mut self, node: &ruby_prism::CallOrWriteNode<'pr>) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_call_and_write_node(&mut self, node: &ruby_prism::CallAndWriteNode<'pr>) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    // ── Operator-write nodes (+=, -=, etc.): RHS is void context ──
    // RuboCop's assignable_node doesn't handle op_asgn, so persist calls
    // in operator-write values are NOT treated as assigned — they're flagged.

    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        // RuboCop's `return_value_assigned?` checks `assignable_node(node).parent.assignment?`.
        // `op_asgn` is in SHORTHAND_ASSIGNMENTS, so `.assignment?` returns true.
        // Persist calls in operator-write values ARE exempt (return value captured).
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_instance_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableOperatorWriteNode<'pr>,
    ) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_class_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::ClassVariableOperatorWriteNode<'pr>,
    ) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_global_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableOperatorWriteNode<'pr>,
    ) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    // ── ReturnNode / NextNode: arguments are explicit return context ─────

    fn visit_return_node(&mut self, node: &ruby_prism::ReturnNode<'pr>) {
        if let Some(args) = node.arguments() {
            self.context_stack.push(Context::ExplicitReturn);
            self.visit_arguments_node(&args);
            self.context_stack.pop();
        }
    }

    fn visit_next_node(&mut self, node: &ruby_prism::NextNode<'pr>) {
        if let Some(args) = node.arguments() {
            self.context_stack.push(Context::ExplicitReturn);
            self.visit_arguments_node(&args);
            self.context_stack.pop();
        }
    }

    // ── YieldNode / SuperNode: arguments are in argument context ──────────

    fn visit_yield_node(&mut self, node: &ruby_prism::YieldNode<'pr>) {
        // RuboCop's argument? only checks send_type?/csend_type? parents.
        // yield is NOT send_type?, so yield args are NOT in argument context.
        // Also, yield breaks the implicit return chain — even if yield is the last
        // statement in a block (ImplicitReturn context), the create inside yield's
        // args is NOT exempt. Push VoidStatement to override inherited context.
        if let Some(args) = node.arguments() {
            self.context_stack.push(Context::VoidStatement);
            self.visit_arguments_node(&args);
            self.context_stack.pop();
        }
    }

    fn visit_super_node(&mut self, node: &ruby_prism::SuperNode<'pr>) {
        // RuboCop's argument? only checks send_type?/csend_type? parents.
        // super is NOT send_type?, so super args are NOT in argument context.
        // Push VoidStatement to override inherited context (same as yield).
        if let Some(args) = node.arguments() {
            self.context_stack.push(Context::VoidStatement);
            self.visit_arguments_node(&args);
            self.context_stack.pop();
        }
        if let Some(block) = node.block() {
            if let Some(block_node) = block.as_block_node() {
                self.visit_block_node(&block_node);
            }
        }
    }

    // ── And/Or nodes: both children are condition context ────────────────

    fn visit_and_node(&mut self, node: &ruby_prism::AndNode<'pr>) {
        let saved = self.in_compound_boolean;
        self.in_compound_boolean = true;
        self.context_stack.push(Context::Condition);
        self.visit(&node.left());
        self.visit(&node.right());
        self.context_stack.pop();
        self.in_compound_boolean = saved;
    }

    fn visit_or_node(&mut self, node: &ruby_prism::OrNode<'pr>) {
        // RuboCop's implicit_return? uses find_method_with_sibling_index which
        // increments sibling_index when walking through or_type? parents. The
        // formula `method.children.size == node.sibling_index + sibling_index`
        // means only the RIGHT child (index 1) of an or expression qualifies as
        // implicit return. The LEFT child (index 0) does NOT — so create on the
        // left side of || in implicit return is still flagged as compound_boolean.
        //
        // For ExplicitReturn: `find_method_with_sibling_index` also walks through
        // or, and explicit_return? checks the parent similarly. Both children
        // are exempt from explicit return (the return applies to the whole or expr).
        //
        // For Argument: both children inherit — the or result is used as an argument.
        //
        // Assignment does NOT inherit through || — RuboCop's return_value_assigned?
        // uses assignable_node which only walks through hash/array parents, NOT
        // through or/and nodes.
        let saved = self.in_compound_boolean;
        self.in_compound_boolean = true;
        let ctx = self.current_context();
        match ctx {
            Some(Context::ImplicitReturn) => {
                // Only the RIGHT child inherits ImplicitReturn.
                // Left child gets Condition (compound_boolean context).
                self.context_stack.push(Context::Condition);
                self.visit(&node.left());
                self.context_stack.pop();
                self.visit(&node.right());
            }
            Some(Context::ExplicitReturn) => {
                // RuboCop's explicit_return? uses assignable_node which only walks
                // hash/array parents, NOT or/and. So create inside `||` inside `return`
                // is NOT exempt by explicit_return?. Both children get Condition context
                // (compound_boolean path).
                self.context_stack.push(Context::Condition);
                self.visit(&node.left());
                self.visit(&node.right());
                self.context_stack.pop();
            }
            Some(Context::Argument) => {
                // Both children inherit — the || result is being used as an argument
                self.visit(&node.left());
                self.visit(&node.right());
            }
            _ => {
                // VoidStatement, Assignment, Condition, or None:
                // Children are in condition/boolean context.
                self.context_stack.push(Context::Condition);
                self.visit(&node.left());
                self.visit(&node.right());
                self.context_stack.pop();
            }
        }
        self.in_compound_boolean = saved;
    }

    // ── Array / Hash literals: children are collection context ───────────

    // Arrays, hashes, and keyword hashes are transparent for context.
    // Their elements inherit the parent context, matching RuboCop's `assignable_node`
    // which climbs through array/hash parents to apply exemption checks at the
    // enclosing expression level. For example, `[save]` in void context still
    // flags `save`, but `return [save]` or `x = [save]` exempts it.
    fn visit_array_node(&mut self, node: &ruby_prism::ArrayNode<'pr>) {
        // Don't propagate in_local_assignment through arrays/hashes.
        // RuboCop's VariableForce check_assignment checks `if rhs_node.send_type?` —
        // ArrayNode doesn't match, so create calls inside arrays in local assignments
        // are not flagged by VariableForce.
        // Also reset in_compound_boolean: RuboCop's in_condition_or_compound_boolean?
        // checks the FIRST non-begin ancestor. Array is not operator_keyword? or
        // conditional?, so compound_boolean doesn't apply inside arrays.
        // Also override Condition context: RuboCop's in_condition_or_compound_boolean?
        // doesn't see through arrays (array is not conditional? or operator_keyword?).
        // So elements inside arrays in Condition context should get VoidStatement.
        let saved_local = self.in_local_assignment;
        let saved_transparent = self.in_transparent_container;
        let saved_compound = self.in_compound_boolean;
        self.in_local_assignment = false;
        self.in_transparent_container = true;
        self.in_compound_boolean = false;
        let override_condition = matches!(self.current_context(), Some(Context::Condition));
        if override_condition {
            self.context_stack.push(Context::VoidStatement);
        }
        for element in node.elements().iter() {
            self.visit(&element);
        }
        if override_condition {
            self.context_stack.pop();
        }
        self.in_local_assignment = saved_local;
        self.in_transparent_container = saved_transparent;
        self.in_compound_boolean = saved_compound;
    }

    fn visit_hash_node(&mut self, node: &ruby_prism::HashNode<'pr>) {
        let saved_local = self.in_local_assignment;
        let saved_transparent = self.in_transparent_container;
        let saved_compound = self.in_compound_boolean;
        self.in_local_assignment = false;
        self.in_transparent_container = true;
        self.in_compound_boolean = false;
        for element in node.elements().iter() {
            self.visit(&element);
        }
        self.in_local_assignment = saved_local;
        self.in_transparent_container = saved_transparent;
        self.in_compound_boolean = saved_compound;
    }

    fn visit_keyword_hash_node(&mut self, node: &ruby_prism::KeywordHashNode<'pr>) {
        let saved_local = self.in_local_assignment;
        let saved_transparent = self.in_transparent_container;
        let saved_compound = self.in_compound_boolean;
        self.in_local_assignment = false;
        self.in_transparent_container = true;
        self.in_compound_boolean = false;
        for element in node.elements().iter() {
            self.visit(&element);
        }
        self.in_local_assignment = saved_local;
        self.in_transparent_container = saved_transparent;
        self.in_compound_boolean = saved_compound;
    }

    // ── BeginNode: body statements are in the parent's context ───────────

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }
        if let Some(rescue_clause) = node.rescue_clause() {
            self.visit_rescue_node(&rescue_clause);
        }
        if let Some(else_clause) = node.else_clause() {
            self.visit_else_node(&else_clause);
        }
        if let Some(ensure_clause) = node.ensure_clause() {
            self.visit_ensure_node(&ensure_clause);
        }
    }

    // ── Parentheses: transparent, pass through context ───────────────────

    fn visit_parentheses_node(&mut self, node: &ruby_prism::ParenthesesNode<'pr>) {
        // Parentheses are transparent for Condition context (e.g., `if(object.save)`),
        // because RuboCop's `in_condition_or_compound_boolean?` deparenthesizes conditions.
        // However, parentheses break argument?, return_value_assigned?, explicit_return?,
        // and implicit_return? checks in RuboCop because those check `node.parent.send_type?`
        // etc. and `begin` (from parens) is not a matching type.
        // So for Argument and Assignment contexts, push VoidStatement to prevent incorrect
        // exemption of parenthesized persist calls.
        let ctx = self.current_context();
        let needs_void = matches!(ctx, Some(Context::Argument) | Some(Context::Assignment));
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                if needs_void {
                    for stmt in stmts.body().iter() {
                        self.context_stack.push(Context::VoidStatement);
                        self.visit(&stmt);
                        self.context_stack.pop();
                    }
                } else {
                    for stmt in stmts.body().iter() {
                        self.visit(&stmt);
                    }
                }
            } else if needs_void {
                self.context_stack.push(Context::VoidStatement);
                self.visit(&body);
                self.context_stack.pop();
            } else {
                self.visit(&body);
            }
        }
    }

    // ── ClassNode / ModuleNode: body is void context (not method/block) ──

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        if let Some(superclass) = node.superclass() {
            self.visit(&superclass);
        }
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                self.visit_statements_with_context(&stmts, false);
            } else {
                self.visit(&body);
            }
        }
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                self.visit_statements_with_context(&stmts, false);
            } else {
                self.visit(&body);
            }
        }
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        self.visit(&node.expression());
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                self.visit_statements_with_context(&stmts, false);
            } else {
                self.visit(&body);
            }
        }
    }

    // ── ProgramNode: top-level statements ────────────────────────────────

    fn visit_program_node(&mut self, node: &ruby_prism::ProgramNode<'pr>) {
        self.visit_statements_with_context(&node.statements(), false);
    }

    // ── WhileNode / UntilNode / ForNode: body is void context ────────────

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        self.context_stack.push(Context::Condition);
        self.visit(&node.predicate());
        self.context_stack.pop();

        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        self.context_stack.push(Context::Condition);
        self.visit(&node.predicate());
        self.context_stack.pop();

        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }
    }

    fn visit_for_node(&mut self, node: &ruby_prism::ForNode<'pr>) {
        self.visit(&node.collection());

        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }
    }

    // ── Ternary (IfNode handles this already) ────────────────────────────
    // Prism uses IfNode for ternary as well, so visit_if_node covers it.

    // ── RescueModifierNode: breaks implicit return and assignment chains ─

    fn visit_rescue_modifier_node(&mut self, node: &ruby_prism::RescueModifierNode<'pr>) {
        // rescue modifier breaks the implicit return and assignment chains.
        // In RuboCop, rescue_modifier is not in the accepted parent types for
        // implicit_return?, and assignable_node stops at rescue_mod (not array/hash).
        self.context_stack.push(Context::VoidStatement);
        self.visit(&node.expression());
        self.context_stack.pop();
        // The rescue value (e.g., `nil` in `save rescue nil`) is just a value
        self.visit(&node.rescue_expression());
    }

    // ── SplatNode / AssocSplatNode: breaks argument context chain ──────

    fn visit_splat_node(&mut self, node: &ruby_prism::SplatNode<'pr>) {
        // Splat (*expr) breaks the argument context chain in RuboCop.
        // assignable_node stops at the splat, and splat.argument? returns false.
        if let Some(expr) = node.expression() {
            self.context_stack.push(Context::VoidStatement);
            self.visit(&expr);
            self.context_stack.pop();
        }
    }

    fn visit_assoc_splat_node(&mut self, node: &ruby_prism::AssocSplatNode<'pr>) {
        // Keyword splat (**expr) breaks the argument context chain in RuboCop,
        // same as regular splat. kwsplat is not send_type?, so argument? returns false.
        // Example: `binding(**{}.update(kwargs))` — update is flagged.
        if let Some(expr) = node.value() {
            self.context_stack.push(Context::VoidStatement);
            self.visit(&expr);
            self.context_stack.pop();
        }
    }

    // ── Interpolation: children are in argument context ──────────────────

    fn visit_embedded_statements_node(&mut self, node: &ruby_prism::EmbeddedStatementsNode<'pr>) {
        // String interpolation does NOT suppress persist call offenses.
        // RuboCop treats `"#{object.save}"` the same as a void-context `save` call
        // because the return value is not meaningfully checked (only stringified).
        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SaveBang, "cops/rails/save_bang");
}
