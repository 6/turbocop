use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct OneClassPerFile;

impl Cop for OneClassPerFile {
    fn name(&self) -> &'static str {
        "Style/OneClassPerFile"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut visitor = ClassCollector {
            source,
            classes: Vec::new(),
            depth: 0,
        };
        visitor.visit(&parse_result.node());

        // If there's more than one top-level class, flag all but the first
        if visitor.classes.len() > 1 {
            for &(line, column) in &visitor.classes[1..] {
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Only define one class per source file.".to_string(),
                ));
            }
        }
    }
}

struct ClassCollector<'src> {
    source: &'src SourceFile,
    classes: Vec<(usize, usize)>,
    /// Nesting depth: 0 means file top-level. Modules, classes, blocks, defs,
    /// and singleton classes all increment depth so that only truly top-level
    /// class definitions are counted.
    depth: usize,
}

impl<'pr> Visit<'pr> for ClassCollector<'_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        if self.depth == 0 {
            let loc = node.class_keyword_loc();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.classes.push((line, column));
        }
        self.depth += 1;
        ruby_prism::visit_class_node(self, node);
        self.depth -= 1;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        // Modules increase nesting depth. Classes inside modules are not
        // top-level class definitions and should not be counted.
        self.depth += 1;
        ruby_prism::visit_module_node(self, node);
        self.depth -= 1;
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        self.depth += 1;
        ruby_prism::visit_singleton_class_node(self, node);
        self.depth -= 1;
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        self.depth += 1;
        ruby_prism::visit_def_node(self, node);
        self.depth -= 1;
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        self.depth += 1;
        ruby_prism::visit_block_node(self, node);
        self.depth -= 1;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        self.depth += 1;
        ruby_prism::visit_lambda_node(self, node);
        self.depth -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OneClassPerFile, "cops/style/one_class_per_file");
}
