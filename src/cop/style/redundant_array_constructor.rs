use crate::cop::node_type::{ARRAY_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantArrayConstructor;

impl Cop for RedundantArrayConstructor {
    fn name(&self) -> &'static str {
        "Style/RedundantArrayConstructor"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ARRAY_NODE,
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
        ]
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

        let method_bytes = call.name().as_slice();

        // Check for Array.new([...]) or Array[...] or Array([...])
        let receiver = match call.receiver() {
            Some(r) => r,
            None => {
                // Check for Kernel method: Array([...])
                if method_bytes == b"Array" {
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if arg_list.len() == 1 && arg_list[0].as_array_node().is_some() {
                            let loc = call.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                "Remove the redundant `Array` constructor.".to_string(),
                            ));
                        }
                    }
                }
                return;
            }
        };

        let is_array = if let Some(cr) = receiver.as_constant_read_node() {
            cr.name().as_slice() == b"Array"
        } else if let Some(cp) = receiver.as_constant_path_node() {
            cp.parent().is_none() && cp.name().map(|n| n.as_slice() == b"Array").unwrap_or(false)
        } else {
            false
        };

        if !is_array {
            return;
        }

        if method_bytes == b"new" {
            // Array.new([...]) - must have exactly one array literal argument
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() == 1 && arg_list[0].as_array_node().is_some() {
                    let _msg_loc = call.message_loc().unwrap_or_else(|| call.location());
                    let recv_start = receiver.location().start_offset();
                    let (line, column) = source.offset_to_line_col(recv_start);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Remove the redundant `Array` constructor.".to_string(),
                    ));
                }
            }
        } else if method_bytes == b"[]" {
            // Array[...] - any usage is redundant
            let recv_start = receiver.location().start_offset();
            let (line, column) = source.offset_to_line_col(recv_start);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Remove the redundant `Array` constructor.".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantArrayConstructor,
        "cops/style/redundant_array_constructor"
    );
}
