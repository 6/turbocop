use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, NIL_NODE};

pub struct RedundantArrayFlatten;

impl Cop for RedundantArrayFlatten {
    fn name(&self) -> &'static str {
        "Style/RedundantArrayFlatten"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, NIL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Looking for: x.flatten.join or x.flatten.join(nil)
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Outer call must be `join`
        if call.name().as_slice() != b"join" {
            return;
        }

        // join must have 0 or 1 args, and if 1, it must be nil
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() > 1 {
                return;
            }
            if arg_list.len() == 1 && arg_list[0].as_nil_node().is_none() {
                // Has a non-nil argument (separator) - then flatten is not redundant
                return;
            }
        }

        // Receiver must be a call to `flatten` with a receiver
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if recv_call.name().as_slice() != b"flatten" {
            return;
        }

        // flatten must have a receiver (not bare `flatten`)
        if recv_call.receiver().is_none() {
            return;
        }

        // flatten can have 0 or 1 args (depth), but not more
        if let Some(args) = recv_call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() > 1 {
                return;
            }
        }

        let msg_loc = recv_call.message_loc().unwrap_or_else(|| recv_call.location());
        // Include the dot before flatten
        let dot_start = if let Some(op) = recv_call.call_operator_loc() {
            op.start_offset()
        } else {
            msg_loc.start_offset()
        };
        let (line, column) = source.offset_to_line_col(dot_start);
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Remove the redundant `flatten`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantArrayFlatten, "cops/style/redundant_array_flatten");
}
