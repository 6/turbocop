use std::cell::RefCell;
use std::ops::Range;

use crate::cop::shared::method_identifier_predicates;
use crate::cop::variable_force::{self, Scope, VariableTable};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

// Thread-local storage for per-file data. Within a rayon task, a single
// file is processed sequentially: check_source → VF engine →
// after_leaving_scope, so thread-local storage is safe and avoids the
// TOCTOU race that Mutex fields on the shared cop struct would cause.
thread_local! {
    static SAVE_BANG_DATA: RefCell<SaveBangData> = const { RefCell::new(SaveBangData::new()) };
}

struct SaveBangData {
    persisted_receiver_offsets: Vec<usize>,
    persisted_if_body_ranges: Vec<Range<usize>>,
    create_assignments: Vec<CreateAssignmentInfo>,
}

impl SaveBangData {
    const fn new() -> Self {
        Self {
            persisted_receiver_offsets: Vec::new(),
            persisted_if_body_ranges: Vec::new(),
            create_assignments: Vec::new(),
        }
    }
}

/// Rails/SaveBang - flags ActiveRecord persist methods (save, update, destroy, create, etc.)
/// whose return value is not checked, suggesting bang variants instead.
///
/// ## Architecture
///
/// Uses a hybrid check_source + VariableForce approach:
///
/// **check_source** (on_send path): A context-tracking AST visitor handles ALL persist calls
/// except create-type methods assigned to local variables. It maintains a context stack to
/// determine if the return value is used (assigned, in condition, as argument, etc.).
///
/// **VariableForce after_leaving_scope** (create-in-assignment path): For create-type methods
/// assigned to local variables, VF tracks whether `persisted?` is ever called on the variable.
/// This replaces the manual persisted?-scanning logic from the standalone implementation.
///
/// The check_source phase pre-computes byte offsets where `var.persisted?` calls appear
/// (storing the local variable read offset). The VF hook checks if any reference to a
/// create-assigned variable matches a persisted? call offset.
///
/// ## RuboCop compatibility
///
/// This cop matches RuboCop's `Rails/SaveBang` behavior, which uses VariableForce for the
/// create-in-local-assignment path and `on_send` for everything else. See the RuboCop source
/// at `vendor/rubocop-rails/lib/rubocop/cop/rails/save_bang.rb`.
///
/// ## Corpus results
///
/// FP=0, FN=0 (99.98%+ match rate). See previous doc comments in git history for
/// detailed investigation notes on each edge case.
pub struct SaveBang;

/// Info about a create-type persist call in a local variable assignment.
struct CreateAssignmentInfo {
    /// Byte offset of the LocalVariableWriteNode (matches VF assignment.node_offset).
    assignment_offset: usize,
    /// Start byte offset of the method name selector (for diagnostic location).
    message_offset: usize,
    /// The method name string (e.g., "create", "find_or_create_by").
    method_name: &'static str,
}

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
fn is_setter_method(name: &[u8]) -> bool {
    method_identifier_predicates::is_setter_method(name)
}

/// Check if a Prism node is a literal type (matches RuboCop's `Node#literal?`).
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

impl SaveBang {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SaveBang {
    fn default() -> Self {
        Self
    }
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

        // Pre-compute data for the VF hook.
        let mut collector = PreComputeCollector {
            receiver_offsets: Vec::new(),
            if_body_ranges: Vec::new(),
            create_assignments: Vec::new(),
        };
        collector.visit(&parse_result.node());
        SAVE_BANG_DATA.with(|cell| {
            let mut data = cell.borrow_mut();
            data.persisted_receiver_offsets = collector.receiver_offsets;
            data.persisted_if_body_ranges = collector.if_body_ranges;
            data.create_assignments = collector.create_assignments;
        });

        // Run the context-tracking visitor for on_send path.
        let mut visitor = SaveBangVisitor {
            cop: self,
            source,
            allow_implicit_return,
            allowed_receivers,
            diagnostics: Vec::new(),
            context_stack: Vec::new(),
            in_compound_boolean: false,
            in_transparent_container: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }

    fn as_variable_force_consumer(&self) -> Option<&dyn variable_force::VariableForceConsumer> {
        Some(self)
    }
}

impl variable_force::VariableForceConsumer for SaveBang {
    fn after_leaving_scope(
        &self,
        scope: &Scope,
        _variable_table: &VariableTable,
        source: &SourceFile,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        SAVE_BANG_DATA.with(|cell| {
            let data = cell.borrow();
            let create_assignments = &data.create_assignments;
            let persisted_offsets = &data.persisted_receiver_offsets;
            let persisted_if_ranges = &data.persisted_if_body_ranges;

            for variable in scope.variables.values() {
                // Check if any of the variable's references land on a persisted?-equivalent offset.
                // RuboCop's persisted_referenced? checks assignment.variable.references (ALL refs).
                let has_persisted_ref = variable.references.iter().any(|r| {
                    persisted_offsets.contains(&r.node_offset)
                        || persisted_if_ranges
                            .iter()
                            .any(|range| range.contains(&r.node_offset))
                });

                for assignment in &variable.assignments {
                    // Only check simple assignments (x = expr), not ||=, &&=, operator writes.
                    // RuboCop's right_assignment_node returns early for or_asgn/and_asgn.
                    if !matches!(assignment.kind, variable_force::AssignmentKind::Simple) {
                        continue;
                    }

                    // Look up this assignment in the pre-computed create assignments.
                    let create_info = create_assignments
                        .iter()
                        .find(|ca| ca.assignment_offset == assignment.node_offset);

                    let create_info = match create_info {
                        Some(info) => info,
                        None => continue,
                    };

                    // RuboCop: return if persisted_referenced?(assignment)
                    // persisted_referenced? requires assignment.referenced? AND variable has persisted? ref.
                    if !assignment.references.is_empty() && has_persisted_ref {
                        continue;
                    }

                    // Emit CREATE_MSG offense.
                    let (line, column) = source.offset_to_line_col(create_info.message_offset);
                    let message = CREATE_MSG
                        .replace("%prefer%", &format!("{}!", create_info.method_name))
                        .replace("%current%", create_info.method_name);
                    diagnostics.push(self.diagnostic(source, line, column, message));
                }
            }
        });
    }
}

// ── Persisted? call collector ─────────────────────────────────────────────

/// Pre-computes data for the VF hook:
/// 1. Byte offsets of local variable reads that are receivers of `.persisted?` calls
/// 2. Byte ranges of if-bodies where the condition is `.persisted?`
/// 3. Create-type persist calls in local variable assignments
struct PreComputeCollector {
    receiver_offsets: Vec<usize>,
    if_body_ranges: Vec<Range<usize>>,
    create_assignments: Vec<CreateAssignmentInfo>,
}

impl PreComputeCollector {
    fn is_persisted_send(node: &ruby_prism::CallNode<'_>) -> bool {
        let is_csend = node
            .call_operator_loc()
            .is_some_and(|loc| loc.end_offset() - loc.start_offset() == 2);
        !is_csend && node.name().as_slice() == b"persisted?"
    }

    fn create_method_name(name: &[u8]) -> Option<&'static str> {
        match name {
            b"create" => Some("create"),
            b"create_or_find_by" => Some("create_or_find_by"),
            b"first_or_create" => Some("first_or_create"),
            b"find_or_create_by" => Some("find_or_create_by"),
            _ => None,
        }
    }

    /// Check if a CallNode is a create-type persist call with valid signature.
    /// Mirrors the checks in `SaveBangVisitor::classify_persist_call` for the
    /// create-in-assignment VF path. Returns the method name if it's a valid
    /// create persist call, None otherwise.
    fn classify_create_persist_call(
        &self,
        call: &ruby_prism::CallNode<'_>,
    ) -> Option<&'static str> {
        let name = call.name().as_slice();
        let method_name = Self::create_method_name(name)?;

        // Bare calls (no receiver) are not persist methods — e.g. FactoryBot's create().
        // RuboCop's allowed_receiver? returns false for nil receivers, which means
        // persist_method? still passes, but on_send's return_value_assigned? checks
        // the parent assignment which filters these out. However, our VF path
        // pre-collects assignments, so we must filter here.
        call.receiver()?;

        // Check expected_signature: no arguments, or one hash/non-literal argument.
        let has_block_arg = call
            .block()
            .is_some_and(|b| b.as_block_argument_node().is_some());

        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            let total_args = arg_list.len() + usize::from(has_block_arg);

            if total_args >= 2 {
                return None;
            }

            if arg_list.len() == 1 {
                let arg = &arg_list[0];
                if arg.as_hash_node().is_some() || arg.as_keyword_hash_node().is_some() {
                    // Valid persistence signature
                } else if is_literal_node(arg) {
                    return None;
                }
            }
        }

        Some(method_name)
    }
}

impl<'pr> Visit<'pr> for PreComputeCollector {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if Self::is_persisted_send(node) {
            if let Some(recv) = node.receiver() {
                if recv.as_local_variable_read_node().is_some() {
                    self.receiver_offsets.push(recv.location().start_offset());
                }
            }
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let pred = node.predicate();
        let condition_is_persisted = pred
            .as_call_node()
            .is_some_and(|c| Self::is_persisted_send(&c));

        if condition_is_persisted {
            if let Some(stmts) = node.statements() {
                let loc = stmts.location();
                self.if_body_ranges
                    .push(loc.start_offset()..loc.end_offset());
            }
        }
        ruby_prism::visit_if_node(self, node);
    }

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        // Check if the RHS is a create-type persist call.
        // RuboCop's right_assignment_node unwraps block nodes to the send node,
        // but in Prism `Model.create { ... }` is a single CallNode, so we just
        // check the value directly.
        let value = node.value();
        if let Some(call) = value.as_call_node() {
            if let Some(method_name) = self.classify_create_persist_call(&call) {
                if let Some(msg_loc) = call.message_loc() {
                    self.create_assignments.push(CreateAssignmentInfo {
                        assignment_offset: node.location().start_offset(),
                        message_offset: msg_loc.start_offset(),
                        method_name,
                    });
                }
            }
        }
        ruby_prism::visit_local_variable_write_node(self, node);
    }
}

// ── Context-tracking visitor (on_send path) ───────────────────────────────

struct SaveBangVisitor<'a, 'src> {
    cop: &'a SaveBang,
    source: &'src SourceFile,
    allow_implicit_return: bool,
    allowed_receivers: Vec<String>,
    diagnostics: Vec<Diagnostic>,
    context_stack: Vec<Context>,
    /// When true, we are inside an `||` or `&&` (compound boolean) expression.
    in_compound_boolean: bool,
    /// When true, the current context was inherited through a transparent container
    /// (hash/array/keyword_hash).
    in_transparent_container: bool,
}

impl SaveBangVisitor<'_, '_> {
    fn current_context(&self) -> Option<Context> {
        self.context_stack.last().copied()
    }

    /// Check if a CallNode is a persistence method. Returns Some(is_create) or None.
    fn classify_persist_call(&self, call: &ruby_prism::CallNode<'_>) -> Option<bool> {
        let method_name = call.name().as_slice();

        let is_modify = MODIFY_PERSIST_METHODS.contains(&method_name);
        let is_create = CREATE_PERSIST_METHODS.contains(&method_name);

        if !is_modify && !is_create {
            return None;
        }

        // Check expected_signature: no arguments, or one hash/non-literal argument.
        let has_block_arg = call
            .block()
            .is_some_and(|b| b.as_block_argument_node().is_some());

        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            let total_args = arg_list.len() + usize::from(has_block_arg);

            if method_name == b"destroy" {
                return None;
            }

            if total_args >= 2 {
                return None;
            }

            if arg_list.len() == 1 {
                let arg = &arg_list[0];
                if arg.as_hash_node().is_some() || arg.as_keyword_hash_node().is_some() {
                    // Valid persistence signature
                } else if is_literal_node(arg) {
                    return None;
                }
            }
        } else if has_block_arg {
            // Only a &block argument — still valid
        }

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

        for allowed in &self.allowed_receivers {
            if self.receiver_chain_matches(call, allowed) {
                return true;
            }
            if recv_str == allowed.as_str() {
                return true;
            }
        }

        false
    }

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

    fn const_matches(&self, const_name: &str, allowed: &str) -> bool {
        if allowed.starts_with("::") {
            const_name == allowed
                || format!("::{const_name}") == allowed
                || const_name == &allowed[2..]
        } else {
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

    /// Process a call node that has been identified as a persist method.
    fn process_persist_call(&mut self, call: &ruby_prism::CallNode<'_>, is_create: bool) {
        // Compound boolean handling for create methods.
        if is_create && self.in_compound_boolean {
            let current = self.current_context();
            if matches!(current, Some(Context::Assignment)) {
                // Direct assignment inside boolean: let the assignment path handle it
            } else {
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

        // Block-bearing persist calls inside transparent containers lose exemption.
        let has_block_body = call.block().is_some_and(|b| b.as_block_node().is_some());
        let effective_context = if has_block_body && self.in_transparent_container {
            Some(Context::VoidStatement)
        } else {
            self.current_context()
        };

        match effective_context {
            Some(Context::VoidStatement) => {
                self.flag_void_context(call);
            }
            Some(Context::Assignment) => {
                // Modify methods: return value is captured, exempt.
                // Create in local assignment: VF hook handles it.
                // Create in non-local assignment (ivar/cvar/gvar): RuboCop's
                // return_value_assigned? exempts them in on_send.
            }
            Some(Context::Condition) => {
                if is_create {
                    self.flag_create_conditional(call);
                }
            }
            Some(Context::ImplicitReturn)
            | Some(Context::Argument)
            | Some(Context::ExplicitReturn) => {
                // Return value is used — no offense.
            }
            None => {
                self.flag_void_context(call);
            }
        }
    }

    /// Visit children of a StatementsNode with proper context tracking.
    fn visit_statements_with_context(
        &mut self,
        node: &ruby_prism::StatementsNode<'_>,
        in_method_or_block: bool,
    ) {
        let body: Vec<_> = node.body().iter().collect();
        let len = body.len();

        for (i, stmt) in body.iter().enumerate() {
            let is_last = i == len - 1;
            let ctx = if is_last && in_method_or_block && self.allow_implicit_return && len == 1 {
                Context::ImplicitReturn
            } else {
                Context::VoidStatement
            };

            self.context_stack.push(ctx);
            self.visit(stmt);
            self.context_stack.pop();
        }
    }
}

impl<'pr> Visit<'pr> for SaveBangVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if let Some(is_create) = self.classify_persist_call(node) {
            self.process_persist_call(node, is_create);
        }

        // Visit receiver with proper context.
        if let Some(recv) = node.receiver() {
            let method_name = node.name().as_slice();
            let is_csend_call = node
                .call_operator_loc()
                .is_some_and(|loc| loc.end_offset() - loc.start_offset() == 2);
            let is_persisted_check = method_name == b"persisted?" && !is_csend_call;
            let is_negation = method_name == b"!" && node.arguments().is_none();
            let is_setter = is_setter_method(method_name);

            if is_persisted_check {
                self.context_stack.push(Context::Argument);
                self.visit(&recv);
                self.context_stack.pop();
            } else if is_negation {
                self.context_stack.push(Context::Condition);
                self.visit(&recv);
                self.context_stack.pop();
            } else if is_setter {
                self.context_stack.push(Context::Assignment);
                self.visit(&recv);
                self.context_stack.pop();
            } else {
                self.context_stack.push(Context::VoidStatement);
                self.visit(&recv);
                self.context_stack.pop();
            }
        }

        if let Some(args) = node.arguments() {
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

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        if let Some(params) = node.parameters() {
            self.visit(&params);
        }
        let saved_transparent = self.in_transparent_container;
        self.in_transparent_container = false;
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                self.visit_statements_with_context(&stmts, true);
            } else {
                self.visit(&body);
            }
        }
        self.in_transparent_container = saved_transparent;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        if let Some(params) = node.parameters() {
            self.visit(&params);
        }
        let saved_transparent = self.in_transparent_container;
        self.in_transparent_container = false;
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                self.visit_statements_with_context(&stmts, true);
            } else {
                self.visit(&body);
            }
        }
        self.in_transparent_container = saved_transparent;
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if let Some(params) = node.parameters() {
            self.visit_parameters_node(&params);
        }
        let is_instance_method = node.receiver().is_none();
        let saved_transparent = self.in_transparent_container;
        self.in_transparent_container = false;
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                self.visit_statements_with_context(&stmts, is_instance_method);
            } else {
                self.visit(&body);
            }
        }
        self.in_transparent_container = saved_transparent;
    }

    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        let body: Vec<_> = node.body().iter().collect();

        for stmt in body.iter() {
            self.context_stack.push(Context::VoidStatement);
            self.visit(stmt);
            self.context_stack.pop();
        }
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let predicate = node.predicate();
        self.context_stack.push(Context::Condition);
        self.visit(&predicate);
        self.context_stack.pop();

        let pred_src = &self.source.as_bytes()
            [predicate.location().start_offset()..predicate.location().end_offset()];

        if let Some(stmts) = node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            for stmt in body.iter() {
                let stmt_src = &self.source.as_bytes()
                    [stmt.location().start_offset()..stmt.location().end_offset()];
                let ctx = if stmt_src == pred_src {
                    Context::Condition
                } else {
                    Context::VoidStatement
                };

                self.context_stack.push(ctx);
                self.visit(stmt);
                self.context_stack.pop();
            }
        }

        if let Some(subsequent) = node.subsequent() {
            self.context_stack.push(Context::VoidStatement);
            self.visit(&subsequent);
            self.context_stack.pop();
        }
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
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

    // ── Assignment nodes ────────────────────────────────────────────────

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
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
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
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
        self.context_stack.push(Context::Assignment);
        self.visit(&node.value());
        self.context_stack.pop();
    }

    fn visit_call_operator_write_node(&mut self, node: &ruby_prism::CallOperatorWriteNode<'pr>) {
        if let Some(receiver) = node.receiver() {
            self.context_stack.push(Context::VoidStatement);
            self.visit(&receiver);
            self.context_stack.pop();
        }
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

    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
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

    // ── Return / Next ───────────────────────────────────────────────────

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

    fn visit_yield_node(&mut self, node: &ruby_prism::YieldNode<'pr>) {
        if let Some(args) = node.arguments() {
            self.context_stack.push(Context::VoidStatement);
            self.visit_arguments_node(&args);
            self.context_stack.pop();
        }
    }

    fn visit_super_node(&mut self, node: &ruby_prism::SuperNode<'pr>) {
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

    // ── And / Or ────────────────────────────────────────────────────────

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
        let saved = self.in_compound_boolean;
        self.in_compound_boolean = true;
        let ctx = self.current_context();
        match ctx {
            Some(Context::ImplicitReturn) => {
                self.context_stack.push(Context::Condition);
                self.visit(&node.left());
                self.context_stack.pop();
                self.visit(&node.right());
            }
            Some(Context::ExplicitReturn) => {
                self.context_stack.push(Context::Condition);
                self.visit(&node.left());
                self.visit(&node.right());
                self.context_stack.pop();
            }
            Some(Context::Argument) => {
                self.visit(&node.left());
                self.visit(&node.right());
            }
            _ => {
                self.context_stack.push(Context::Condition);
                self.visit(&node.left());
                self.visit(&node.right());
                self.context_stack.pop();
            }
        }
        self.in_compound_boolean = saved;
    }

    // ── Array / Hash / KeywordHash ──────────────────────────────────────

    fn visit_array_node(&mut self, node: &ruby_prism::ArrayNode<'pr>) {
        let saved_transparent = self.in_transparent_container;
        let saved_compound = self.in_compound_boolean;
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
        self.in_transparent_container = saved_transparent;
        self.in_compound_boolean = saved_compound;
    }

    fn visit_hash_node(&mut self, node: &ruby_prism::HashNode<'pr>) {
        let saved_transparent = self.in_transparent_container;
        let saved_compound = self.in_compound_boolean;
        self.in_transparent_container = true;
        self.in_compound_boolean = false;
        for element in node.elements().iter() {
            self.visit(&element);
        }
        self.in_transparent_container = saved_transparent;
        self.in_compound_boolean = saved_compound;
    }

    fn visit_keyword_hash_node(&mut self, node: &ruby_prism::KeywordHashNode<'pr>) {
        let saved_transparent = self.in_transparent_container;
        let saved_compound = self.in_compound_boolean;
        self.in_transparent_container = true;
        self.in_compound_boolean = false;
        for element in node.elements().iter() {
            self.visit(&element);
        }
        self.in_transparent_container = saved_transparent;
        self.in_compound_boolean = saved_compound;
    }

    // ── Begin / Parentheses ─────────────────────────────────────────────

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

    fn visit_parentheses_node(&mut self, node: &ruby_prism::ParenthesesNode<'pr>) {
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

    // ── Class / Module / Singleton ──────────────────────────────────────

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

    fn visit_program_node(&mut self, node: &ruby_prism::ProgramNode<'pr>) {
        self.visit_statements_with_context(&node.statements(), false);
    }

    // ── While / Until / For ─────────────────────────────────────────────

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

    // ── Rescue modifier ─────────────────────────────────────────────────

    fn visit_rescue_modifier_node(&mut self, node: &ruby_prism::RescueModifierNode<'pr>) {
        self.context_stack.push(Context::VoidStatement);
        self.visit(&node.expression());
        self.context_stack.pop();
        self.visit(&node.rescue_expression());
    }

    // ── Splat ───────────────────────────────────────────────────────────

    fn visit_splat_node(&mut self, node: &ruby_prism::SplatNode<'pr>) {
        if let Some(expr) = node.expression() {
            self.context_stack.push(Context::VoidStatement);
            self.visit(&expr);
            self.context_stack.pop();
        }
    }

    fn visit_assoc_splat_node(&mut self, node: &ruby_prism::AssocSplatNode<'pr>) {
        if let Some(expr) = node.value() {
            self.context_stack.push(Context::VoidStatement);
            self.visit(&expr);
            self.context_stack.pop();
        }
    }

    // ── Interpolation ───────────────────────────────────────────────────

    fn visit_embedded_statements_node(&mut self, node: &ruby_prism::EmbeddedStatementsNode<'pr>) {
        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SaveBang::new(), "cops/rails/save_bang");

    /// Regression: bare create (no receiver) like FactoryBot should not be flagged
    /// in VF create-in-assignment path (was causing 11k FP).
    #[test]
    fn bare_create_in_assignment_not_flagged() {
        let source = b"describe Project do
  it 'test' do
    project = create :project, github_url: 'http://example.com'
  end
end
";
        let diagnostics = crate::testutil::run_cop_full(&SaveBang::new(), source);
        assert_eq!(
            diagnostics.len(),
            0,
            "Expected 0 offenses for bare create (FactoryBot), got {}: {:?}",
            diagnostics.len(),
            diagnostics
                .iter()
                .map(|d| format!("{}:{}: {}", d.location.line, d.location.column, d.message))
                .collect::<Vec<_>>()
        );
    }

    /// Regression test: when the same variable is assigned with create twice at
    /// the top level, and persisted? is called after the first assignment but not
    /// after the second, the second assignment should still be flagged.
    #[test]
    fn reassigned_create_var_second_not_suppressed() {
        let source =
            b"field = A.create name: 'test'\nfield.persisted?\nfield = B.create name: 'info'\n";
        let diagnostics = crate::testutil::run_cop_full(&SaveBang::new(), source);
        assert_eq!(
            diagnostics.len(),
            1,
            "Expected 1 offense for the second create, got {}: {:?}",
            diagnostics.len(),
            diagnostics
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
        assert_eq!(diagnostics[0].location.line, 3);
    }
}
