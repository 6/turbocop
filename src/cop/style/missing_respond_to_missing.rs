use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct MissingRespondToMissing;

impl Cop for MissingRespondToMissing {
    fn name(&self) -> &'static str {
        "Style/MissingRespondToMissing"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut visitor = MethodMissingVisitor {
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct MethodMissingVisitor<'src> {
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl MethodMissingVisitor<'_> {
    /// Check a class or module body for method_missing without respond_to_missing?
    fn check_body(&mut self, body: Option<ruby_prism::Node<'_>>) {
        let body = match body {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        // Collect all method defs in the body (one level deep only)
        let mut has_instance_method_missing = Vec::new();
        let mut has_instance_respond_to_missing = false;
        let mut has_class_method_missing = Vec::new();
        let mut has_class_respond_to_missing = false;

        for stmt in stmts.body().into_iter() {
            // Check direct def nodes
            if let Some(def_node) = stmt.as_def_node() {
                let name = def_node.name();
                let name_bytes = name.as_slice();

                if def_node.receiver().is_some() {
                    // self.method_missing or self.respond_to_missing?
                    if name_bytes == b"method_missing" {
                        has_class_method_missing.push(def_node.location().start_offset());
                    } else if name_bytes == b"respond_to_missing?" {
                        has_class_respond_to_missing = true;
                    }
                } else {
                    if name_bytes == b"method_missing" {
                        has_instance_method_missing.push(def_node.location().start_offset());
                    } else if name_bytes == b"respond_to_missing?" {
                        has_instance_respond_to_missing = true;
                    }
                }
            }

            // Check for inline access modifier: `private def method_missing`
            if let Some(call_node) = stmt.as_call_node() {
                let method_bytes = call_node.name().as_slice();
                if method_bytes == b"private" || method_bytes == b"protected" || method_bytes == b"public" {
                    if let Some(args) = call_node.arguments() {
                        for arg in args.arguments().into_iter() {
                            if let Some(def_node) = arg.as_def_node() {
                                let name = def_node.name();
                                let name_bytes = name.as_slice();

                                if def_node.receiver().is_some() {
                                    if name_bytes == b"method_missing" {
                                        has_class_method_missing.push(def_node.location().start_offset());
                                    } else if name_bytes == b"respond_to_missing?" {
                                        has_class_respond_to_missing = true;
                                    }
                                } else {
                                    if name_bytes == b"method_missing" {
                                        has_instance_method_missing.push(def_node.location().start_offset());
                                    } else if name_bytes == b"respond_to_missing?" {
                                        has_instance_respond_to_missing = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Flag instance method_missing without instance respond_to_missing?
        if !has_instance_respond_to_missing {
            for offset in &has_instance_method_missing {
                let (line, column) = self.source.offset_to_line_col(*offset);
                self.diagnostics.push(MissingRespondToMissing.diagnostic(
                    self.source,
                    line,
                    column,
                    "When using `method_missing`, define `respond_to_missing?`.".to_string(),
                ));
            }
        }

        // Flag class method_missing without class respond_to_missing?
        if !has_class_respond_to_missing {
            for offset in &has_class_method_missing {
                let (line, column) = self.source.offset_to_line_col(*offset);
                self.diagnostics.push(MissingRespondToMissing.diagnostic(
                    self.source,
                    line,
                    column,
                    "When using `method_missing`, define `respond_to_missing?`.".to_string(),
                ));
            }
        }
    }
}

impl<'pr> Visit<'pr> for MethodMissingVisitor<'_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        self.check_body(node.body());
        // Don't recurse into nested classes/modules here;
        // the Visit trait will call us for nested classes automatically
        ruby_prism::visit_class_node(self, node);
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        self.check_body(node.body());
        ruby_prism::visit_module_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MissingRespondToMissing, "cops/style/missing_respond_to_missing");
}
