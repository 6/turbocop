use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ClassCheck;

impl Cop for ClassCheck {
    fn name(&self) -> &'static str {
        "Style/ClassCheck"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "is_a?");

        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call_node.name();
        let method_bytes = method_name.as_slice();

        // Must be is_a? or kind_of?
        if method_bytes != b"is_a?" && method_bytes != b"kind_of?" {
            return Vec::new();
        }

        // Must have a receiver
        if call_node.receiver().is_none() {
            return Vec::new();
        }

        // Check against enforced style
        let (prefer, current) = if enforced_style == "is_a?" {
            ("is_a?", "kind_of?")
        } else {
            ("kind_of?", "is_a?")
        };

        // Only flag the non-preferred style
        if method_bytes != current.as_bytes() {
            return Vec::new();
        }

        let msg_loc = call_node.message_loc().unwrap_or_else(|| call_node.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Prefer `Object#{}` over `Object#{}`.", prefer, current),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ClassCheck, "cops/style/class_check");
}
