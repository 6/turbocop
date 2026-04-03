use std::collections::HashSet;

use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// RuboCop parity notes:
/// - Local variables are tracked in source order (not pre-scanned). `self.x` before
///   `x = ...` is flagged as redundant, matching RuboCop's lazy variable tracking.
/// - `if`/`unless`/`while`/`until` nodes prescan ALL descendants (including inside
///   blocks) for local variable assignments. This makes `self.x` in the condition
///   allowed when `x` is assigned anywhere inside the conditional body, even in
///   nested blocks. This matches RuboCop's `on_if` behavior.
/// - Nested block and lambda locals leak forward into the enclosing scope for later
///   disambiguation, so `self.x` stays allowed after an earlier `do |x| ... end` or
///   `->(x) { ... }`, but not before that nested scope appears.
/// - Compound self-assignments (`self.count += 1`, `self.count ||= 1`, `self.count &&= 1`)
///   make later `self.count` reads acceptable in source order, even across later methods
///   and class/module nesting. Plain setters like `self.value = 1` do not.
/// - Parameter default values are visited for `self.` checks. `def foo(x = self.bar)`
///   flags `self.bar` unless `bar` is also a parameter name.
pub struct RedundantSelf;

/// Methods where self. is always required (Ruby keywords).
const ALLOWED_METHODS: &[&[u8]] = &[
    b"alias",
    b"and",
    b"begin",
    b"break",
    b"case",
    b"class",
    b"def",
    b"defined?",
    b"do",
    b"else",
    b"elsif",
    b"end",
    b"ensure",
    b"false",
    b"for",
    b"if",
    b"in",
    b"module",
    b"next",
    b"nil",
    b"not",
    b"or",
    b"redo",
    b"rescue",
    b"retry",
    b"return",
    b"self",
    b"super",
    b"then",
    b"true",
    b"undef",
    b"unless",
    b"until",
    b"when",
    b"while",
    b"yield",
    b"__FILE__",
    b"__LINE__",
    b"__ENCODING__",
    // raise is commonly treated as keyword-like
    b"raise",
];

/// Kernel methods where self. is required to avoid ambiguity with top-level functions.
const KERNEL_METHODS: &[&[u8]] = &[
    b"open",
    b"puts",
    b"print",
    b"p",
    b"pp",
    b"warn",
    b"fail",
    b"sleep",
    b"exit",
    b"exit!",
    b"abort",
    b"at_exit",
    b"fork",
    b"exec",
    b"system",
    b"spawn",
    b"rand",
    b"srand",
    b"gets",
    b"readline",
    b"readlines",
    b"select",
    b"format",
    b"sprintf",
    b"printf",
    b"putc",
    b"loop",
    b"require",
    b"require_relative",
    b"load",
    b"autoload",
    b"autoload?",
    b"binding",
    b"block_given?",
    b"iterator?",
    b"caller",
    b"caller_locations",
    b"catch",
    b"throw",
    b"global_variables",
    b"local_variables",
    b"set_trace_func",
    b"trace_var",
    b"untrace_var",
    b"trap",
    b"lambda",
    b"proc",
    b"Array",
    b"Complex",
    b"Float",
    b"Hash",
    b"Integer",
    b"Rational",
    b"String",
    b"__callee__",
    b"__dir__",
    b"__method__",
];

/// Returns true if the method name starts with an uppercase letter,
/// which could be confused with a constant reference.
fn is_uppercase_method(name: &[u8]) -> bool {
    name.first().is_some_and(|&b| b.is_ascii_uppercase())
}

impl Cop for RedundantSelf {
    fn name(&self) -> &'static str {
        "Style/RedundantSelf"
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
        let mut visitor = RedundantSelfVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            local_scopes: vec![HashSet::new()],
            allowed_self_methods: HashSet::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct RedundantSelfVisitor<'a> {
    cop: &'a RedundantSelf,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Stack of local variable scopes. Each method/block introduces a new scope.
    local_scopes: Vec<HashSet<Vec<u8>>>,
    /// Method names where `self.x` is allowed because a prior compound assignment
    /// (`self.x ||=`, `self.x &&=`, `self.x +=`, etc.) appeared earlier in the
    /// current enclosing file/class/module. This matches RuboCop's source-order
    /// accumulation across later methods, while still excluding plain setters.
    allowed_self_methods: HashSet<Vec<u8>>,
}

impl RedundantSelfVisitor<'_> {
    fn add_local(&mut self, name: &[u8]) {
        if let Some(scope) = self.local_scopes.last_mut() {
            scope.insert(name.to_vec());
        }
    }

    fn is_local_variable(&self, name: &[u8]) -> bool {
        for scope in self.local_scopes.iter().rev() {
            if scope.contains(name) {
                return true;
            }
        }
        false
    }

    fn add_allowed_self_method(&mut self, name: &[u8]) {
        self.allowed_self_methods.insert(name.to_vec());
    }

    fn is_allowed_self_method(&self, name: &[u8]) -> bool {
        self.allowed_self_methods.contains(name)
    }

    fn collect_params_from_node(&mut self, params: &ruby_prism::ParametersNode<'_>) {
        for p in params.requireds().iter() {
            if let Some(req) = p.as_required_parameter_node() {
                self.add_local(req.name().as_slice());
            }
        }
        for p in params.optionals().iter() {
            if let Some(op) = p.as_optional_parameter_node() {
                self.add_local(op.name().as_slice());
            }
        }
        if let Some(rest) = params.rest() {
            if let Some(rp) = rest.as_rest_parameter_node() {
                if let Some(name) = rp.name() {
                    self.add_local(name.as_slice());
                }
            }
        }
        for p in params.keywords().iter() {
            if let Some(kw) = p.as_required_keyword_parameter_node() {
                self.add_local(kw.name().as_slice());
            } else if let Some(kw) = p.as_optional_keyword_parameter_node() {
                self.add_local(kw.name().as_slice());
            }
        }
        if let Some(kw_rest) = params.keyword_rest() {
            if let Some(kw_rest_param) = kw_rest.as_keyword_rest_parameter_node() {
                if let Some(name) = kw_rest_param.name() {
                    self.add_local(name.as_slice());
                }
            }
        }
        if let Some(block) = params.block() {
            if let Some(name) = block.name() {
                self.add_local(name.as_slice());
            }
        }
    }

    /// Apply the results of a conditional prescan to the current scope.
    fn apply_conditional_prescan(&mut self, scanner: ConditionalLocalScanner) {
        for name in scanner.names {
            self.add_local(&name);
        }
    }

    fn merge_current_scope_into_parent(&mut self) {
        if self.local_scopes.len() < 2 {
            return;
        }

        let current_scope = self.local_scopes.pop().unwrap();
        if let Some(parent_scope) = self.local_scopes.last_mut() {
            parent_scope.extend(current_scope);
        }
    }
}

/// Prescan visitor for conditional nodes (`if`/`unless`/`while`/`until`).
/// Collects all local variable names from ALL descendants, including those
/// inside blocks, lambdas, defs, classes, and modules. This matches RuboCop's
/// `node.each_descendant(:lvasgn, :masgn)` behavior in `on_if`.
struct ConditionalLocalScanner {
    names: Vec<Vec<u8>>,
}

impl<'pr> Visit<'pr> for ConditionalLocalScanner {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        self.names.push(node.name().as_slice().to_vec());
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_local_variable_target_node(
        &mut self,
        node: &ruby_prism::LocalVariableTargetNode<'pr>,
    ) {
        self.names.push(node.name().as_slice().to_vec());
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        self.names.push(node.name().as_slice().to_vec());
        ruby_prism::visit_local_variable_or_write_node(self, node);
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        self.names.push(node.name().as_slice().to_vec());
        ruby_prism::visit_local_variable_and_write_node(self, node);
    }

    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        self.names.push(node.name().as_slice().to_vec());
        ruby_prism::visit_local_variable_operator_write_node(self, node);
    }

    // Don't stop at any scope boundary — scan everything (matches RuboCop's each_descendant)
}

impl<'pr> Visit<'pr> for RedundantSelfVisitor<'_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        self.local_scopes.push(HashSet::new());

        if let Some(params) = node.parameters() {
            // Collect parameter names into scope first (before visiting defaults).
            // This ensures `def foo(x = self.x)` sees `x` as a local, matching RuboCop.
            self.collect_params_from_node(&params);

            // Visit parameter default value expressions — they may contain `self.` calls
            // that should be checked for redundancy.
            for p in params.optionals().iter() {
                if let Some(op) = p.as_optional_parameter_node() {
                    self.visit(&op.value());
                }
            }
            for p in params.keywords().iter() {
                if let Some(kw) = p.as_optional_keyword_parameter_node() {
                    self.visit(&kw.value());
                }
            }
        }

        // No prescan — locals are tracked in visit order, matching RuboCop's
        // lazy variable tracking. `self.x` before `x = ...` is flagged.
        if let Some(body) = node.body() {
            self.visit(&body);
        }

        self.local_scopes.pop();
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if let Some(receiver) = node.receiver() {
            if receiver.as_self_node().is_some() {
                if let Some(call_op) = node.call_operator_loc() {
                    if call_op.as_slice() == b"." {
                        let method_name = node.name();
                        let name_bytes = method_name.as_slice();

                        let is_setter = name_bytes.ends_with(b"=")
                            && name_bytes != b"=="
                            && name_bytes != b"!="
                            && name_bytes != b"<="
                            && name_bytes != b">="
                            && name_bytes != b"===";

                        if !is_setter
                            && name_bytes != b"[]"
                            && name_bytes != b"[]="
                            && !ALLOWED_METHODS.contains(&name_bytes)
                            && !KERNEL_METHODS.contains(&name_bytes)
                            && !is_uppercase_method(name_bytes)
                            && !self.is_local_variable(name_bytes)
                            && !self.is_allowed_self_method(name_bytes)
                        {
                            let self_loc = receiver.location();
                            let (line, column) =
                                self.source.offset_to_line_col(self_loc.start_offset());
                            self.diagnostics.push(self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                "Redundant `self` detected.".to_string(),
                            ));
                        }
                    }
                }
            }
        }

        // Visit receiver (for chained calls like self.name.demodulize — we need to
        // check the inner self.name), arguments, and block.
        if let Some(receiver) = node.receiver() {
            // Only visit non-self receivers (self is already handled above)
            if receiver.as_self_node().is_none() {
                self.visit(&receiver);
            }
        }
        if let Some(args) = node.arguments() {
            for arg in args.arguments().iter() {
                self.visit(&arg);
            }
        }
        if let Some(block) = node.block() {
            self.visit(&block);
        }
    }

    // Class/module bodies have a different `self` context.
    // We still need to visit them to find `self.` calls within method definitions.
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        // Push a new scope for the class body (local variables from the enclosing scope
        // are not visible inside a class body).
        self.local_scopes.push(HashSet::new());
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.local_scopes.pop();
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        self.local_scopes.push(HashSet::new());
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.local_scopes.pop();
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        self.local_scopes.push(HashSet::new());
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.local_scopes.pop();
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // Block parameters shadow method names — `self.x` is required for
        // disambiguation when a block parameter `x` is in scope.
        // Push a new scope for block params (they are block-local).
        self.local_scopes.push(HashSet::new());

        if let Some(params) = node.parameters() {
            if let Some(block_params) = params.as_block_parameters_node() {
                if let Some(inner_params) = block_params.parameters() {
                    self.collect_params_from_node(&inner_params);
                }
            }
        }

        if let Some(body) = node.body() {
            self.visit(&body);
        }

        self.merge_current_scope_into_parent();
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        // Lambda parameters work the same as block parameters for scoping.
        self.local_scopes.push(HashSet::new());

        if let Some(params) = node.parameters() {
            if let Some(block_params) = params.as_block_parameters_node() {
                if let Some(inner_params) = block_params.parameters() {
                    self.collect_params_from_node(&inner_params);
                }
            }
        }

        if let Some(body) = node.body() {
            self.visit(&body);
        }

        self.merge_current_scope_into_parent();
    }

    // --- Local variable tracking in visit order (replaces prescan) ---

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        // Add local BEFORE visiting value — matches RuboCop where `x = self.x`
        // does NOT flag self.x (x is already in scope on the RHS).
        self.add_local(node.name().as_slice());
        self.visit(&node.value());
    }

    fn visit_local_variable_target_node(
        &mut self,
        node: &ruby_prism::LocalVariableTargetNode<'pr>,
    ) {
        self.add_local(node.name().as_slice());
        // No children to visit
        let _ = node;
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        self.add_local(node.name().as_slice());
        self.visit(&node.value());
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        self.add_local(node.name().as_slice());
        self.visit(&node.value());
    }

    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        self.add_local(node.name().as_slice());
        self.visit(&node.value());
    }

    // --- Conditional prescan: if/unless/while/until ---
    // RuboCop's on_if scans all descendants (including inside blocks) for lvasgn
    // and adds those variable names to the scope before visiting. This makes
    // `self.x` allowed in the condition when `x` is assigned anywhere in the body.

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let mut scanner = ConditionalLocalScanner { names: Vec::new() };
        ruby_prism::visit_if_node(&mut scanner, node);
        self.apply_conditional_prescan(scanner);
        ruby_prism::visit_if_node(self, node);
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        let mut scanner = ConditionalLocalScanner { names: Vec::new() };
        ruby_prism::visit_unless_node(&mut scanner, node);
        self.apply_conditional_prescan(scanner);
        ruby_prism::visit_unless_node(self, node);
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        let mut scanner = ConditionalLocalScanner { names: Vec::new() };
        ruby_prism::visit_while_node(&mut scanner, node);
        self.apply_conditional_prescan(scanner);
        ruby_prism::visit_while_node(self, node);
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        let mut scanner = ConditionalLocalScanner { names: Vec::new() };
        ruby_prism::visit_until_node(&mut scanner, node);
        self.apply_conditional_prescan(scanner);
        ruby_prism::visit_until_node(self, node);
    }

    fn visit_call_or_write_node(&mut self, node: &ruby_prism::CallOrWriteNode<'pr>) {
        ruby_prism::visit_call_or_write_node(self, node);

        if let Some(receiver) = node.receiver() {
            if receiver.as_self_node().is_some() {
                self.add_allowed_self_method(node.read_name().as_slice());
            }
        }
    }

    fn visit_call_and_write_node(&mut self, node: &ruby_prism::CallAndWriteNode<'pr>) {
        ruby_prism::visit_call_and_write_node(self, node);

        if let Some(receiver) = node.receiver() {
            if receiver.as_self_node().is_some() {
                self.add_allowed_self_method(node.read_name().as_slice());
            }
        }
    }

    fn visit_call_operator_write_node(&mut self, node: &ruby_prism::CallOperatorWriteNode<'pr>) {
        ruby_prism::visit_call_operator_write_node(self, node);

        if let Some(receiver) = node.receiver() {
            if receiver.as_self_node().is_some() {
                self.add_allowed_self_method(node.read_name().as_slice());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantSelf, "cops/style/redundant_self");
}
