use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct ModuleMemberExistenceCheck;

/// Maps array-returning methods to their predicate equivalents
const METHOD_MAPPINGS: &[(&[u8], &str)] = &[
    (b"instance_methods", "method_defined?"),
    (b"public_instance_methods", "public_method_defined?"),
    (b"private_instance_methods", "private_method_defined?"),
    (b"protected_instance_methods", "protected_method_defined?"),
    (b"constants", "const_defined?"),
];

impl Cop for ModuleMemberExistenceCheck {
    fn name(&self) -> &'static str {
        "Style/ModuleMemberExistenceCheck"
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
        let allowed_methods = config.get_string_array("AllowedMethods");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must be `include?` or `member?`
        let outer_method = call.name();
        let outer_bytes = outer_method.as_slice();
        if outer_bytes != b"include?" && outer_bytes != b"member?" {
            return Vec::new();
        }

        // Must have an argument
        if call.arguments().is_none() {
            return Vec::new();
        }

        // Receiver must be a call to one of the array-returning methods
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let recv_method = recv_call.name();
        let recv_bytes = recv_method.as_slice();

        let predicate = match METHOD_MAPPINGS.iter().find(|(m, _)| *m == recv_bytes) {
            Some((_, p)) => *p,
            None => return Vec::new(),
        };

        // Check AllowedMethods
        if let Some(ref allowed) = allowed_methods {
            let recv_str = std::str::from_utf8(recv_bytes).unwrap_or("");
            if allowed.iter().any(|m| m == recv_str) {
                return Vec::new();
            }
        }

        let msg_loc = recv_call.message_loc().unwrap_or_else(|| recv_call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Use `{predicate}` instead."),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ModuleMemberExistenceCheck, "cops/style/module_member_existence_check");
}
