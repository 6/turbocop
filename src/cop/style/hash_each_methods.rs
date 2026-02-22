use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct HashEachMethods;

impl Cop for HashEachMethods {
    fn name(&self) -> &'static str {
        "Style/HashEachMethods"
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

        let method_bytes = call.name().as_slice();

        if method_bytes != b"each" {
            return;
        }

        let _allowed_receivers = config.get_string_array("AllowedReceivers");

        // Must have a receiver
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Pattern 1: hash.keys.each / hash.values.each
        if let Some(recv_call) = receiver.as_call_node() {
            let recv_method = recv_call.name().as_slice();
            if (recv_method == b"keys" || recv_method == b"values")
                && recv_call.receiver().is_some()
                && recv_call.arguments().is_none()
            {
                let is_keys = recv_method == b"keys";
                let replacement = if is_keys { "each_key" } else { "each_value" };
                let original = if is_keys { "keys.each" } else { "values.each" };

                let has_safe_nav = call
                    .call_operator_loc()
                    .is_some_and(|op| op.as_slice() == b"&.");
                let recv_has_safe_nav = recv_call
                    .call_operator_loc()
                    .is_some_and(|op| op.as_slice() == b"&.");

                let display_original = if has_safe_nav || recv_has_safe_nav {
                    if is_keys {
                        "keys&.each"
                    } else {
                        "values&.each"
                    }
                } else {
                    original
                };

                let msg_loc = recv_call
                    .message_loc()
                    .unwrap_or_else(|| recv_call.location());
                let (line, column) = source.offset_to_line_col(msg_loc.start_offset());

                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `{}` instead of `{}`.", replacement, display_original),
                ));
                return;
            }
        }

        // Pattern 2: hash.each { |k, _unused_v| ... } — unused block arg
        self.check_each_block(source, &call, config, diagnostics);
    }
}

impl HashEachMethods {
    /// Check `.each { |k, v| ... }` blocks where one argument is unused.
    /// An argument is considered unused if it starts with `_`.
    fn check_each_block(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if call.name().as_slice() != b"each" {
            return;
        }

        // Must have a receiver
        if call.receiver().is_none() {
            return;
        }

        // Receiver must not be a hash/array literal
        if let Some(recv) = call.receiver() {
            if recv.as_array_node().is_some() {
                return;
            }
        }

        // Must NOT be `keys.each` or `values.each` (handled above)
        if let Some(recv) = call.receiver() {
            if let Some(rc) = recv.as_call_node() {
                let name = rc.name().as_slice();
                if name == b"keys" || name == b"values" {
                    return;
                }
            }
        }

        // Must have a block
        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Block must have exactly 2 parameters
        let params = match block_node.parameters() {
            Some(p) => p,
            None => return,
        };
        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return,
        };
        let params_node = match block_params.parameters() {
            Some(p) => p,
            None => return,
        };
        let requireds: Vec<_> = params_node.requireds().iter().collect();
        if requireds.len() != 2 {
            return;
        }

        let key_param = match requireds[0].as_required_parameter_node() {
            Some(p) => p,
            None => return,
        };
        let value_param = match requireds[1].as_required_parameter_node() {
            Some(p) => p,
            None => return,
        };

        let key_name = key_param.name().as_slice();
        let value_name = value_param.name().as_slice();
        let key_unused = key_name.starts_with(b"_");
        let value_unused = value_name.starts_with(b"_");

        // Both unused — skip (RuboCop skips too)
        if key_unused && value_unused {
            return;
        }
        // Neither unused — skip
        if !key_unused && !value_unused {
            return;
        }

        let bytes = source.as_bytes();
        let unused_code = if value_unused {
            std::str::from_utf8(value_name).unwrap_or("_")
        } else {
            std::str::from_utf8(key_name).unwrap_or("_")
        };

        let replacement = if value_unused {
            "each_key"
        } else {
            "each_value"
        };

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        let _ = config.get_string_array("AllowedReceivers");

        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{replacement}` instead of `each` and remove the unused `{unused_code}` block argument."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashEachMethods, "cops/style/hash_each_methods");
}
