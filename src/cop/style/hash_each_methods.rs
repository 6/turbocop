use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

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
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_bytes = call.name().as_slice();

        if method_bytes != b"each" {
            return Vec::new();
        }

        let _allowed_receivers = config.get_string_array("AllowedReceivers");

        // Check receiver is foo.keys or foo.values
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let recv_method = recv_call.name().as_slice();
        if recv_method != b"keys" && recv_method != b"values" {
            return Vec::new();
        }

        // The keys/values call must have a receiver (not bare `keys.each`)
        if recv_call.receiver().is_none() {
            return Vec::new();
        }

        // Must have no arguments to keys/values
        if recv_call.arguments().is_some() {
            return Vec::new();
        }

        let is_keys = recv_method == b"keys";
        let replacement = if is_keys { "each_key" } else { "each_value" };
        let original = if is_keys { "keys.each" } else { "values.each" };

        // Check safe navigation (&. vs regular .)
        let has_safe_nav = call.call_operator_loc().is_some_and(|op| op.as_slice() == b"&.");
        let recv_has_safe_nav = recv_call.call_operator_loc().is_some_and(|op| op.as_slice() == b"&.");

        let display_original = if has_safe_nav || recv_has_safe_nav {
            if is_keys { "keys&.each" } else { "values&.each" }
        } else {
            original
        };

        let msg_loc = recv_call.message_loc().unwrap_or_else(|| recv_call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());

        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Use `{}` instead of `{}`.", replacement, display_original),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashEachMethods, "cops/style/hash_each_methods");
}
