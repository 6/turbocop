use crate::cop::node_type::{
    CALL_NODE, CLASS_NODE, MODULE_NODE, SINGLETON_CLASS_NODE, STATEMENTS_NODE,
};
use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct AccessModifierIndentation;

const ACCESS_MODIFIERS: &[&[u8]] = &[b"private", b"protected", b"public", b"module_function"];

impl Cop for AccessModifierIndentation {
    fn name(&self) -> &'static str {
        "Layout/AccessModifierIndentation"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CLASS_NODE,
            MODULE_NODE,
            SINGLETON_CLASS_NODE,
            STATEMENTS_NODE,
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
        let style = config.get_str("EnforcedStyle", "indent");
        let _indent_width = config.get_usize("IndentationWidth", 2);

        // We need a class, module, sclass, or block node that contains access modifiers
        let (body, container_offset) = if let Some(class_node) = node.as_class_node() {
            match class_node.body() {
                Some(b) => (b, class_node.location().start_offset()),
                None => return,
            }
        } else if let Some(module_node) = node.as_module_node() {
            match module_node.body() {
                Some(b) => (b, module_node.location().start_offset()),
                None => return,
            }
        } else if let Some(sclass_node) = node.as_singleton_class_node() {
            match sclass_node.body() {
                Some(b) => (b, sclass_node.location().start_offset()),
                None => return,
            }
        } else {
            return;
        };

        // Get the indentation of the container keyword line
        let (container_line, _) = source.offset_to_line_col(container_offset);
        let container_indent = match util::line_at(source, container_line) {
            Some(line) => util::indentation_of(line),
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        for stmt in stmts.body().iter() {
            let call = match stmt.as_call_node() {
                Some(c) => c,
                None => continue,
            };

            // Must be a bare access modifier (no receiver, no arguments or one argument)
            if call.receiver().is_some() {
                continue;
            }

            let method_name = call.name().as_slice();
            if !ACCESS_MODIFIERS.contains(&method_name) {
                continue;
            }

            // Check if this is a bare modifier (no args) - skip inline modifiers like `private def foo`
            if let Some(args) = call.arguments() {
                // If the argument is a def node or a symbol, it's an inline modifier
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if !arg_list.is_empty() {
                    continue;
                }
            }

            let (mod_line, mod_col) = source.offset_to_line_col(call.location().start_offset());

            // Same line as container keyword — skip
            if mod_line == container_line {
                continue;
            }

            let expected_col = match style {
                "outdent" => container_indent,
                _ => container_indent + 2, // "indent" (default)
            };

            if mod_col != expected_col {
                let style_word = if style == "outdent" {
                    "Outdent"
                } else {
                    "Indent"
                };
                let modifier_name = std::str::from_utf8(method_name).unwrap_or("private");
                diagnostics.push(self.diagnostic(
                    source,
                    mod_line,
                    mod_col,
                    format!("{style_word} access modifiers like `{modifier_name}`."),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        AccessModifierIndentation,
        "cops/layout/access_modifier_indentation"
    );
}
