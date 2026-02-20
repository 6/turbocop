use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_ARGUMENT_NODE, CALL_NODE};

pub struct ObjectThen;

impl Cop for ObjectThen {
    fn name(&self) -> &'static str {
        "Style/ObjectThen"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_ARGUMENT_NODE, CALL_NODE]
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
        let enforced_style = config.get_str("EnforcedStyle", "then");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name();
        let method_bytes = method_name.as_slice();

        // Check if this is yield_self or then
        if !matches!(method_bytes, b"yield_self" | b"then") {
            return;
        }

        // Must have a block or a block_pass argument
        let has_block = call.block().is_some();
        let has_block_pass = if let Some(args) = call.arguments() {
            args.arguments().iter().any(|a| a.as_block_argument_node().is_some())
        } else {
            false
        };

        if !has_block && !has_block_pass {
            return;
        }

        if enforced_style == "then" && method_bytes == b"yield_self" {
            let msg_loc = match call.message_loc() {
                Some(l) => l,
                None => return,
            };
            let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Prefer `then` over `yield_self`.".to_string(),
            ));
        } else if enforced_style == "yield_self" && method_bytes == b"then" {
            let msg_loc = match call.message_loc() {
                Some(l) => l,
                None => return,
            };
            let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Prefer `yield_self` over `then`.".to_string(),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ObjectThen, "cops/style/object_then");
}
