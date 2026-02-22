use std::collections::HashSet;

use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantSelf;

/// Methods where self. is always required (keywords, operators, etc.)
const ALLOWED_METHODS: &[&[u8]] = &[
    b"class",
    b"module",
    b"def",
    b"end",
    b"begin",
    b"rescue",
    b"ensure",
    b"if",
    b"unless",
    b"while",
    b"until",
    b"for",
    b"do",
    b"return",
    b"yield",
    b"super",
    b"raise",
    b"defined?",
];

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
    /// Method names where `self.x` is allowed because `self.x = ...` or
    /// `self.x ||= ...` (compound assignment) appears in the same scope.
    /// RuboCop allows `self.reader` when `self.writer=` is used in the file.
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

    /// Collect local variable names from the method/block body by scanning
    /// for LocalVariableWriteNode and LocalVariableTargetNode at the top level.
    /// We need to pre-scan because Ruby allows `self.foo` BEFORE `foo = ...`
    /// to still be shadowed (the local variable is visible throughout the scope).
    fn prescan_locals(&mut self, body: &ruby_prism::Node<'_>) {
        let mut scanner = LocalScanner { names: Vec::new() };
        scanner.visit(body);
        for name in scanner.names {
            self.add_local(&name);
        }
    }

    /// Pre-scan for `self.x = ...`, `self.x ||= ...`, `self.x op= ...` patterns.
    /// When a setter `self.foo=` is used, `self.foo` (the reader) is allowed
    /// because removing `self` would create a local variable assignment instead.
    fn prescan_self_assignments(&mut self, body: &ruby_prism::Node<'_>) {
        let mut scanner = SelfAssignmentScanner { names: Vec::new() };
        scanner.visit(body);
        for name in scanner.names {
            self.allowed_self_methods.insert(name);
        }
    }
}

/// Pre-scan visitor that collects method names used in `self.x = ...` compound assignments.
struct SelfAssignmentScanner {
    names: Vec<Vec<u8>>,
}

impl SelfAssignmentScanner {
    /// If the node is a self.foo= call (including compound assignment targets),
    /// record "foo" as an allowed self-method.
    fn check_self_call(&mut self, node: &ruby_prism::CallNode<'_>) {
        if let Some(receiver) = node.receiver() {
            if receiver.as_self_node().is_some() {
                let name = node.name();
                let name_bytes = name.as_slice();
                // self.foo= setter → allow self.foo
                if name_bytes.ends_with(b"=")
                    && name_bytes.len() > 1
                    && name_bytes != b"=="
                    && name_bytes != b"!="
                    && name_bytes != b"<="
                    && name_bytes != b">="
                    && name_bytes != b"==="
                {
                    // Strip trailing "=" to get the reader name
                    self.names.push(name_bytes[..name_bytes.len() - 1].to_vec());
                }
            }
        }
    }
}

impl<'pr> Visit<'pr> for SelfAssignmentScanner {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        self.check_self_call(node);
        // Visit children for nested calls
        if let Some(receiver) = node.receiver() {
            self.visit(&receiver);
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

    // Also catch compound assignment operators: self.x ||= v, self.x &&= v, self.x += v
    fn visit_call_or_write_node(&mut self, node: &ruby_prism::CallOrWriteNode<'pr>) {
        if let Some(receiver) = node.receiver() {
            if receiver.as_self_node().is_some() {
                let name = node.read_name();
                self.names.push(name.as_slice().to_vec());
            }
        }
    }

    fn visit_call_and_write_node(&mut self, node: &ruby_prism::CallAndWriteNode<'pr>) {
        if let Some(receiver) = node.receiver() {
            if receiver.as_self_node().is_some() {
                let name = node.read_name();
                self.names.push(name.as_slice().to_vec());
            }
        }
    }

    fn visit_call_operator_write_node(&mut self, node: &ruby_prism::CallOperatorWriteNode<'pr>) {
        if let Some(receiver) = node.receiver() {
            if receiver.as_self_node().is_some() {
                let name = node.read_name();
                self.names.push(name.as_slice().to_vec());
            }
        }
    }

    // Don't descend into nested scopes
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

/// Pre-scan visitor that collects all local variable names in a scope.
struct LocalScanner {
    names: Vec<Vec<u8>>,
}

impl<'pr> Visit<'pr> for LocalScanner {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        self.names.push(node.name().as_slice().to_vec());
        // Continue visiting the value expression
        self.visit(&node.value());
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
        self.visit(&node.value());
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        self.names.push(node.name().as_slice().to_vec());
        self.visit(&node.value());
    }

    // Don't descend into nested scopes
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

impl<'pr> Visit<'pr> for RedundantSelfVisitor<'_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        self.local_scopes.push(HashSet::new());

        if let Some(params) = node.parameters() {
            self.collect_params_from_node(&params);
        }

        // Pre-scan the body to collect all local variable names.
        // In Ruby, a local variable assignment anywhere in a scope makes
        // that name a local variable throughout the entire scope.
        if let Some(body) = node.body() {
            self.prescan_locals(&body);
            // Also scan for self.x= / self.x ||= within this method
            self.prescan_self_assignments(&body);
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
                            && !ALLOWED_METHODS.iter().any(|&m| m == name_bytes)
                            && !self.is_local_variable(name_bytes)
                            && !self.allowed_self_methods.contains(name_bytes)
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
        // Pre-scan the entire class body for self.x= / self.x ||= patterns.
        // These make `self.x` (the reader) allowed across all methods in the class.
        let saved = std::mem::take(&mut self.allowed_self_methods);
        if let Some(body) = node.body() {
            self.prescan_self_assignments(&body);
            self.visit(&body);
        }
        self.allowed_self_methods = saved;
        self.local_scopes.pop();
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        self.local_scopes.push(HashSet::new());
        let saved = std::mem::take(&mut self.allowed_self_methods);
        if let Some(body) = node.body() {
            self.prescan_self_assignments(&body);
            self.visit(&body);
        }
        self.allowed_self_methods = saved;
        self.local_scopes.pop();
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        self.local_scopes.push(HashSet::new());
        let saved = std::mem::take(&mut self.allowed_self_methods);
        if let Some(body) = node.body() {
            self.prescan_self_assignments(&body);
            self.visit(&body);
        }
        self.allowed_self_methods = saved;
        self.local_scopes.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantSelf, "cops/style/redundant_self");
}
