use crate::cop::node_type::{
    CALL_NODE, CLASS_NODE, CONSTANT_PATH_WRITE_NODE, CONSTANT_WRITE_NODE, MODULE_NODE,
    STATEMENTS_NODE, SYMBOL_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use std::collections::HashSet;

/// Checks that constants defined in classes and modules have an explicit
/// visibility declaration (`public_constant` or `private_constant`).
///
/// Investigation note: the remaining corpus FN examples for nested class/module
/// constants (`CONSUME_ON_ESCAPE`, `ClosedQueueError`, `TYPE`, `DEFAULTS`, etc.)
/// reproduce as offenses in isolated fixture/stdin runs once they are placed in
/// their original class/module scopes, so this cop's AST detection matches
/// RuboCop for those shapes. The unresolved corpus mismatch comes from the
/// file-based corpus run honoring repo/default config instead of the explicit
/// baseline config, which leaves this disabled-by-default cop turned off.
pub struct ConstantVisibility;

impl Cop for ConstantVisibility {
    fn name(&self) -> &'static str {
        "Style/ConstantVisibility"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CLASS_NODE,
            CONSTANT_PATH_WRITE_NODE,
            CONSTANT_WRITE_NODE,
            MODULE_NODE,
            STATEMENTS_NODE,
            SYMBOL_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let _ignore_pattern = config.get_str("IgnoreModuleContaining", "");
        let ignore_modules = config.get_bool("IgnoreModules", false);

        // Only check class and module bodies
        let body = if let Some(class_node) = node.as_class_node() {
            class_node.body()
        } else if let Some(module_node) = node.as_module_node() {
            if ignore_modules {
                return;
            }
            module_node.body()
        } else {
            return;
        };

        let body = match body {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
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
                                let sym_name = std::str::from_utf8(sym.unescaped()).unwrap_or("");
                                visible_constants.insert(sym_name.to_string());
                            }
                        }
                    }
                }
            }
        }

        // Check for constant assignments without visibility
        for stmt in stmts.body().iter() {
            let const_name = if let Some(const_write) = stmt.as_constant_write_node() {
                Some(
                    std::str::from_utf8(const_write.name().as_slice())
                        .unwrap_or("")
                        .to_string(),
                )
            } else if let Some(cpw) = stmt.as_constant_path_write_node() {
                cpw.target()
                    .name()
                    .and_then(|n| std::str::from_utf8(n.as_slice()).ok())
                    .map(|s| s.to_string())
            } else {
                None
            };

            if let Some(const_name) = const_name {
                if !visible_constants.contains(&const_name) {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ConstantVisibility, "cops/style/constant_visibility");
}
