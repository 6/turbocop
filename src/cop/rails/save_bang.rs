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
    /// Inside an array or hash literal (return value is "used").
    Collection,
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

        // Check expected_signature: no arguments, or one hash/non-literal argument
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();

            // destroy with any arguments is not a persistence method
            if method_name == b"destroy" {
                return None;
            }

            // More than one argument: not a persistence call (e.g., Model.save(1, name: 'Tom'))
            if arg_list.len() >= 2 {
                return None;
            }

            // Single argument: must be a hash or non-literal
            if arg_list.len() == 1 {
                let arg = &arg_list[0];
                // String literal is not a valid persistence call
                if arg.as_string_node().is_some() {
                    return None;
                }
                // Integer literal is not valid
                if arg.as_integer_node().is_some() {
                    return None;
                }
                // Symbol literal is not valid
                if arg.as_symbol_node().is_some() {
                    return None;
                }
            }
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

    /// Check if the RHS of an assignment contains a create-type persist call.
    fn rhs_has_create_call(&self, node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            if self.classify_persist_call(&call) == Some(true) {
                return true;
            }
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
        // Extract variable name from assignment
        let var_name = match Self::assignment_var_name(stmt) {
            Some(name) => name,
            None => return false,
        };

        // Check if the RHS contains a create-type call
        let rhs = self.get_assignment_rhs(stmt);
        let has_create = match rhs {
            Some(rhs_node) => self.rhs_has_create_call(&rhs_node),
            None => false,
        };
        if !has_create {
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

    /// Get the RHS value node from an assignment statement.
    fn get_assignment_rhs<'n>(
        &self,
        node: &'n ruby_prism::Node<'n>,
    ) -> Option<ruby_prism::Node<'n>> {
        if let Some(lv) = node.as_local_variable_write_node() {
            return Some(lv.value());
        }
        if let Some(iv) = node.as_instance_variable_write_node() {
            return Some(iv.value());
        }
        if let Some(gv) = node.as_global_variable_write_node() {
            return Some(gv.value());
        }
        if let Some(cv) = node.as_class_variable_write_node() {
            return Some(cv.value());
        }
        if let Some(lov) = node.as_local_variable_or_write_node() {
            return Some(lov.value());
        }
        if let Some(mw) = node.as_multi_write_node() {
            return Some(mw.value());
        }
        None
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

        match self.current_context() {
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
                    self.flag_create_assignment(call);
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
            Some(Context::Argument) | Some(Context::ExplicitReturn) | Some(Context::Collection) => {
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
    }
}

/// A simple visitor that searches a subtree for `var.persisted?` calls.
/// Used by `subtree_checks_persisted` to match RuboCop's VariableForce behavior
/// of finding persisted? references anywhere in a scope, not just the next statement.
struct PersistedFinder<'v> {
    var_name: &'v [u8],
    found: bool,
}

impl<'pr> Visit<'pr> for PersistedFinder<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if self.found {
            return;
        }
        if node.name().as_slice() == b"persisted?" {
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
            let is_persisted_check = method_name == b"persisted?";
            let is_negation = method_name == b"!" && node.arguments().is_none();

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
            self.context_stack.push(Context::Argument);
            self.visit_arguments_node(&args);
            self.context_stack.pop();
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
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                self.visit_statements_with_context(&stmts, true);
            } else {
                self.visit(&body);
            }
        }
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        if let Some(params) = node.parameters() {
            self.visit(&params);
        }
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                self.visit_statements_with_context(&stmts, true);
            } else {
                self.visit(&body);
            }
        }
    }

    // ── DefNode: body has implicit return semantics ──────────────────────

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if let Some(params) = node.parameters() {
            self.visit_parameters_node(&params);
        }
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                self.visit_statements_with_context(&stmts, true);
            } else {
                self.visit(&body);
            }
        }
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

        // The then-body and else-body inherit the parent context
        // (they are statement sequences where persist calls may appear)
        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }

        if let Some(subsequent) = node.subsequent() {
            self.visit(&subsequent);
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
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
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
        self.in_local_assignment = true;
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
        self.in_local_assignment = false;
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        self.in_local_assignment = true;
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
        self.in_local_assignment = false;
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
        if let Some(args) = node.arguments() {
            self.context_stack.push(Context::Argument);
            self.visit_arguments_node(&args);
            self.context_stack.pop();
        }
    }

    fn visit_super_node(&mut self, node: &ruby_prism::SuperNode<'pr>) {
        if let Some(args) = node.arguments() {
            self.context_stack.push(Context::Argument);
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
        self.context_stack.push(Context::Condition);
        self.visit(&node.left());
        self.visit(&node.right());
        self.context_stack.pop();
    }

    fn visit_or_node(&mut self, node: &ruby_prism::OrNode<'pr>) {
        // RuboCop's implicit_return? walks up through or_type? nodes.
        // So if an OrNode is in implicit return position, both children
        // inherit ImplicitReturn context (not Condition), matching RuboCop
        // behavior where `find(**opts) || create(**opts)` at end of method
        // is exempt.
        // Same for ExplicitReturn, Assignment, Argument, Collection contexts
        // where the return value of the || expression is being used.
        let ctx = self.current_context();
        match ctx {
            Some(Context::ImplicitReturn)
            | Some(Context::ExplicitReturn)
            | Some(Context::Assignment)
            | Some(Context::Argument)
            | Some(Context::Collection) => {
                // Inherit parent context — the || result is being used
                self.visit(&node.left());
                self.visit(&node.right());
            }
            _ => {
                // VoidStatement or None: the || is in condition/boolean context
                self.context_stack.push(Context::Condition);
                self.visit(&node.left());
                self.visit(&node.right());
                self.context_stack.pop();
            }
        }
    }

    // ── Array / Hash literals: children are collection context ───────────

    fn visit_array_node(&mut self, node: &ruby_prism::ArrayNode<'pr>) {
        self.context_stack.push(Context::Collection);
        for element in node.elements().iter() {
            self.visit(&element);
        }
        self.context_stack.pop();
    }

    fn visit_hash_node(&mut self, node: &ruby_prism::HashNode<'pr>) {
        self.context_stack.push(Context::Collection);
        for element in node.elements().iter() {
            self.visit(&element);
        }
        self.context_stack.pop();
    }

    fn visit_keyword_hash_node(&mut self, node: &ruby_prism::KeywordHashNode<'pr>) {
        self.context_stack.push(Context::Collection);
        for element in node.elements().iter() {
            self.visit(&element);
        }
        self.context_stack.pop();
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
        // Parentheses are transparent for context purposes
        if let Some(body) = node.body() {
            self.visit(&body);
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
