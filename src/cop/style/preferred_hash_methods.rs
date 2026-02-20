use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct PreferredHashMethods;

impl Cop for PreferredHashMethods {
    fn name(&self) -> &'static str {
        "Style/PreferredHashMethods"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must have exactly one argument
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() != 1 {
                return;
            }
        } else {
            return;
        }

        // Must have a receiver
        if call.receiver().is_none() {
            return;
        }

        let method_name = call.name();
        let method_bytes = method_name.as_slice();
        let enforced_style = config.get_str("EnforcedStyle", "short");

        if enforced_style == "short" {
            // Flag has_key? and has_value?
            if method_bytes == b"has_key?" || method_bytes == b"has_value?" {
                let prefer = if method_bytes == b"has_key?" { "key?" } else { "value?" };
                let current = std::str::from_utf8(method_bytes).unwrap_or("");
                let msg_loc = call.message_loc().unwrap();
                let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `Hash#{}` instead of `Hash#{}`.", prefer, current),
                ));
            }
        } else if enforced_style == "verbose" {
            // Flag key? and value?
            if method_bytes == b"key?" || method_bytes == b"value?" {
                let prefer = if method_bytes == b"key?" { "has_key?" } else { "has_value?" };
                let current = std::str::from_utf8(method_bytes).unwrap_or("");
                let msg_loc = call.message_loc().unwrap();
                let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `Hash#{}` instead of `Hash#{}`.", prefer, current),
                ));
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(PreferredHashMethods, "cops/style/preferred_hash_methods");
}
