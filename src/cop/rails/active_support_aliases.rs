use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ARRAY_NODE, CALL_NODE, INTERPOLATED_STRING_NODE, STRING_NODE};

pub struct ActiveSupportAliases;

/// Check if the receiver is a string literal node.
fn is_string_receiver(receiver: &ruby_prism::Node<'_>) -> bool {
    receiver.as_string_node().is_some()
        || receiver.as_interpolated_string_node().is_some()
}

/// Check if the receiver is an array literal node.
fn is_array_receiver(receiver: &ruby_prism::Node<'_>) -> bool {
    receiver.as_array_node().is_some()
}

impl Cop for ActiveSupportAliases {
    fn name(&self) -> &'static str {
        "Rails/ActiveSupportAliases"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, CALL_NODE, INTERPOLATED_STRING_NODE, STRING_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let name = call.name().as_slice();
        let replacement = if (name == b"starts_with?" || name == b"ends_with?")
            && is_string_receiver(&receiver)
        {
            if name == b"starts_with?" {
                "start_with?"
            } else {
                "end_with?"
            }
        } else if (name == b"append" || name == b"prepend") && is_array_receiver(&receiver) {
            if name == b"append" {
                "<<"
            } else {
                "unshift"
            }
        } else {
            return;
        };

        let original = std::str::from_utf8(name).unwrap_or("?");

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{replacement}` instead of `{original}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ActiveSupportAliases, "cops/rails/active_support_aliases");
}
