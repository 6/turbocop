use crate::cop::factory_bot::{is_factory_call, FACTORY_BOT_METHODS, FACTORY_BOT_SPEC_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ASSOC_NODE, CALL_NODE, HASH_NODE, IMPLICIT_NODE, KEYWORD_HASH_NODE, LOCAL_VARIABLE_READ_NODE, STRING_NODE, SYMBOL_NODE};

pub struct ConsistentParenthesesStyle;

impl Cop for ConsistentParenthesesStyle {
    fn name(&self) -> &'static str {
        "FactoryBot/ConsistentParenthesesStyle"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        FACTORY_BOT_SPEC_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, CALL_NODE, HASH_NODE, IMPLICIT_NODE, KEYWORD_HASH_NODE, LOCAL_VARIABLE_READ_NODE, STRING_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if !FACTORY_BOT_METHODS.contains(&method_name) {
            return;
        }

        let explicit_only = config.get_bool("ExplicitOnly", false);
        if !is_factory_call(call.receiver(), explicit_only) {
            return;
        }

        let style = config.get_str("EnforcedStyle", "require_parentheses");

        // Must have arguments
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // First argument must be a symbol, string, send, or local variable
        let first_arg = &arg_list[0];
        let valid_first_arg = first_arg.as_symbol_node().is_some()
            || first_arg.as_string_node().is_some()
            || first_arg.as_call_node().is_some()
            || first_arg.as_local_variable_read_node().is_some();

        if !valid_first_arg {
            return;
        }

        // `generate` with more than 1 argument is excluded
        if method_name == "generate" && arg_list.len() > 1 {
            return;
        }

        let has_parens = call.opening_loc().is_some();

        if style == "require_parentheses" && !has_parens {
            let msg_loc = call.message_loc().unwrap_or(call.location());
            let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Prefer method call with parentheses".to_string(),
            ));
        }

        if style == "omit_parentheses" && has_parens {
            // Don't flag if parent context makes it ambiguous
            // We can't easily check parent in check_node, so we'll use a heuristic:
            // Check if first arg is on same line as method call
            let call_loc = call.location();
            let (call_line, _) = source.offset_to_line_col(call_loc.start_offset());
            let first_arg_loc = first_arg.location();
            let (arg_line, _) = source.offset_to_line_col(first_arg_loc.start_offset());

            if call_line != arg_line {
                // Multi-line: create(\n  :user\n) — don't flag
                return;
            }

            // Check for hash value omission (Ruby 3.1+ `name:` shorthand)
            if has_value_omission_hash(&arg_list) {
                return;
            }

            let msg_loc = call.message_loc().unwrap_or(call.location());
            let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Prefer method call without parentheses".to_string(),
            ));
        }

    }
}

/// Check if any argument is a hash with value omission (Ruby 3.1+ `name:` syntax).
fn has_value_omission_hash(args: &[ruby_prism::Node<'_>]) -> bool {
    for arg in args {
        if let Some(hash) = arg.as_keyword_hash_node() {
            for elem in hash.elements().iter() {
                if let Some(pair) = elem.as_assoc_node() {
                    // In Prism, value omission is represented as an ImplicitNode
                    // or the value matching the key.
                    if pair.value().as_implicit_node().is_some() {
                        return true;
                    }
                }
                // Skip kwsplat nodes
            }
        }
        if let Some(hash) = arg.as_hash_node() {
            for elem in hash.elements().iter() {
                if let Some(pair) = elem.as_assoc_node() {
                    if pair.value().as_implicit_node().is_some() {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ConsistentParenthesesStyle,
        "cops/factorybot/consistent_parentheses_style"
    );
}
