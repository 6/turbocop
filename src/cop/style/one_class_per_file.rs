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
    ) -> Vec<Diagnostic> {
        let mut visitor = ClassCollector {
            source,
            classes: Vec::new(),
            depth: 0,
        };
        visitor.visit(&parse_result.node());

        // If there's more than one top-level class, flag all but the first
        if visitor.classes.len() > 1 {
            visitor.classes[1..]
                .iter()
                .map(|&(line, column)| {
                    self.diagnostic(
                        source,
                        line,
                        column,
                        "Only define one class per source file.".to_string(),
                    )
                })
                .collect()
        } else {
            Vec::new()
        }
    }
}

struct ClassCollector<'src> {
    source: &'src SourceFile,
    classes: Vec<(usize, usize)>,
    depth: usize,
}

impl<'pr> Visit<'pr> for ClassCollector<'_, > {
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
        // Classes inside modules still count as "top-level" in terms of the file,
        // but nested classes within classes don't. For this cop, we track class nesting.
        // Modules are transparent - a class inside a module at top level still counts.
        ruby_prism::visit_module_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OneClassPerFile, "cops/style/one_class_per_file");
}
