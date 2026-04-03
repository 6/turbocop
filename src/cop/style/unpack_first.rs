use crate::cop::shared::node_type::{CALL_NODE, INTEGER_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Investigation: FP=8 FN=6 were caused by a line location bug on multi-line expressions.
/// The cop was reporting at `node.location().start_offset()` which spans the entire call chain
/// from receiver to `.first`/`[0]`. For multi-line expressions (e.g. `OpenSSL::PKCS5.pbkdf2_hmac(...).unpack('H*')[0]`),
/// this put the offense on the wrong line (chain start instead of `.unpack` line).
/// Fix: report from `unpack_call.message_loc()` to the outer node's end, matching RuboCop's behavior.
/// Message also changed to exclude receiver prefix (e.g. `unpack('h*').first` not `'foo'.unpack('h*').first`).
///
/// Investigation (2): FP=2 from bare `unpack("H*")[0]` without explicit receiver (implicit self).
/// RuboCop's NodePattern `(call $(call (...) :unpack $(...)) :first)` requires `(...)` as the
/// receiver of `unpack`, which does NOT match `nil` (implicit self in Parser AST). So RuboCop
/// only flags `obj.unpack("H*")[0]`, not bare `unpack("H*")[0]`. Fixed by checking
/// `unpack_call.receiver().is_some()`.
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
            if unpack_call.name().as_slice() == b"unpack" && unpack_call.receiver().is_some() {
                if let Some(args) = unpack_call.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if arg_list.len() == 1 {
                        let format_src =
                            std::str::from_utf8(arg_list[0].location().as_slice()).unwrap_or("...");
                        // Report from the unpack method name to the end of the outer call,
                        // matching RuboCop's location behavior for multi-line chains.
                        let msg_loc = unpack_call.message_loc().unwrap();
                        let outer_end =
                            node.location().start_offset() + node.location().as_slice().len();
                        let current = source.byte_slice(msg_loc.start_offset(), outer_end, "");
                        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
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
