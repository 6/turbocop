use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ModuleFunction;

impl Cop for ModuleFunction {
    fn name(&self) -> &'static str {
        "Style/ModuleFunction"
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
        let style = config.get_str("EnforcedStyle", "module_function");
        // Autocorrect config key acknowledged (autocorrect not yet implemented)
        let _autocorrect = config.get_bool("Autocorrect", false);
        let mut visitor = ModuleFunctionVisitor {
            cop: self,
            source,
            style,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ModuleFunctionVisitor<'a> {
    cop: &'a ModuleFunction,
    source: &'a SourceFile,
    style: &'a str,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for ModuleFunctionVisitor<'_> {
    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        if let Some(body) = node.body() {
            // Scan the body for `extend self` or `module_function`
            if let Some(stmts) = body.as_statements_node() {
                for stmt in stmts.body().iter() {
                    if let Some(call) = stmt.as_call_node() {
                        let method_bytes = call.name().as_slice();

                        if self.style == "module_function" && method_bytes == b"extend" {
                            // Check if argument is `self`
                            if call.receiver().is_none() {
                                if let Some(args) = call.arguments() {
                                    let arg_list: Vec<_> = args.arguments().iter().collect();
                                    if arg_list.len() == 1 && arg_list[0].as_self_node().is_some() {
                                        let loc = call.location();
                                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                                        self.diagnostics.push(self.cop.diagnostic(
                                            self.source,
                                            line,
                                            column,
                                            "Use `module_function` instead of `extend self`.".to_string(),
                                        ));
                                    }
                                }
                            }
                        } else if self.style == "extend_self" && method_bytes == b"module_function" {
                            // Check if it has no arguments (bare `module_function`)
                            if call.receiver().is_none() && call.arguments().is_none() {
                                let loc = call.location();
                                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                                self.diagnostics.push(self.cop.diagnostic(
                                    self.source,
                                    line,
                                    column,
                                    "Use `extend self` instead of `module_function`.".to_string(),
                                ));
                            }
                        } else if self.style == "forbidden" {
                            if method_bytes == b"module_function" && call.receiver().is_none() {
                                let loc = call.location();
                                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                                self.diagnostics.push(self.cop.diagnostic(
                                    self.source,
                                    line,
                                    column,
                                    "`module_function` and `extend self` are forbidden.".to_string(),
                                ));
                            } else if method_bytes == b"extend" && call.receiver().is_none() {
                                if let Some(args) = call.arguments() {
                                    let arg_list: Vec<_> = args.arguments().iter().collect();
                                    if arg_list.len() == 1 && arg_list[0].as_self_node().is_some() {
                                        let loc = call.location();
                                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                                        self.diagnostics.push(self.cop.diagnostic(
                                            self.source,
                                            line,
                                            column,
                                            "`module_function` and `extend self` are forbidden.".to_string(),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            self.visit(&body);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ModuleFunction, "cops/style/module_function");
}
