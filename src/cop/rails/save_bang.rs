use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

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
                if is_create {
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

            let ctx = if is_last && in_method_or_block && self.allow_implicit_return {
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
    // ── CallNode: check if this is a persist method ──────────────────────

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if let Some(is_create) = self.classify_persist_call(node) {
            self.process_persist_call(node, is_create);
        }

        // Continue visiting children (e.g., receiver, arguments, block)
        // But we need to set appropriate context for arguments
        if let Some(recv) = node.receiver() {
            // Receiver is evaluated but not "checked" in our sense;
            // it's not a context where persist calls matter
            self.context_stack.push(Context::Argument);
            self.visit(&recv);
            self.context_stack.pop();
        }

        if let Some(args) = node.arguments() {
            self.context_stack.push(Context::Argument);
            self.visit_arguments_node(&args);
            self.context_stack.pop();
        }

        if let Some(block) = node.block() {
            // A block argument like `&block` is just a reference, visit it
            if let Some(block_arg) = block.as_block_argument_node() {
                self.visit_block_argument_node(&block_arg);
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

        for stmt in &body {
            self.context_stack.push(Context::VoidStatement);
            self.visit(stmt);
            self.context_stack.pop();
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
        if let Some(stmts) = node.statements() {
            self.context_stack.push(Context::Argument);
            self.visit_statements_node(&stmts);
            self.context_stack.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SaveBang, "cops/rails/save_bang");
}
