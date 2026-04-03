use crate::cop::shared::node_type::{
    AND_NODE, ARRAY_NODE, BLOCK_NODE, CALL_NODE, CALL_TARGET_NODE, CONSTANT_PATH_NODE,
    CONSTANT_READ_NODE, FALSE_NODE, FLOAT_NODE, HASH_NODE, INTEGER_NODE, KEYWORD_HASH_NODE,
    OR_NODE, REGULAR_EXPRESSION_NODE, SELF_NODE, STRING_NODE, SYMBOL_NODE, TRUE_NODE,
    X_STRING_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for redundant safe navigation calls.
///
/// ## Conversion with default literal (Case 6)
/// Detects `foo&.to_h || {}`, `foo&.to_a || []`, `foo&.to_i || 0`,
/// `foo&.to_f || 0.0`, `foo&.to_s || ''` patterns. These are redundant because
/// nil.to_h/to_a/to_i/to_f/to_s already return the same default values.
/// Also handles block form: `foo&.to_h { |k, v| [k, v] } || {}`.
///
/// The RuboCop `conversion_with_default?` node matcher checks that the default
/// value matches the nil-conversion result exactly (e.g., `to_i || 0` yes,
/// `to_i || 1` no; `to_s || ''` yes, `to_s || 'default'` no).
///
/// ### Root cause of 182 FN (corpus)
/// The cop was missing the `on_or` handler entirely. It only visited `CallNode`
/// but the conversion-with-default pattern requires visiting `OrNode` and
/// checking if the LHS is a safe-nav conversion call with a matching default RHS.
///
/// ## Corpus investigation (2026-03-19)
///
/// FP regression FP=1→2.
///
/// FP in DataDog/dd-trace-rb: `get_tag(...)&.to_i(16) || 0` — `to_i(16)` has an
/// argument (base-16 conversion). `nil.to_i` returns 0, but `nil.to_i(16)` raises
/// ArgumentError because NilClass#to_i takes no arguments. Fixed by checking that
/// the conversion method has no arguments before flagging.
///
/// FP in discourse/discourse: `mail[:cc]&.element&.addresses&.to_h { ... } || {}` —
/// chain of safe navigations. This may be a version discrepancy (discourse's
/// RuboCop version may not have the conversion_with_default check). Remaining FP=1
/// from extended corpus; FN=402 from missing features (InferNonNilReceiver, etc).
///
/// ## Corpus investigation (2026-03-21) — AllowedMethods in conditions
///
/// FN=177: Almost all are `receiver&.method` calls where `method` is one of the
/// AllowedMethods (is_a?, kind_of?, eql?, respond_to?, instance_of?, equal?) used
/// in conditional contexts (if, unless, while, until, ternary, &&, ||).
/// RuboCop flags these because in conditional contexts, `nil` (returned by
/// `nil&.method`) is falsy just like `false` (returned by `nil.method`), making
/// the `&.` redundant. Fixed by adding a `check_source` visitor that tracks
/// conditional predicate context (if/unless/while/until predicates, ternary
/// conditions, and &&/|| operands) and flags AllowedMethods `&.` calls within.
/// Exception: `respond_to?` with a nil-specific method argument (:to_a, :to_i,
/// :to_s, :to_f, :to_h) is NOT flagged even in conditions, because nil does
/// respond to those methods.
///
/// ## Corpus investigation (2026-03-23) — Standalone &&/|| and backtick receivers
///
/// FN=30: Many FNs were `&.is_a?` / `&.respond_to?` calls in `&&`/`||` expressions
/// that are NOT inside `if`/`unless` predicates (e.g., standalone boolean expressions
/// used as return values, assignments, or ternary conditions). RuboCop's `on_csend`
/// checks `allow_operator?` which returns true for ANY `&&`/`||` context, not just
/// `if` predicates. Fixed by adding `visit_and_node` and `visit_or_node` to the
/// visitor impl, so AllowedMethods in any `&&`/`||` context are flagged.
///
/// Also fixed: backtick literals (`` `cmd` ``) as receivers. Backtick always returns
/// a String (non-nil), so `&.` after backtick is redundant. Added `XStringNode` to
/// `is_non_nil_literal` check.
///
/// ## Corpus investigation (2026-03-23) — StatementsNode in parenthesized conditions
///
/// FN=30 remaining: `visit_conditional_subtree` did not handle `StatementsNode`, which
/// is the body of `ParenthesesNode` in Prism. Parenthesized `&&`/`||` expressions like
/// `(user&.is_a?(Admin) && ...)` have the structure Parens → StatementsNode → AndNode.
/// Without handling StatementsNode, the recursion stopped at the StatementsNode and
/// never reached the inner `&&`/`||` or `CallNode`. Fixed by adding StatementsNode
/// handling to `visit_conditional_subtree`. Remaining FNs likely from project-specific
/// config (custom AllowedMethods, InferNonNilReceiver: true) or niche patterns like
/// `rescue => self&.foo`.
///
/// ## Corpus investigation (2026-03-23) — FP=2, FN=1 final fixes
///
/// FP (otwarchive): `if @commentable.is_a?(Tag) || (@comment&.parent&.is_a?(Tag))` —
/// parentheses around the csend break the direct-parent chain. RuboCop's `check?`
/// requires the csend's IMMEDIATE parent to be a conditional, `&&`/`||`, or `!`.
/// When wrapped in parens, the parent is `begin` (not `||`), so RuboCop does not flag.
/// Fixed by adding a `direct` context parameter to `visit_conditional_subtree`;
/// parentheses reset `direct` to false, while `&&`/`||`/`!` restore it to true.
///
/// FP (discourse): `mail[:cc]&.element&.addresses&.to_h { ... } || {}` — already
/// fixed in a prior commit (stale corpus data).
///
/// FN (natalie): `rescue => self&.captured_error` — Prism parses this as a
/// `CallTargetNode` (not `CallNode`). Added `CALL_TARGET_NODE` to interested node
/// types and handling in `check_node` for `CallTargetNode` with `&.` and non-nil
/// receiver (self, constant, literal).
///
/// ## Corpus investigation (2026-03-28) — nested `if` expressions inside `||` / parens
///
/// Remaining default-config FNs were AllowedMethods calls such as
/// `scope&.respond_to?(:context)` buried inside ternary or modifier-`if`
/// expressions that themselves appear under `||` or parentheses:
///
/// - `(receiver&.respond_to?(:foo) ? receiver.foo : nil) || fallback`
/// - `memo || (value if receiver&.respond_to?(:value))`
///
/// `visit_or_node` intentionally short-circuits the default Prism traversal, but
/// `visit_conditional_subtree` only descended through calls, `&&`/`||`, parens,
/// and `StatementsNode`. When the subtree hit an `IfNode`, recursion stopped and
/// the predicate was never inspected. Fixed by teaching the subtree walk to
/// descend through nested `IfNode`/`UnlessNode`/`ElseNode`/`WhileNode`/`UntilNode`
/// structures: predicates stay in direct conditional context, while bodies recurse
/// in non-direct context so nested boolean operators still work without turning
/// ordinary method bodies into false positives.
///
/// `present?` remains a default-config no-offense because RuboCop core does not
/// include it in `AllowedMethods`. The corpus oracle baseline loads
/// `rubocop-rails`, which does add `present?`, so the remaining brick FNs came
/// from traversal gaps rather than config lookup.
///
/// ## Corpus investigation (2026-03-29) — AllowedMethods inside predicate blocks
///
/// The lorint/brick FNs were real under the corpus baseline config, but the bug
/// was not `present?` itself. The offending `if snags&.present?` sits inside a
/// `do ... end` block that is nested under a larger `while (...)` predicate.
/// `visit_conditional_subtree` recursed through call receivers and arguments,
/// but not the block attached to a call, so nested conditionals inside
/// predicate-time blocks were skipped entirely. Fixed by descending into
/// `call.block()` / `BlockNode` bodies in non-direct context so inner
/// conditionals still apply RuboCop's immediate-parent rules.
///
/// ## Corpus fix (2026-03-31) — numbered parameter blocks in conversion_with_default
///
/// FP=1 (discourse): `mail[:cc]&.element&.addresses&.to_h { [_1.address, _1.name] } || {}`
/// was incorrectly flagged. RuboCop's `conversion_with_default?` node matcher
/// uses `(or (block $(csend _ :to_h) ...) (hash))` which only matches `block`
/// nodes, not `numblock` nodes (Ruby 2.7+ numbered parameters `_1`, `_2`, etc.).
/// In Prism, both regular and numbered-parameter blocks are `BlockNode`, differing
/// only in the `parameters()` field (`BlockParametersNode` vs
/// `NumberedParametersNode`). Fixed by skipping the conversion-with-default check
/// when the block has `NumberedParametersNode` parameters.
pub struct RedundantSafeNavigation;

/// Methods guaranteed to exist on every instance (their receivers can't be nil)
const GUARANTEED_INSTANCE_METHODS: &[&[u8]] = &[b"to_s", b"to_i", b"to_f", b"to_a", b"to_h"];

/// Methods that are allowed in conditions (default AllowedMethods)
const DEFAULT_ALLOWED_METHODS: &[&[u8]] = &[
    b"instance_of?",
    b"kind_of?",
    b"is_a?",
    b"eql?",
    b"respond_to?",
    b"equal?",
];

impl Cop for RedundantSafeNavigation {
    fn name(&self) -> &'static str {
        "Lint/RedundantSafeNavigation"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            AND_NODE,
            ARRAY_NODE,
            BLOCK_NODE,
            CALL_NODE,
            CALL_TARGET_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            FALSE_NODE,
            FLOAT_NODE,
            HASH_NODE,
            INTEGER_NODE,
            KEYWORD_HASH_NODE,
            OR_NODE,
            REGULAR_EXPRESSION_NODE,
            SELF_NODE,
            STRING_NODE,
            SYMBOL_NODE,
            TRUE_NODE,
            X_STRING_NODE,
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
        // Case 6: conversion with default literal (foo&.to_h || {})
        if let Some(or_node) = node.as_or_node() {
            self.check_conversion_with_default(source, &or_node, diagnostics);
            return;
        }

        // Case 7: CallTargetNode (rescue => self&.foo) — self is never nil
        if let Some(ct) = node.as_call_target_node() {
            if ct.is_safe_navigation() {
                let receiver = ct.receiver();
                if receiver.as_self_node().is_some()
                    || is_camel_case_const(&receiver)
                    || is_non_nil_literal(&receiver)
                {
                    let op_loc = ct.call_operator_loc();
                    let (line, column) = source.offset_to_line_col(op_loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Redundant safe navigation detected, use `.` instead.".to_string(),
                    ));
                }
            }
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must use safe navigation (&.)
        let op_loc = match call.call_operator_loc() {
            Some(loc) if loc.as_slice() == b"&." => loc,
            _ => return,
        };

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Case 1: Receiver is a constant in camel case (not all uppercase/snake case)
        if is_camel_case_const(&receiver) {
            let (line, column) = source.offset_to_line_col(op_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Redundant safe navigation detected, use `.` instead.".to_string(),
            ));
        }

        // Case 2: Receiver is a literal (not nil)
        if is_non_nil_literal(&receiver) {
            let (line, column) = source.offset_to_line_col(op_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Redundant safe navigation detected, use `.` instead.".to_string(),
            ));
        }

        // Case 3: Receiver is `self`
        if receiver.as_self_node().is_some() {
            let (line, column) = source.offset_to_line_col(op_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Redundant safe navigation detected, use `.` instead.".to_string(),
            ));
        }

        // Case 4: Receiver is a guaranteed instance method call (to_s, to_i, etc.)
        // foo.to_s&.strip is redundant because to_s always returns a string
        if is_guaranteed_instance_receiver(&receiver) {
            let (line, column) = source.offset_to_line_col(op_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Redundant safe navigation detected, use `.` instead.".to_string(),
            ));
        }

        // Case 5: AllowedMethods used in conditions — handled by check_source below
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
        let allowed_methods: Vec<Vec<u8>> =
            if let Some(ref allowed) = config.get_string_array("AllowedMethods") {
                allowed.iter().map(|m| m.as_bytes().to_vec()).collect()
            } else {
                DEFAULT_ALLOWED_METHODS.iter().map(|m| m.to_vec()).collect()
            };

        let mut visitor = ConditionalAllowedMethodVisitor {
            cop: self,
            source,
            allowed_methods,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

impl RedundantSafeNavigation {
    /// Check for `foo&.to_h || {}`, `foo&.to_a || []`, `foo&.to_i || 0`,
    /// `foo&.to_f || 0.0`, `foo&.to_s || ''`, and block form `foo&.to_h { ... } || {}`.
    fn check_conversion_with_default(
        &self,
        source: &SourceFile,
        or_node: &ruby_prism::OrNode<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let lhs = or_node.left();
        let rhs = or_node.right();

        // LHS must be a CallNode (foo&.to_h or foo&.to_h { ... })
        // In Prism, the block is a child of CallNode, so both forms are CallNode
        let csend = match lhs.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must use safe navigation (&.)
        let op_loc = match csend.call_operator_loc() {
            Some(loc) if loc.as_slice() == b"&." => loc,
            _ => return,
        };

        let method_name = csend.name().as_slice();

        // Conversion methods with arguments (e.g., to_i(16)) are NOT redundant.
        // nil.to_i returns 0, but nil.to_i(16) raises ArgumentError.
        if csend.arguments().is_some() {
            return;
        }

        // Blocks with numbered parameters (_1, _2, etc.) are not matched by
        // RuboCop's `conversion_with_default?` pattern, which only matches
        // `(block ...)` but not `(numblock ...)` in the parser gem AST.
        if csend
            .block()
            .and_then(|b| b.as_block_node())
            .and_then(|b| b.parameters())
            .is_some_and(|p| p.as_numbered_parameters_node().is_some())
        {
            return;
        }

        // Check method is a conversion method and RHS is its matching default
        let is_match = match method_name {
            b"to_h" => is_empty_hash(&rhs),
            b"to_a" => is_empty_array(&rhs),
            b"to_i" => is_integer_zero(&rhs),
            b"to_f" => is_float_zero(&rhs),
            b"to_s" => is_empty_string(&rhs),
            _ => false,
        };

        if is_match {
            // Offense at the &. operator position
            let (line, column) = source.offset_to_line_col(op_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Redundant safe navigation with default literal detected.".to_string(),
            ));
        }
    }
}

/// Nil-specific methods — `respond_to?(:to_a)` etc. should NOT be flagged
/// because nil genuinely responds to these methods.
const NIL_METHODS: &[&[u8]] = &[b"to_a", b"to_i", b"to_s", b"to_f", b"to_h"];

struct ConditionalAllowedMethodVisitor<'a> {
    cop: &'a RedundantSafeNavigation,
    source: &'a SourceFile,
    allowed_methods: Vec<Vec<u8>>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> ConditionalAllowedMethodVisitor<'a> {
    /// Check if a CallNode is an AllowedMethod with &. in conditional context
    fn check_call_in_conditional(&mut self, call: &ruby_prism::CallNode<'_>) {
        // Must use safe navigation (&.)
        let op_loc = match call.call_operator_loc() {
            Some(loc) if loc.as_slice() == b"&." => loc,
            _ => return,
        };

        // Must have a receiver
        if call.receiver().is_none() {
            return;
        }

        let method_name = call.name().as_slice();

        // Must be an AllowedMethod
        if !self
            .allowed_methods
            .iter()
            .any(|m| m.as_slice() == method_name)
        {
            return;
        }

        // Special case: respond_to? with a nil-specific method argument is NOT redundant
        // because nil does respond to :to_a, :to_i, etc.
        if method_name == b"respond_to?" {
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if let Some(first_arg) = arg_list.first() {
                    if let Some(sym) = first_arg.as_symbol_node() {
                        let sym_name: &[u8] = sym.unescaped();
                        if NIL_METHODS.contains(&sym_name) {
                            return;
                        }
                    }
                }
            }
        }

        let (line, column) = self.source.offset_to_line_col(op_loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Redundant safe navigation detected, use `.` instead.".to_string(),
        ));
    }

    /// Visit all CallNodes within a node tree (recursive), checking for offenses.
    ///
    /// `direct` tracks whether the current node is a direct operand of a conditional,
    /// boolean operator (`&&`/`||`), or negation (`!`). RuboCop only flags AllowedMethods
    /// when their immediate parent is one of these; parentheses break the chain.
    /// For example, `if (foo&.is_a?(X))` is NOT flagged (parens wrap the csend),
    /// but `if foo&.is_a?(X)` IS flagged. Nested modifier-`if` / ternary expressions
    /// are handled explicitly because Prism represents them as `IfNode`s inside the
    /// surrounding `||` / parentheses tree.
    fn visit_conditional_subtree(&mut self, node: &ruby_prism::Node<'_>, direct: bool) {
        if let Some(call) = node.as_call_node() {
            if direct {
                self.check_call_in_conditional(&call);
            }
            // Recurse into receiver for negation patterns (e.g., !foo&.is_a?(X))
            if let Some(recv) = call.receiver() {
                // If this call is a negation (!), the receiver is in direct context
                let is_negation = call.name().as_slice() == b"!";
                self.visit_conditional_subtree(&recv, is_negation);
            }
            if let Some(args) = call.arguments() {
                for arg in args.arguments().iter() {
                    self.visit_conditional_subtree(&arg, false);
                }
            }
            if let Some(block) = call.block().and_then(|b| b.as_block_node()) {
                if let Some(body) = block.body() {
                    self.visit_conditional_subtree(&body, false);
                }
            }
            return;
        }

        // Recurse through boolean operators (&&, ||, and, or)
        if let Some(and_node) = node.as_and_node() {
            self.visit_conditional_subtree(&and_node.left(), true);
            self.visit_conditional_subtree(&and_node.right(), true);
            return;
        }
        if let Some(or_node) = node.as_or_node() {
            self.visit_conditional_subtree(&or_node.left(), true);
            self.visit_conditional_subtree(&or_node.right(), true);
            return;
        }

        // Recurse through parentheses — parens break the direct context
        if let Some(parens) = node.as_parentheses_node() {
            if let Some(body) = parens.body() {
                self.visit_conditional_subtree(&body, false);
            }
            return;
        }

        if let Some(block) = node.as_block_node() {
            if let Some(body) = block.body() {
                self.visit_conditional_subtree(&body, false);
            }
            return;
        }

        if let Some(write) = node.as_local_variable_write_node() {
            self.visit_conditional_subtree(&write.value(), false);
            return;
        }

        if let Some(if_node) = node.as_if_node() {
            self.visit_conditional_subtree(&if_node.predicate(), true);
            if let Some(stmts) = if_node.statements() {
                self.visit_conditional_subtree(&stmts.as_node(), false);
            }
            if let Some(sub) = if_node.subsequent() {
                self.visit_conditional_subtree(&sub, false);
            }
            return;
        }

        if let Some(unless_node) = node.as_unless_node() {
            self.visit_conditional_subtree(&unless_node.predicate(), true);
            if let Some(stmts) = unless_node.statements() {
                self.visit_conditional_subtree(&stmts.as_node(), false);
            }
            if let Some(else_clause) = unless_node.else_clause() {
                self.visit_conditional_subtree(&else_clause.as_node(), false);
            }
            return;
        }

        if let Some(else_node) = node.as_else_node() {
            if let Some(stmts) = else_node.statements() {
                self.visit_conditional_subtree(&stmts.as_node(), false);
            }
            return;
        }

        if let Some(while_node) = node.as_while_node() {
            self.visit_conditional_subtree(&while_node.predicate(), true);
            if let Some(stmts) = while_node.statements() {
                self.visit_conditional_subtree(&stmts.as_node(), false);
            }
            return;
        }

        if let Some(until_node) = node.as_until_node() {
            self.visit_conditional_subtree(&until_node.predicate(), true);
            if let Some(stmts) = until_node.statements() {
                self.visit_conditional_subtree(&stmts.as_node(), false);
            }
            return;
        }

        // Recurse through statements (body of parentheses)
        if let Some(stmts) = node.as_statements_node() {
            for stmt in stmts.body().iter() {
                self.visit_conditional_subtree(&stmt, false);
            }
        }
    }
}

impl<'a> Visit<'a> for ConditionalAllowedMethodVisitor<'a> {
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'a>) {
        // Visit the predicate in conditional context
        let predicate = node.predicate();
        self.visit_conditional_subtree(&predicate, true);

        // Visit body normally (not in conditional context)
        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }
        // Visit subsequent (elsif or else) — recurse via default visit
        if let Some(sub) = node.subsequent() {
            if let Some(if_node) = sub.as_if_node() {
                self.visit_if_node(&if_node);
            } else if let Some(else_node) = sub.as_else_node() {
                self.visit_else_node(&else_node);
            }
        }
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'a>) {
        let predicate = node.predicate();
        self.visit_conditional_subtree(&predicate, true);

        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }
        if let Some(else_clause) = node.else_clause() {
            self.visit_else_node(&else_clause);
        }
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'a>) {
        let predicate = node.predicate();
        self.visit_conditional_subtree(&predicate, true);

        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'a>) {
        let predicate = node.predicate();
        self.visit_conditional_subtree(&predicate, true);

        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }
    }

    fn visit_and_node(&mut self, node: &ruby_prism::AndNode<'a>) {
        // Any &.allowed_method inside && is in a boolean/conditional context
        self.visit_conditional_subtree(&node.left(), true);
        self.visit_conditional_subtree(&node.right(), true);
        // Don't call default visit — we already recursed into both operands
    }

    fn visit_or_node(&mut self, node: &ruby_prism::OrNode<'a>) {
        // Any &.allowed_method inside || is in a boolean/conditional context
        self.visit_conditional_subtree(&node.left(), true);
        self.visit_conditional_subtree(&node.right(), true);
        // Don't call default visit — we already recursed into both operands
    }
}

fn is_empty_hash(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(hash) = node.as_hash_node() {
        hash.elements().is_empty()
    } else {
        false
    }
}

fn is_empty_array(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(arr) = node.as_array_node() {
        arr.elements().is_empty()
    } else {
        false
    }
}

fn is_integer_zero(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(int) = node.as_integer_node() {
        let src = int.location().as_slice();
        src == b"0"
    } else {
        false
    }
}

fn is_float_zero(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(float) = node.as_float_node() {
        float.value() == 0.0
    } else {
        false
    }
}

fn is_empty_string(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(s) = node.as_string_node() {
        s.unescaped().is_empty()
    } else {
        false
    }
}

fn is_camel_case_const(node: &ruby_prism::Node<'_>) -> bool {
    let const_name = if let Some(c) = node.as_constant_read_node() {
        c.name().as_slice().to_vec()
    } else if let Some(cp) = node.as_constant_path_node() {
        // Use the last part of the path
        if let Some(name) = cp.name() {
            name.as_slice().to_vec()
        } else {
            return false;
        }
    } else {
        return false;
    };

    // All-uppercase or all-uppercase+underscore = snake case constant, not camel case
    // Check if it's NOT all uppercase/underscore/digits
    !const_name
        .iter()
        .all(|&b| b.is_ascii_uppercase() || b == b'_' || b.is_ascii_digit())
}

fn is_non_nil_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_string_node().is_some()
        || node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_array_node().is_some()
        || node.as_hash_node().is_some()
        || node.as_keyword_hash_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_regular_expression_node().is_some()
        || node.as_x_string_node().is_some()
}

fn is_guaranteed_instance_receiver(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        // Don't check if the call itself uses safe navigation
        if let Some(op) = call.call_operator_loc() {
            if op.as_slice() == b"&." {
                return false;
            }
        }
        let method = call.name().as_slice();
        GUARANTEED_INSTANCE_METHODS.contains(&method)
    } else if let Some(block) = node.as_block_node() {
        // Block wrapping: foo.to_h { ... }&.keys
        let src = block.location().as_slice();
        // Check if this block's source contains a guaranteed method with regular dot
        for method in GUARANTEED_INSTANCE_METHODS {
            let dot_method = [b"." as &[u8], *method].concat();
            if src
                .windows(dot_method.len())
                .any(|w| w == dot_method.as_slice())
            {
                // Make sure it doesn't use &.
                let safe_method = [b"&." as &[u8], *method].concat();
                if !src
                    .windows(safe_method.len())
                    .any(|w| w == safe_method.as_slice())
                {
                    return true;
                }
            }
        }
        false
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    crate::cop_fixture_tests!(
        RedundantSafeNavigation,
        "cops/lint/redundant_safe_navigation"
    );

    #[test]
    fn flags_configured_allowed_method_inside_predicate_block() {
        let mut options = HashMap::new();
        options.insert(
            "AllowedMethods".to_string(),
            serde_yml::Value::Sequence(vec![serde_yml::Value::String("present?".to_string())]),
        );
        let config = CopConfig {
            options,
            ..CopConfig::default()
        };

        let diagnostics = crate::testutil::run_cop_full_with_config(
            &RedundantSafeNavigation,
            b"while (items = values.reject do |value|\nif value&.present?\n  selected << value\nend\nend).any?\n  process\nend\n",
            config,
        );

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].location.line, 2);
        assert_eq!(diagnostics[0].location.column, 8);
        assert_eq!(diagnostics[0].cop_name, "Lint/RedundantSafeNavigation");
        assert_eq!(
            diagnostics[0].message,
            "Redundant safe navigation detected, use `.` instead."
        );
    }
}
