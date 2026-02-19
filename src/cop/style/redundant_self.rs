use std::collections::HashSet;

use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantSelf;

/// Methods where self. is always required (keywords, operators, etc.)
const ALLOWED_METHODS: &[&[u8]] = &[
    b"class", b"module", b"def", b"end", b"begin", b"rescue", b"ensure",
    b"if", b"unless", b"while", b"until", b"for", b"do", b"return",
    b"yield", b"super", b"raise", b"defined?",
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
    ) {
        let mut visitor = RedundantSelfVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            local_scopes: vec![HashSet::new()],
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

    fn visit_local_variable_target_node(&mut self, node: &ruby_prism::LocalVariableTargetNode<'pr>) {
        self.names.push(node.name().as_slice().to_vec());
    }

    fn visit_local_variable_or_write_node(&mut self, node: &ruby_prism::LocalVariableOrWriteNode<'pr>) {
        self.names.push(node.name().as_slice().to_vec());
        self.visit(&node.value());
    }

    fn visit_local_variable_and_write_node(&mut self, node: &ruby_prism::LocalVariableAndWriteNode<'pr>) {
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
                        {
                            let self_loc = receiver.location();
                            let (line, column) = self.source.offset_to_line_col(self_loc.start_offset());
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

        // Visit arguments and block but NOT the receiver (already handled)
        if let Some(args) = node.arguments() {
            for arg in args.arguments().iter() {
                self.visit(&arg);
            }
        }
        if let Some(block) = node.block() {
            self.visit(&block);
        }
    }

    // Don't recurse into class/module (separate scope for self)
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantSelf, "cops/style/redundant_self");
}
