use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for useless constant scoping under `private` access modifier.
/// Private constants must be defined using `private_constant`, not `private`.
pub struct UselessConstantScoping;

impl Cop for UselessConstantScoping {
    fn name(&self) -> &'static str {
        "Lint/UselessConstantScoping"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut visitor = ConstScopingVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ConstScopingVisitor<'a, 'src> {
    cop: &'a UselessConstantScoping,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for ConstScopingVisitor<'_, '_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        if let Some(body) = node.body() {
            self.check_body(&body);
        }
        ruby_prism::visit_class_node(self, node);
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        if let Some(body) = node.body() {
            self.check_body(&body);
        }
        ruby_prism::visit_module_node(self, node);
    }
}

impl ConstScopingVisitor<'_, '_> {
    fn check_body(&mut self, body: &ruby_prism::Node<'_>) {
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let body_nodes: Vec<_> = stmts.body().iter().collect();

        // Track private modifier and constant assignments
        let mut seen_private = false;
        let mut private_constant_names: Vec<Vec<u8>> = Vec::new();

        // First pass: collect private_constant names
        for node in &body_nodes {
            if let Some(call) = node.as_call_node() {
                if call.name().as_slice() == b"private_constant" && call.receiver().is_none() {
                    if let Some(args) = call.arguments() {
                        for arg in args.arguments().iter() {
                            if let Some(sym) = arg.as_symbol_node() {
                                private_constant_names.push(sym.unescaped().to_vec());
                            }
                        }
                    }
                }
            }
        }

        // Second pass: check for constants after private modifier
        for node in &body_nodes {
            if let Some(call) = node.as_call_node() {
                if call.name().as_slice() == b"private"
                    && call.receiver().is_none()
                    && call.arguments().is_none()
                {
                    seen_private = true;
                    continue;
                }
            }

            if seen_private {
                if let Some(casgn) = node.as_constant_write_node() {
                    let const_name = casgn.name().as_slice();
                    // Check if this constant has a private_constant call
                    if !private_constant_names.iter().any(|n| n.as_slice() == const_name) {
                        let loc = casgn.location();
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            "Useless `private` access modifier for constant scope.".to_string(),
                        ));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessConstantScoping, "cops/lint/useless_constant_scoping");
}
