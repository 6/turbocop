use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct ClassMethods;

impl Cop for ClassMethods {
    fn name(&self) -> &'static str {
        "Style/ClassMethods"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut visitor = ClassMethodsVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            class_names: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ClassMethodsVisitor<'a, 'src> {
    cop: &'a ClassMethods,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    class_names: Vec<Vec<u8>>,
}

impl<'pr> Visit<'pr> for ClassMethodsVisitor<'_, '_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let name = node.constant_path().location().as_slice().to_vec();
        self.class_names.push(name);
        // Visit children
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.class_names.pop();
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let name = node.constant_path().location().as_slice().to_vec();
        self.class_names.push(name);
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.class_names.pop();
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        // Only check singleton methods (def X.method)
        let receiver = match node.receiver() {
            Some(r) => r,
            None => return,
        };

        // Get the class/module name we're inside
        let current_class = match self.class_names.last() {
            Some(n) => n,
            None => return,
        };

        // Check if receiver matches the enclosing class/module name
        let recv_bytes = receiver.location().as_slice();
        if recv_bytes == current_class.as_slice() {
            let method_name = node.name();
            let (line, column) = self.source.offset_to_line_col(receiver.location().start_offset());
            let msg = format!(
                "Use `self.{}` instead of `{}.{}`.",
                String::from_utf8_lossy(method_name.as_slice()),
                String::from_utf8_lossy(current_class),
                String::from_utf8_lossy(method_name.as_slice()),
            );
            self.diagnostics.push(self.cop.diagnostic(self.source, line, column, msg));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ClassMethods, "cops/style/class_methods");
}
