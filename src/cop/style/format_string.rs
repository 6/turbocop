use crate::cop::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTERPOLATED_STRING_NODE, STRING_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FormatString;

impl Cop for FormatString {
    fn name(&self) -> &'static str {
        "Style/FormatString"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            INTERPOLATED_STRING_NODE,
            STRING_NODE,
        ]
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
        let style = config.get_str("EnforcedStyle", "format");

        match method_bytes {
            b"%" => {
                // String#% - only flag when style prefers format or sprintf
                if style == "percent" {
                    return;
                }
                // Must have a non-nil receiver
                let receiver = match call.receiver() {
                    Some(r) => r,
                    None => return,
                };

                let is_string_receiver = receiver.as_string_node().is_some()
                    || receiver.as_interpolated_string_node().is_some();

                if !is_string_receiver {
                    // For non-string receivers, only flag when RHS is an array or hash literal
                    // RuboCop pattern: (send !nil? $:% {array hash})
                    let has_array_or_hash_arg = call.arguments().is_some_and(|args| {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        arg_list.len() == 1
                            && (arg_list[0].as_array_node().is_some()
                                || arg_list[0].as_hash_node().is_some()
                                || arg_list[0].as_keyword_hash_node().is_some())
                    });
                    if !has_array_or_hash_arg {
                        return;
                    }
                }

                // RuboCop points at the % operator (node.loc.selector), not the whole expression
                let loc = call.message_loc().unwrap_or_else(|| call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let preferred = if style == "format" {
                    "format"
                } else {
                    "sprintf"
                };
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Favor `{}` over `String#%`.", preferred),
                ));
            }
            b"format" => {
                if style == "format" {
                    return;
                }
                // Only flag top-level or Kernel.format / ::Kernel.format
                if let Some(recv) = call.receiver() {
                    if !is_kernel_constant(&recv) {
                        return;
                    }
                }
                // RuboCop requires at least 2 arguments: (send nil? :format _ _ ...)
                let arg_count = call
                    .arguments()
                    .map(|a| a.arguments().iter().count())
                    .unwrap_or(0);
                if arg_count < 2 {
                    return;
                }

                // RuboCop points at the method name (node.loc.selector), not the whole expression
                let loc = call.message_loc().unwrap_or_else(|| call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let preferred = if style == "sprintf" {
                    "sprintf"
                } else {
                    "String#%"
                };
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Favor `{}` over `format`.", preferred),
                ));
            }
            b"sprintf" => {
                if style == "sprintf" {
                    return;
                }
                // Only flag top-level or Kernel.sprintf / ::Kernel.sprintf
                if let Some(recv) = call.receiver() {
                    if !is_kernel_constant(&recv) {
                        return;
                    }
                }
                // RuboCop requires at least 2 arguments: (send nil? :sprintf _ _ ...)
                let arg_count = call
                    .arguments()
                    .map(|a| a.arguments().iter().count())
                    .unwrap_or(0);
                if arg_count < 2 {
                    return;
                }

                // RuboCop points at the method name (node.loc.selector), not the whole expression
                let loc = call.message_loc().unwrap_or_else(|| call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let preferred = if style == "format" {
                    "format"
                } else {
                    "String#%"
                };
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Favor `{}` over `sprintf`.", preferred),
                ));
            }
            _ => {}
        }
    }
}

/// Check if a node is the `Kernel` constant (simple or qualified via constant_path_node).
fn is_kernel_constant(node: &ruby_prism::Node<'_>) -> bool {
    if node
        .as_constant_read_node()
        .is_some_and(|c| c.name().as_slice() == b"Kernel")
    {
        return true;
    }
    if let Some(cp) = node.as_constant_path_node() {
        if cp.parent().is_none() && cp.name().is_some_and(|n| n.as_slice() == b"Kernel") {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FormatString, "cops/style/format_string");
}
