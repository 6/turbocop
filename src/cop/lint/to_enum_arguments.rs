use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct ToEnumArguments;

impl Cop for ToEnumArguments {
    fn name(&self) -> &'static str {
        "Lint/ToEnumArguments"
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
        let mut visitor = ToEnumVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            method_stack: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct MethodInfo {
    name: Vec<u8>,
    param_count: usize, // number of required + optional params (excluding block)
}

struct ToEnumVisitor<'a, 'src> {
    cop: &'a ToEnumArguments,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    method_stack: Vec<MethodInfo>,
}

impl<'pr> Visit<'pr> for ToEnumVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let name = node.name().as_slice().to_vec();

        // Count parameters (excluding block args)
        let param_count = if let Some(params) = node.parameters() {
            let mut count = 0;
            count += params.requireds().len();
            count += params.optionals().len();
            if params.rest().is_some() {
                count += 1;
            }
            count += params.keywords().len();
            if params.keyword_rest().is_some() {
                count += 1;
            }
            count
        } else {
            0
        };

        self.method_stack.push(MethodInfo { name, param_count });
        ruby_prism::visit_def_node(self, node);
        self.method_stack.pop();
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();

        if (method_name == b"to_enum" || method_name == b"enum_for")
            && (node.receiver().is_none() || node.receiver().as_ref().map_or(false, |r| r.as_self_node().is_some()))
        {
            if let Some(current_method) = self.method_stack.last() {
                if let Some(args) = node.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();

                    if !arg_list.is_empty() {
                        // First arg should be the method name
                        let first = &arg_list[0];
                        let refers_to_current = is_method_ref(first, &current_method.name);

                        if refers_to_current {
                            // Remaining args should match the method params
                            let provided_args = arg_list.len() - 1; // minus the method name arg
                            if provided_args < current_method.param_count {
                                let loc = node.location();
                                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                                self.diagnostics.push(self.cop.diagnostic(
                                    self.source,
                                    line,
                                    column,
                                    "Ensure you correctly provided all the arguments.".to_string(),
                                ));
                            }
                        }
                    }
                }
            }
        }

        // Visit children normally
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
        if let Some(args) = node.arguments() {
            self.visit(&args.as_node());
        }
        if let Some(block) = node.block() {
            self.visit(&block);
        }
    }

    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

fn is_method_ref(node: &ruby_prism::Node<'_>, method_name: &[u8]) -> bool {
    // Check for :method_name (symbol)
    if let Some(sym) = node.as_symbol_node() {
        return &*sym.unescaped() == method_name;
    }

    // Check for __method__ or __callee__
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if (name == b"__method__" || name == b"__callee__") && call.receiver().is_none() {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ToEnumArguments, "cops/lint/to_enum_arguments");
}
