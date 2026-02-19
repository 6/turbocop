use crate::cop::factory_bot::{is_factory_call, FACTORY_BOT_METHODS, FACTORY_BOT_SPEC_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, STRING_NODE, SYMBOL_NODE};

pub struct FactoryNameStyle;

impl Cop for FactoryNameStyle {
    fn name(&self) -> &'static str {
        "FactoryBot/FactoryNameStyle"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        FACTORY_BOT_SPEC_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if !FACTORY_BOT_METHODS.contains(&method_name) {
            return Vec::new();
        }

        let explicit_only = config.get_bool("ExplicitOnly", false);
        if !is_factory_call(call.receiver(), explicit_only) {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        let first_arg = &arg_list[0];
        let style = config.get_str("EnforcedStyle", "symbol");

        if style == "symbol" {
            // Flag string names (but not interpolated strings or namespaced strings with /)
            if let Some(str_node) = first_arg.as_string_node() {
                let value = str_node.unescaped();
                let value_str = std::str::from_utf8(value).unwrap_or("");

                // Skip namespaced names (contain /)
                if value_str.contains('/') {
                    return Vec::new();
                }

                let loc = first_arg.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use symbol to refer to a factory.".to_string(),
                )];
            }
            // Skip interpolated strings
        } else if style == "string" {
            // Flag symbol names
            if first_arg.as_symbol_node().is_some() {
                let loc = first_arg.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use string to refer to a factory.".to_string(),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FactoryNameStyle, "cops/factorybot/factory_name_style");
}
