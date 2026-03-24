use crate::cop::node_type::{
    CALL_NODE, CLASS_VARIABLE_READ_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, DEF_NODE,
    GLOBAL_VARIABLE_READ_NODE, INSTANCE_VARIABLE_READ_NODE, LOCAL_VARIABLE_READ_NODE,
    REQUIRED_PARAMETER_NODE, SELF_NODE, STATEMENTS_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/Delegate cop detects method definitions that simply delegate to another object,
/// suggesting the use of Rails' `delegate` macro instead.
///
/// ## Investigation findings (2026-03-10)
///
/// **FP root causes (49 FP):**
/// - Missing `module_function` check: RuboCop skips methods in modules that declare
///   `module_function`. Our cop was flagging these methods incorrectly.
/// - Missing `private :method_name` handling: The `is_private_or_protected` utility
///   only checked for standalone `private` keyword and inline `private def`, not
///   the `private :method_name` form that makes a specific method private after definition.
///
/// **FN root causes (136 FN):**
/// - Missing prefixed delegation detection: When `EnforceForPrefixed: true` (default),
///   `def bar_foo; bar.foo; end` should be flagged as a delegation that can use
///   `delegate :foo, to: :bar, prefix: true`. Our cop only matched exact method names.
///
/// **Fixes applied:**
/// - Added `module_function` detection via line scanning in enclosing scope
/// - Added `private :method_name` form detection
/// - Added prefixed delegation matching when `EnforceForPrefixed: true`
/// - Extended prefix skip (for `EnforceForPrefixed: false`) to all receiver types
///
/// ## Investigation (2026-03-14): FP=20
///
/// **FP root cause**: `is_in_module_function_scope` only scanned BACKWARDS from the def
/// for `module_function`. Patterns like `end; module_function :adapters` (inline after
/// the def's `end`) and `module_function :method_name` declared later in the module body
/// were missed. RuboCop's `module_function_declared?` walks ALL descendants of the
/// ancestor module — both before and after the def.
///
/// Fix: Added forward scan from the def line that checks if any subsequent line in the
/// same scope contains `module_function` (including `module_function :name` symbol forms).
///
/// ## Investigation (2026-03-15): FP=12
///
/// **FP root cause 1**: `is_in_module_function_scope` backward scan stopped at `class `
/// boundaries. When a class was nested inside a module that declared `module_function`
/// (e.g., `module Open4; module_function :open4; class SpawnError; def exitstatus`),
/// the backward scan would hit `class SpawnError` and break before reaching
/// `module_function :open4`. RuboCop's `module_function_declared?` checks ALL ancestors.
///
/// Fix: Changed backward scan to expand the search depth when crossing class boundaries,
/// so `module_function` at the outer module level is still found.
///
/// **FP root cause 2**: Endless methods (`def foo() = expr`) were flagged. RuboCop's
/// NodePattern matches `(def _method_name _args (send ...))` which in Parser gem doesn't
/// match endless defs. In Prism, endless defs have `equal_loc().is_some()`.
///
/// Fix: Skip defs with `equal_loc().is_some()` (endless method syntax).
///
/// **FP root cause 3**: `is_private_or_protected` (in util.rs) didn't match `private `
/// with a trailing space on its own line. The check compared against exact bytes
/// `b"private"` and specific continuations (`\n`, `\r`, ` #`) but not trailing spaces.
///
/// Fix: Added `starts_with(b"private ")` match that validates the remainder is
/// only whitespace (to avoid matching `private :method_name` or `private def foo`).
///
/// ## Investigation (2026-03-15): FP=1, FN=102
///
/// **FN root cause 1 (~majority)**: Endless methods (`def foo = bar.foo`) were incorrectly
/// skipped. The previous investigation assumed RuboCop's `(def ...)` NodePattern didn't
/// match endless defs, but corpus data proves RuboCop DOES flag them. In Prism, endless
/// methods have the body as a direct child (not wrapped in StatementsNode).
///
/// Fix: Removed the `equal_loc().is_some()` early return. Added fallback path that
/// handles the body as a direct CallNode when it's not a StatementsNode.
///
/// **FN root cause 2**: Prefixed delegation via `self.class` receiver
/// (e.g., `def class_name; self.class.name; end`) was not detected. `get_receiver_name`
/// only returned names for receiverless calls, but `self.class` has a receiver (`self`).
/// RuboCop's `determine_prefixed_method_receiver_name` returns `receiver.method_name`
/// for send nodes, which would be `"class"` for `self.class`.
///
/// Fix: Added handling in `get_receiver_name` for call nodes with a `self` receiver,
/// returning the method name (e.g., `"class"` for `self.class`).
///
/// **FP (1, antiwork/gumroad)**: `def to_stripejs_customer_id; to_stripejs_customer.id; end`
/// flagged by nitrocop but not RuboCop. Likely a private/module_function scope issue
/// in the full file that our line-based scanning doesn't detect. Cannot verify without
/// corpus file access.
///
/// **Remaining FNs**: 102 FNs in corpus, mostly in files not locally available.
/// Many are likely the endless method and self.class patterns now fixed. Others may
/// involve scope/visibility patterns not yet detected by line-based scanning.
///
/// ## Investigation (2026-03-15): FP=2, FN=28
///
/// **FN root cause 1**: `is_private_symbol_arg` was too broad — it matched
/// `private :method_name, :other` (multi-symbol calls). RuboCop's `VisibilityHelp`
/// pattern `(send nil? VISIBILITY_SCOPES (sym %method_name))` only matches single-symbol
/// `private :method_name`. Multi-symbol calls like `private :[]=, :set_element` do NOT
/// make the method private for delegate purposes.
///
/// Fix: Added comma check in `is_private_symbol_arg` to reject multi-symbol calls.
///
/// **FN root cause 2**: `is_in_module_function_scope` forward scan was too broad:
/// (a) matched `module_function` in comments (e.g., `# module_function...`),
/// (b) matched `module_function` in nested scopes at deeper indentation (e.g.,
/// `namespace :parallel do; module X; module_function; end; end`).
///
/// Fix: Added comment filtering (strip `#`-prefixed content) and indent check
/// (`indent <= def_col`) in the forward scan to restrict matches to the same scope.
///
/// **FP 1 (antiwork/gumroad)**: `def to_stripejs_customer_id; to_stripejs_customer.id; end`
/// correctly matched as prefixed delegation but RuboCop doesn't flag it. Without corpus
/// file access, cannot determine visibility context (likely private block earlier in file).
///
/// **FP 2 (palkan/anyway_config)**: `def clear() = value.clear` — endless method
/// delegation. RuboCop doesn't flag it. Without corpus access, cannot determine visibility
/// context (likely private block earlier in file).
///
/// ## Investigation (2026-03-16): FP=2, FN=24
///
/// **FN root cause**: `is_in_module_function_scope` forward scan used substring matching
/// (`windows().any()`) to detect `module_function`. This falsely matched identifiers
/// containing `module_function` as a substring, e.g., `register_module_function`,
/// `module_function?`, `make_module_function`. This was the primary FN source —
/// particularly in yard (10 FNs), where `lib/yard/handlers/base.rb` has delegation
/// methods like `def owner; parser.owner end` followed later by method
/// `def register_module_function(object)` which contains the substring.
///
/// Fix: Replaced `windows()` substring matching with `has_module_function_token()`
/// that checks word boundaries — `module_function` must be preceded and followed by
/// non-identifier characters (not alphanumeric, `_`, `?`, `!`).
///
/// **FP 1 & 2**: Both FPs remain — they are caused by visibility context (private
/// block earlier in the file) that our line-based scanning doesn't detect. The methods
/// ARE valid delegation patterns that RuboCop flags when public, confirmed via testing.
///
/// ## Investigation (2026-03-18): FP=2, FN=14
///
/// **FP root causes (2 FP — gumroad and anyway_config)**:
/// Both cases have `module_function` declared in an OUTER ancestor module, AFTER a nested
/// class/module definition in that outer module. The forward scan in `is_in_module_function_scope`
/// broke as soon as it encountered a `class`/`module` at `indent < def_col`, stopping before
/// it could reach the `module_function` in the outer scope.
///
/// Example (gumroad): `def to_stripejs_customer_id` inside `module ExtensionMethods` (indent 4).
/// After `ExtensionMethods` ends, `class StripeJs` appears at indent 2 (the outer scope).
/// The scan stopped at `class StripeJs`, never reaching `module_function` at indent 2 in
/// `module StripePaymentMethodHelper`. RuboCop's `module_function_declared?` checks ALL
/// ancestors, so it finds it.
///
/// Fix: Changed forward scan to track `sibling_scope_depth`. When `class`/`module` at
/// `indent < def_col` is encountered, increment depth (skip its body). When the matching
/// `end` is seen, decrement. Only check `module_function` when `sibling_scope_depth == 0`.
/// This allows scanning past sibling class/module bodies to find `module_function` in the
/// outer ancestor scope.
///
/// **FN root causes (14 FN — mongomapper, rouge, rails, coderay, etc.)**:
/// All FNs involve delegations via `def foo; self.class.foo; end` inside a module that
/// has a sibling `module ClassMethods` (or `class << self`) at the same indent level.
/// The sibling has `private` declared inside it at the same indent. The forward scan in
/// `is_private_or_protected` (which scans top-to-bottom) set `in_private = true` when
/// it encountered `private` inside the sibling, and never reset it when the sibling's
/// `end` was reached (because `end` at `indent == def_col` did not reset in_private).
///
/// Example (mongomapper): `module ClassMethods` at indent 6, with `private` at indent 6
/// inside it. After `end` of ClassMethods, `def associations` at indent 6 was incorrectly
/// considered private.
///
/// Fix: Added `peer_scope_depth` tracking in `is_private_or_protected`. When `class`/`module`
/// at `indent == def_col` is encountered, increment depth (entering a peer scope). When `end`
/// at `indent == def_col` decrements it to 0 (exiting peer scope), `in_private` updates are
/// skipped while inside the peer scope. This prevents `private` from inside sibling
/// class/modules from bleeding into instance methods at the same level.
///
/// ## Investigation (2026-03-18): FP=1, FN=2
///
/// **FP (rubocop, line 88)**: Already fixed by prior `is_private_or_protected` improvements.
/// `private` at same indent as `def` in deeply nested class correctly detected.
///
/// **FN (aruba, line 149)**: Already fixed by prior `peer_scope_depth` improvements.
/// `def mode; @announcer.mode; end` after `public` keyword correctly detected.
///
/// **FN (asciidoctor, line 66)**: `def now; ::Time.now; end` inside `if/else` block after
/// `private`. RuboCop's `node_visibility` uses AST sibling checks — a `def` inside an
/// `if/else` body is NOT a sibling of `private` in the class body, so RuboCop considers
/// it public. Our line-based `is_private_or_protected` incorrectly set `in_private = true`
/// because `private` at indent 4 <= def_col 6.
///
/// Fix: Added `is_inside_conditional_block()` check in the delegate cop. After
/// `is_private_or_protected` returns true, scan backwards from the def for block-opening
/// keywords (if/unless/case/else/elsif/while/etc.) at lower indent. If found, the def
/// is inside a conditional block and `private` doesn't apply per RuboCop's AST semantics.
///
/// ## Investigation (2026-03-19): FP=8, FN=0
///
/// **FP root cause (all 8)**: `is_inside_conditional_block()` backward scan didn't stop
/// at `end` keywords at indent < def_col. It scanned through sibling method/block bodies
/// and falsely matched conditional keywords (rescue/ensure/elsif/if) from INSIDE those
/// other methods. Examples:
/// - rails/rails: `ensure` at indent 2 inside a test method → falsely marked `def mkdir`
///   (at indent 4 after `private`) as inside a conditional block.
/// - ruby/debug: `elsif` at indent 6 inside `setup_sigdump` → falsely marked `private def
///   config` (at indent 12) as inside a conditional.
/// - antiwork/gumroad: `rescue` at indent 2 inside other methods → falsely marked
///   `def paypal_api` (at indent 4 after `private`) as inside a conditional.
///
/// Fix: Added `end` at indent < def_col as a stop condition in the backward scan of
/// `is_inside_conditional_block()`. When scanning backwards, an `end` at lower indent
/// closes a sibling scope — conditional keywords beyond it are in a different scope and
/// should not affect the current def.
///
/// ## Reverted fix attempt (2026-03-23, commit 0956d7b0)
///
/// Attempted to fix FP on parameter receivers and FN inside else blocks.
/// Introduced FP=1 on standard corpus; reverted in 1bf1bea3.
///
/// **FP=1 (is_inside_conditional_block overrides private in same branch):**
/// `def connection` (indent 6) inside an `else` branch (indent 4) preceded by
/// `private` (indent 6) in the same else branch. `is_private_or_protected`
/// returns true, but `is_inside_conditional_block` also returns true (finds
/// `else` at lower indent). The skip logic `is_private && !is_inside_conditional`
/// evaluates to false, so the private method gets flagged. But `private` is in
/// the SAME conditional branch as the def — it should still apply. Fix: when
/// `is_inside_conditional_block` is true, check whether `private` appears AFTER
/// the enclosing conditional keyword and BEFORE the def (same nesting level),
/// which means private still applies within that branch.
pub struct Delegate;

impl Cop for Delegate {
    fn name(&self) -> &'static str {
        "Rails/Delegate"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CLASS_VARIABLE_READ_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            DEF_NODE,
            GLOBAL_VARIABLE_READ_NODE,
            INSTANCE_VARIABLE_READ_NODE,
            LOCAL_VARIABLE_READ_NODE,
            REQUIRED_PARAMETER_NODE,
            SELF_NODE,
            STATEMENTS_NODE,
        ]
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
        let enforce_for_prefixed = config.get_bool("EnforceForPrefixed", true);

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        // Skip class/module methods (def self.foo)
        if def_node.receiver().is_some() {
            return;
        }

        // Collect parameter names (for argument forwarding check)
        let param_names: Vec<Vec<u8>> = if let Some(params) = def_node.parameters() {
            // Only support simple required positional parameters for forwarding
            let has_non_required = params.optionals().iter().next().is_some()
                || params.rest().is_some()
                || params.keywords().iter().next().is_some()
                || params.keyword_rest().is_some()
                || params.block().is_some();
            if has_non_required {
                return;
            }
            params
                .requireds()
                .iter()
                .filter_map(|p| {
                    p.as_required_parameter_node()
                        .map(|rp| rp.name().as_slice().to_vec())
                })
                .collect()
        } else {
            Vec::new()
        };

        // Body must be a single call expression.
        // For regular defs: body is StatementsNode with one statement.
        // For endless methods (def foo = expr): body is the call node directly.
        let body = match def_node.body() {
            Some(b) => b,
            None => return,
        };

        let call = if let Some(stmts) = body.as_statements_node() {
            let body_nodes: Vec<_> = stmts.body().iter().collect();
            if body_nodes.len() != 1 {
                return;
            }
            match body_nodes[0].as_call_node() {
                Some(c) => c,
                None => return,
            }
        } else {
            // Endless method: body is the call node directly
            match body.as_call_node() {
                Some(c) => c,
                None => return,
            }
        };

        // Check method name matching:
        // 1. Direct match: def foo; bar.foo; end
        // 2. Prefixed match (when EnforceForPrefixed): def bar_foo; bar.foo; end
        let def_name = def_node.name().as_slice();
        let call_name = call.name().as_slice();

        // Must have a receiver (delegating to another object)
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let name_matches_directly = call_name == def_name;
        let name_matches_prefixed = if enforce_for_prefixed && !name_matches_directly {
            // Check if def_name == receiver_name + "_" + call_name
            // Skip prefix check for `self` receiver (RuboCop returns '' for self prefix)
            if receiver.as_self_node().is_some() {
                false
            } else {
                let recv_name = get_receiver_name(&receiver);
                if let Some(rn) = recv_name {
                    let mut expected = rn;
                    expected.push(b'_');
                    expected.extend_from_slice(call_name);
                    expected == def_name
                } else {
                    false
                }
            }
        } else {
            false
        };

        if !name_matches_directly && !name_matches_prefixed {
            return;
        }

        // Safe navigation (&.) is ignored — Rails' delegate with allow_nil
        // has different semantics than safe navigation
        if call
            .call_operator_loc()
            .is_some_and(|op: ruby_prism::Location<'_>| op.as_slice() == b"&.")
        {
            return;
        }

        // Receiver must be a delegatable target:
        // - Instance variable (@foo.bar → delegate :bar, to: :foo)
        // - Simple method/local variable (foo.bar → delegate :bar, to: :foo)
        // - Constant (Setting.bar → delegate :bar, to: :Setting)
        // - self (self.bar → delegate :bar, to: :self)
        // - self.class (self.class.bar → delegate :bar, to: :class)
        // - Class/global variable (@@var.bar, $var.bar)
        // NOT: literals, arbitrary chained calls, etc.
        let is_delegatable_receiver = if receiver.as_instance_variable_read_node().is_some()
            || receiver.as_self_node().is_some()
            || receiver.as_class_variable_read_node().is_some()
            || receiver.as_global_variable_read_node().is_some()
        {
            true
        } else if let Some(recv_call) = receiver.as_call_node() {
            // self.class → delegate to :class
            if recv_call.name().as_slice() == b"class"
                && recv_call
                    .receiver()
                    .is_some_and(|r| r.as_self_node().is_some())
                && recv_call.arguments().is_none()
            {
                true
            } else {
                // Simple receiverless method call (acts as a local variable)
                recv_call.receiver().is_none()
                    && recv_call.arguments().is_none()
                    && recv_call.block().is_none()
            }
        } else if receiver.as_local_variable_read_node().is_some() {
            true
        } else {
            receiver.as_constant_read_node().is_some() || receiver.as_constant_path_node().is_some()
        };

        if !is_delegatable_receiver {
            return;
        }

        // Check argument forwarding: call args must match def params 1:1
        let call_arg_names: Vec<Vec<u8>> = if let Some(args) = call.arguments() {
            args.arguments()
                .iter()
                .filter_map(|a| {
                    a.as_local_variable_read_node()
                        .map(|lv| lv.name().as_slice().to_vec())
                })
                .collect()
        } else {
            Vec::new()
        };

        // Argument count must match and all must be simple lvar forwards
        if call_arg_names.len() != param_names.len() {
            return;
        }
        let call_arg_count = if let Some(args) = call.arguments() {
            args.arguments().iter().count()
        } else {
            0
        };
        if call_arg_count != param_names.len() {
            return;
        }
        // Each param must forward to matching lvar in same order
        for (param, arg) in param_names.iter().zip(call_arg_names.iter()) {
            if param != arg {
                return;
            }
        }

        // Should not have a block
        if call.block().is_some() {
            return;
        }

        // When EnforceForPrefixed is false, skip prefixed delegations
        // (e.g., `def foo_bar; foo.bar; end` where method starts with receiver name)
        // Must check all receiver types, not just CallNode.
        if !enforce_for_prefixed && !name_matches_directly {
            // If the name only matched via prefix, skip it
            return;
        }

        // Skip private/protected methods — RuboCop only flags public methods.
        // Exception: defs inside conditional blocks (if/unless/case/etc.) are NOT
        // affected by a preceding `private` keyword in RuboCop's AST-based sibling
        // check, because the def is a child of the conditional node, not the class body.
        if crate::cop::util::is_private_or_protected(source, node.location().start_offset())
            && !is_inside_conditional_block(source, node.location().start_offset())
        {
            return;
        }

        // Skip methods marked private via `private :method_name` after the def
        if is_private_symbol_arg(source, def_name, node.location().start_offset()) {
            return;
        }

        // Skip methods inside modules with `module_function` declared
        if is_in_module_function_scope(source, node.location().start_offset()) {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `delegate` to define delegations.".to_string(),
        ));
    }
}

/// Check if a def is inside a conditional/begin block at the class body level.
/// RuboCop's VisibilityHelp uses AST sibling checks — a `def` inside an if/else/case
/// block is NOT a sibling of `private`, so `private` doesn't affect it.
/// Returns true if the def is inside such a block.
fn is_inside_conditional_block(source: &SourceFile, def_offset: usize) -> bool {
    let (def_line, def_col) = source.offset_to_line_col(def_offset);
    let lines: Vec<&[u8]> = source.lines().collect();

    // Scan backwards from the def to find an enclosing block-opening keyword
    // at a LOWER indent than the def. If found before a class/module/end
    // boundary, the def is nested inside a conditional.
    for line in lines[..def_line.saturating_sub(1)].iter().rev() {
        let indent = line
            .iter()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count();
        let trimmed = &line[indent..];

        // Skip blank lines and comments
        if trimmed.is_empty() || trimmed.starts_with(b"#") {
            continue;
        }

        // Stop at class/module boundary at lower indent — we've left the scope
        if indent < def_col && (trimmed.starts_with(b"class ") || trimmed.starts_with(b"module ")) {
            return false;
        }

        // Stop at `def ` at same indent — reached another method definition
        if indent == def_col && trimmed.starts_with(b"def ") {
            return false;
        }

        // Stop at `end` at lower indent — crossed out of a sibling method/block
        // scope. Without this, conditional keywords (rescue/ensure/elsif/etc.)
        // from inside OTHER methods would falsely match.
        if indent < def_col
            && (trimmed == b"end"
                || trimmed.starts_with(b"end ")
                || trimmed.starts_with(b"end;")
                || trimmed.starts_with(b"end#"))
        {
            return false;
        }

        // Check for block-opening keywords at lower indent (the class body level).
        // These indicate the def is nested inside a conditional/loop/begin block.
        if indent < def_col
            && (trimmed.starts_with(b"if ")
                || trimmed.starts_with(b"unless ")
                || trimmed.starts_with(b"case ")
                || trimmed.starts_with(b"case\n")
                || trimmed.starts_with(b"while ")
                || trimmed.starts_with(b"until ")
                || trimmed.starts_with(b"for ")
                || trimmed.starts_with(b"begin")
                || trimmed == b"else"
                || trimmed.starts_with(b"else ")
                || trimmed.starts_with(b"elsif ")
                || trimmed.starts_with(b"when ")
                || trimmed.starts_with(b"rescue")
                || trimmed.starts_with(b"ensure"))
        {
            return true;
        }
    }
    false
}

/// Extract the receiver name as bytes for prefix checking.
/// Returns None if the receiver type doesn't support prefix matching.
fn get_receiver_name(receiver: &ruby_prism::Node<'_>) -> Option<Vec<u8>> {
    if let Some(recv_call) = receiver.as_call_node() {
        if recv_call.receiver().is_none() {
            return Some(recv_call.name().as_slice().to_vec());
        }
        // self.class → prefix is "class"
        if recv_call
            .receiver()
            .is_some_and(|r| r.as_self_node().is_some())
            && recv_call.arguments().is_none()
        {
            return Some(recv_call.name().as_slice().to_vec());
        }
    }
    if let Some(lv) = receiver.as_local_variable_read_node() {
        return Some(lv.name().as_slice().to_vec());
    }
    if let Some(iv) = receiver.as_instance_variable_read_node() {
        // ivar name includes @, e.g. @foo → prefix is "@foo"
        return Some(iv.name().as_slice().to_vec());
    }
    if let Some(cv) = receiver.as_class_variable_read_node() {
        return Some(cv.name().as_slice().to_vec());
    }
    if let Some(gv) = receiver.as_global_variable_read_node() {
        return Some(gv.name().as_slice().to_vec());
    }
    if let Some(cr) = receiver.as_constant_read_node() {
        return Some(cr.name().as_slice().to_vec());
    }
    if receiver.as_constant_path_node().is_some() {
        // For ConstantPathNode, extract source text
        let loc = receiver.location();
        return Some(loc.as_slice().to_vec());
    }
    None
}

/// Check if the method name appears as an argument to `private :method_name`
/// or `protected :method_name` after the method definition.
fn is_private_symbol_arg(source: &SourceFile, method_name: &[u8], def_offset: usize) -> bool {
    let (def_line, def_col) = source.offset_to_line_col(def_offset);
    let lines: Vec<&[u8]> = source.lines().collect();

    // Build the patterns: `private :method_name` and `protected :method_name`
    let mut private_pattern = b"private :".to_vec();
    private_pattern.extend_from_slice(method_name);
    let mut protected_pattern = b"protected :".to_vec();
    protected_pattern.extend_from_slice(method_name);

    // Search lines after the def for `private :method_name` or `protected :method_name`
    // Look within the same scope (stop at class/module boundary at lower indent).
    // `private :foo` typically appears right after the method's `end`, so we must
    // scan past `end` keywords at the same indent level.
    for line in lines.iter().skip(def_line) {
        let indent = line
            .iter()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count();
        let trimmed: Vec<u8> = line[indent..].to_vec();

        // Check for exact match or match followed by separator (newline, space, comma)
        for pattern in [&private_pattern, &protected_pattern] {
            if trimmed.starts_with(pattern) {
                let rest = &trimmed[pattern.len()..];
                // RuboCop's VisibilityHelp pattern only matches single-symbol calls:
                //   (send nil? VISIBILITY_SCOPES (sym %method_name))
                // So `private :foo` matches, but `private :foo, :bar` does NOT.
                // Only match when there's no comma (no additional symbol args).
                if rest.is_empty()
                    || rest[0] == b'\n'
                    || rest[0] == b'\r'
                    || rest[0] == b' '
                    || rest[0] == b'#'
                {
                    // Make sure there's no comma in the rest (multi-symbol call)
                    if !rest.contains(&b',') {
                        return true;
                    }
                }
            }
        }

        // Stop at scope boundary (class/module at same or lower indent)
        if indent <= def_col && (trimmed.starts_with(b"class ") || trimmed.starts_with(b"module "))
        {
            break;
        }
    }
    false
}

/// Check if the def is inside a module that has `module_function` declared.
/// This matches RuboCop's `module_function_declared?` which checks ancestors
/// for any `module_function` call (both standalone and inline) — BEFORE OR AFTER
/// the def. The key difference from the original: we scan both backwards AND
/// forwards for `module_function :method_name` (with symbol arg, appearing after).
///
/// Patterns detected:
/// - Standalone `module_function` (makes all following methods module functions)
/// - `module_function def method_name` (inline on same line)
/// - `module_function :method_name` (applies to specific method, often after the def)
/// - `end; module_function :name` (inline after def's `end`)
fn is_in_module_function_scope(source: &SourceFile, def_offset: usize) -> bool {
    let (def_line, def_col) = source.offset_to_line_col(def_offset);
    let lines: Vec<&[u8]> = source.lines().collect();

    /// Check if a trimmed line is any module_function form.
    fn is_module_function_line(trimmed: &[u8]) -> bool {
        trimmed == b"module_function"
            || trimmed.starts_with(b"module_function\n")
            || trimmed.starts_with(b"module_function\r")
            || trimmed.starts_with(b"module_function ")
            || trimmed.starts_with(b"module_function#")
    }

    // Scan backwards from the def line looking for `module_function`.
    // RuboCop's `module_function_declared?` checks ALL ancestors, so we must look
    // through class boundaries (a class nested inside a module can still have
    // module_function declared at the outer module level). We only stop at `module `
    // boundaries since module_function scope is module-level. When we cross a class
    // boundary, we expand the search to the outer indentation level.
    let mut current_col = def_col;
    for line in lines[..def_line].iter().rev() {
        let indent = line
            .iter()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count();
        let trimmed: Vec<u8> = line[indent..].to_vec();

        if indent <= current_col && is_module_function_line(&trimmed) {
            return true;
        }

        // Stop at module boundary at lower indentation (crossed into outer module scope)
        if indent < current_col && trimmed.starts_with(b"module ") {
            break;
        }

        // When hitting a class boundary at lower indentation, expand search to the
        // outer indentation so we can find module_function declared at the module level.
        if indent < current_col && trimmed.starts_with(b"class ") {
            current_col = indent;
        }
    }

    // Also check inline: the def line itself might have `module_function def foo`
    if let Some(line) = lines.get(def_line.saturating_sub(1)) {
        let trimmed: Vec<u8> = line
            .iter()
            .copied()
            .skip_while(|&b| b == b' ' || b == b'\t')
            .collect();
        if trimmed.starts_with(b"module_function def ") {
            return true;
        }
    }

    // RuboCop's `module_function_declared?` searches ALL descendants of the ancestor
    // module, including nodes that appear AFTER the def. Scan forward from the def's
    // line for any `module_function` reference, stopping at the enclosing scope boundary.
    //
    // This catches patterns like:
    //   `end; module_function :method_name`  (inline on same line as end)
    //   `module_function :method_name`        (after the def on its own line)
    //
    // Sibling scope skipping: when a class/module at indent < def_col appears AFTER the def
    // (e.g., `class StripeJs` after `def to_stripejs_customer_id`), we skip its body and
    // continue scanning for module_function in the outer scope. This matches RuboCop's
    // `each_ancestor(:module, :begin)` behavior which checks ALL ancestor modules.
    //
    // Example (gumroad pattern): module_function in outer module after nested class:
    //   module StripeHelper
    //     module ExtensionMethods
    //       def to_customer_id   ← def_col=4
    //         to_customer.id
    //       end
    //     end
    //     class StripeJs         ← sibling, skip over it
    //       ...
    //     end
    //     module_function        ← found in ancestor StripeHelper ✓
    //   end
    let mut sibling_scope_depth = 0usize;
    for line in lines[def_line..].iter() {
        let indent = line
            .iter()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count();
        let trimmed: &[u8] = &line[indent..];

        // At indent < def_col, track sibling class/module bodies.
        // When entering a sibling, increment depth to skip its contents.
        // When exiting a sibling (its end), decrement depth.
        // `end` at indent < def_col with sibling_scope_depth == 0 means we've exited
        // the def's own containing scope — but we continue scanning the outer scope
        // because module_function may be declared there (as in the gumroad pattern).
        if indent < def_col {
            if trimmed.starts_with(b"module ") || trimmed.starts_with(b"class ") {
                sibling_scope_depth += 1;
            } else if sibling_scope_depth > 0
                && (trimmed == b"end"
                    || trimmed.starts_with(b"end ")
                    || trimmed.starts_with(b"end;"))
            {
                sibling_scope_depth -= 1;
            }
        }

        // Only check for module_function when not inside a sibling scope body.
        // Check if this line contains `module_function` as an actual statement (not in a comment).
        // Only match at the same or enclosing scope level (indent <= def_col) to avoid
        // matching `module_function` in nested blocks, modules, or method calls.
        // Handles `module_function :name`, `end; module_function :name`, etc.
        // IMPORTANT: Use word boundary matching, not substring matching. Otherwise
        // identifiers like `register_module_function` or `module_function?` falsely trigger.
        if sibling_scope_depth == 0 && indent <= def_col {
            // Strip comment portion: find first `#` that's not inside a string
            let code_portion = if let Some(hash_pos) = trimmed.iter().position(|&b| b == b'#') {
                &trimmed[..hash_pos]
            } else {
                trimmed
            };
            if has_module_function_token(code_portion) {
                return true;
            }
        }
    }

    false
}

/// Check if a code portion contains `module_function` as a standalone token,
/// not as a substring of a larger identifier (e.g., `register_module_function`).
/// Returns true only when `module_function` is bounded by non-identifier characters
/// (or start/end of the slice).
fn has_module_function_token(code: &[u8]) -> bool {
    let needle = b"module_function";
    let nlen = needle.len();
    for window_start in 0..code.len() {
        if window_start + nlen > code.len() {
            break;
        }
        if &code[window_start..window_start + nlen] != needle.as_slice() {
            continue;
        }
        // Check preceding character is not an identifier char
        if window_start > 0 {
            let prev = code[window_start - 1];
            if prev.is_ascii_alphanumeric() || prev == b'_' {
                continue;
            }
        }
        // Check following character is not an identifier char
        if window_start + nlen < code.len() {
            let next_ch = code[window_start + nlen];
            if next_ch.is_ascii_alphanumeric()
                || next_ch == b'_'
                || next_ch == b'?'
                || next_ch == b'!'
            {
                continue;
            }
        }
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Delegate, "cops/rails/delegate");
}
