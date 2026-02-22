use crate::cop::node_type::{CALL_NODE, SYMBOL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct AccessModifierDeclarations;

const ACCESS_MODIFIERS: &[&str] = &["private", "protected", "public", "module_function"];

impl Cop for AccessModifierDeclarations {
    fn name(&self) -> &'static str {
        "Style/AccessModifierDeclarations"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, SYMBOL_NODE]
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
        let enforced_style = config.get_str("EnforcedStyle", "group");
        let allow_modifiers_on_symbols = config.get_bool("AllowModifiersOnSymbols", true);
        let allow_modifiers_on_attrs = config.get_bool("AllowModifiersOnAttrs", true);
        let allow_modifiers_on_alias_method = config.get_bool("AllowModifiersOnAliasMethod", true);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if !ACCESS_MODIFIERS.contains(&method_name) {
            return;
        }

        // Skip if no receiver (must be bare access modifier call)
        if call.receiver().is_some() {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return, // Group-style modifier with no args is fine
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // Check if the argument is a symbol
        let first_arg = &arg_list[0];
        let is_symbol_arg = first_arg.as_symbol_node().is_some();

        if is_symbol_arg && allow_modifiers_on_symbols {
            return;
        }

        // Check for attr_* calls
        if allow_modifiers_on_attrs {
            if let Some(inner_call) = first_arg.as_call_node() {
                let inner_name = std::str::from_utf8(inner_call.name().as_slice()).unwrap_or("");
                if matches!(
                    inner_name,
                    "attr_reader" | "attr_writer" | "attr_accessor" | "attr"
                ) {
                    return;
                }
            }
        }

        // Check for alias_method
        if allow_modifiers_on_alias_method {
            if let Some(inner_call) = first_arg.as_call_node() {
                let inner_name = std::str::from_utf8(inner_call.name().as_slice()).unwrap_or("");
                if inner_name == "alias_method" {
                    return;
                }
            }
        }

        // Distinguish between inline modifier declarations and visibility-change calls:
        // - Inline modifier declaration: `private def foo` (argument is a DefNode) — offense under group style
        // - Visibility-change call with symbol: `private :foo` — already handled above (AllowModifiersOnSymbols)
        // - Visibility-change call with method/variable: `public target`, `private method_var` — NOT an offense
        //   These change visibility of an already-defined method, not inline declarations.
        let is_inline_modifier =
            first_arg.as_def_node().is_some() || first_arg.as_symbol_node().is_some();

        match enforced_style {
            "inline" => {
                // Inline style: access modifiers should be applied to individual methods
                // If we see a bare modifier without args inside a class, it's group style
                // This is only triggered for group-style, which is bare modifier without args
                // Since we already checked args exist, this is inline-style with args = OK
            }
            "group" => {
                // Group style: access modifiers should not be inlined with method definitions
                // Only flag actual inline modifier declarations, not visibility-change calls
                if !is_inline_modifier {
                    return;
                }

                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "`{}` should not be inlined in method definitions.",
                        method_name
                    ),
                ));
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        AccessModifierDeclarations,
        "cops/style/access_modifier_declarations"
    );
}
