use crate::cop::node_type::{CALL_NODE, INTEGER_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct UnpackFirst;

impl UnpackFirst {
    fn int_value(node: &ruby_prism::Node<'_>) -> Option<i64> {
        if let Some(int_node) = node.as_integer_node() {
            let src = int_node.location().as_slice();
            if let Ok(s) = std::str::from_utf8(src) {
                return s.parse::<i64>().ok();
            }
        }
        None
    }
}

impl Cop for UnpackFirst {
    fn name(&self) -> &'static str {
        "Style/UnpackFirst"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, INTEGER_NODE]
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

        let method_name = call.name();
        let method_bytes = method_name.as_slice();

        // Must be .first, .[], .slice, or .at
        if !matches!(method_bytes, b"first" | b"[]" | b"slice" | b"at") {
            return;
        }

        // For .first, no arguments required
        // For .[], .slice, .at — argument must be 0
        if matches!(method_bytes, b"[]" | b"slice" | b"at") {
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() != 1 || Self::int_value(&arg_list[0]) != Some(0) {
                    return;
                }
            } else {
                return;
            }
        } else if method_bytes == b"first" && call.arguments().is_some() {
            return;
        }

        // Receiver must be a call to .unpack with one argument
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        if let Some(unpack_call) = receiver.as_call_node() {
            if unpack_call.name().as_slice() == b"unpack" {
                if let Some(args) = unpack_call.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if arg_list.len() == 1 {
                        let format_src =
                            std::str::from_utf8(arg_list[0].location().as_slice()).unwrap_or("...");
                        let loc = node.location();
                        let current = std::str::from_utf8(loc.as_slice()).unwrap_or("");
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            format!("Use `unpack1({})` instead of `{}`.", format_src, current),
                        ));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnpackFirst, "cops/style/unpack_first");
}
