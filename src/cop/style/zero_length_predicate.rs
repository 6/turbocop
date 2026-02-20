use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, INTEGER_NODE};

pub struct ZeroLengthPredicate;

impl ZeroLengthPredicate {
    /// Check if a call is `.length` or `.size`
    fn is_length_or_size(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            let name = call.name();
            let name_bytes = name.as_slice();
            if (name_bytes == b"length" || name_bytes == b"size")
                && call.arguments().is_none()
                && call.receiver().is_some()
            {
                return true;
            }
        }
        false
    }

    /// Get the integer value from a node
    fn int_value(node: &ruby_prism::Node<'_>) -> Option<i64> {
        if let Some(int_node) = node.as_integer_node() {
            // We need to extract the integer value from the source
            let src = int_node.location().as_slice();
            if let Ok(s) = std::str::from_utf8(src) {
                return s.parse::<i64>().ok();
            }
        }
        None
    }
}

impl Cop for ZeroLengthPredicate {
    fn name(&self) -> &'static str {
        "Style/ZeroLengthPredicate"
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

        // Pattern: x.length.zero? or x.size.zero?
        if method_bytes == b"zero?" && call.arguments().is_none() {
            if let Some(receiver) = call.receiver() {
                if Self::is_length_or_size(&receiver) {
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    let src = std::str::from_utf8(loc.as_slice()).unwrap_or("");
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Use `empty?` instead of `{}`.", src),
                    ));
                }
            }
        }

        // Pattern: x.length == 0, x.size == 0, 0 == x.length, x.length < 1, etc.
        if matches!(method_bytes, b"==" | b"!=" | b">" | b"<") {
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() == 1 {
                    if let Some(receiver) = call.receiver() {
                        // x.length == 0 or x.length < 1
                        if Self::is_length_or_size(&receiver) {
                            let arg_val = Self::int_value(&arg_list[0]);
                            let is_zero_check = match method_bytes {
                                b"==" => arg_val == Some(0),
                                b"<" => arg_val == Some(1),
                                b"!=" | b">" => arg_val == Some(0),
                                _ => false,
                            };
                            if is_zero_check {
                                let loc = node.location();
                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                let src = std::str::from_utf8(loc.as_slice()).unwrap_or("");
                                let msg = if method_bytes == b"!=" || method_bytes == b">" {
                                    format!("Use `!empty?` instead of `{}`.", src)
                                } else {
                                    format!("Use `empty?` instead of `{}`.", src)
                                };
                                diagnostics.push(self.diagnostic(source, line, column, msg));
                            }
                        }
                        // 0 == x.length, 1 > x.length
                        if let Some(recv_val) = Self::int_value(&receiver) {
                            if Self::is_length_or_size(&arg_list[0]) {
                                let is_zero_check = match method_bytes {
                                    b"==" => recv_val == 0,
                                    b">" => recv_val == 1,
                                    b"!=" | b"<" => recv_val == 0,
                                    _ => false,
                                };
                                if is_zero_check {
                                    let loc = node.location();
                                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                                    let src = std::str::from_utf8(loc.as_slice()).unwrap_or("");
                                    let msg = if method_bytes == b"!=" || method_bytes == b"<" {
                                        format!("Use `!empty?` instead of `{}`.", src)
                                    } else {
                                        format!("Use `empty?` instead of `{}`.", src)
                                    };
                                    diagnostics.push(self.diagnostic(source, line, column, msg));
                                }
                            }
                        }
                    }
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ZeroLengthPredicate, "cops/style/zero_length_predicate");
}
