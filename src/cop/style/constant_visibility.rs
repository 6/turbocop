use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use std::collections::HashSet;

pub struct ConstantVisibility;

impl Cop for ConstantVisibility {
    fn name(&self) -> &'static str {
        "Style/ConstantVisibility"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let _ignore_pattern = config.get_str("IgnoreModuleContaining", "");

        // Only check class and module bodies
        let body = if let Some(class_node) = node.as_class_node() {
            class_node.body()
        } else if let Some(module_node) = node.as_module_node() {
            module_node.body()
        } else {
            return Vec::new();
        };

        let body = match body {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Collect constant names that have visibility declarations
        let mut visible_constants: HashSet<String> = HashSet::new();

        for stmt in stmts.body().iter() {
            if let Some(call) = stmt.as_call_node() {
                let name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
                if name == "private_constant" || name == "public_constant" {
                    if let Some(args) = call.arguments() {
                        for arg in args.arguments().iter() {
                            if let Some(sym) = arg.as_symbol_node() {
                                let sym_name = std::str::from_utf8(sym.unescaped())
                                    .unwrap_or("");
                                visible_constants.insert(sym_name.to_string());
                            }
                        }
                    }
                }
            }
        }

        // Check for constant assignments without visibility
        let mut diagnostics = Vec::new();
        for stmt in stmts.body().iter() {
            if let Some(const_write) = stmt.as_constant_write_node() {
                let const_name = std::str::from_utf8(const_write.name().as_slice())
                    .unwrap_or("");
                if !visible_constants.contains(const_name) {
                    let loc = stmt.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!(
                            "Explicitly make `{}` public or private using either `#public_constant` or `#private_constant`.",
                            const_name
                        ),
                    ));
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ConstantVisibility, "cops/style/constant_visibility");
}
