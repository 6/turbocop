use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/OperatorMethodCall — flags redundant dot before binary operator methods.
///
/// Investigation (2026-03-15): 61 FPs, mostly from xiki repo patterns like `Tree.<<(result)`
/// and `Image.>> dest`. Root cause: RuboCop's `on_send` returns early when the receiver is
/// a constant (`node.receiver.const_type?`), because removing the dot before an operator
/// on a constant creates parsing ambiguity (e.g., `Tree << result` could be a heredoc).
/// Also excludes splat/kwsplat/forwarded args (`INVALID_SYNTAX_ARG_TYPES`), since removing
/// the dot would produce invalid syntax.
///
/// Fix: Added constant-receiver check and invalid-argument-type check to match RuboCop behavior.
///
/// Investigation (2026-03-15): 18 remaining FPs from parenthesized operator calls nested
/// inside other method calls, e.g. `expect(one.==(two))`. RuboCop's
/// `method_call_with_parenthesized_arg?` skips when the operator call is parenthesized AND
/// its parent is another send node. Without parent pointers in Prism, we detect this by
/// checking what follows the closing paren: `)` or `,` means the call is nested inside
/// another call's argument list.
///
/// Fix: Extended the post-closing-paren check to also skip when followed by `)` or `,`.
pub struct OperatorMethodCall;

const OPERATOR_METHODS: &[&[u8]] = &[
    b"+", b"-", b"*", b"/", b"%", b"**", b"==", b"!=", b"<", b">", b"<=", b">=", b"<=>", b"<<",
    b">>", b"|", b"&", b"^",
];

impl Cop for OperatorMethodCall {
    fn name(&self) -> &'static str {
        "Style/OperatorMethodCall"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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

        // Must be an operator method
        if !OPERATOR_METHODS.contains(&method_bytes) {
            return;
        }

        // Must have a receiver, and receiver must not be a constant
        // RuboCop skips const_type? receivers (e.g., `Tree.<<(result)`)
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        if receiver.as_constant_read_node().is_some() || receiver.as_constant_path_node().is_some()
        {
            return;
        }

        // Must have a dot call operator (redundant dot before operator)
        let call_op = match call.call_operator_loc() {
            Some(op) => op,
            None => return,
        };

        if call_op.as_slice() != b"." {
            return;
        }

        // Must have exactly one argument (binary operator)
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() != 1 {
                return;
            }
            // Skip splat, kwsplat, forwarded args — removing dot would be
            // invalid syntax (RuboCop's INVALID_SYNTAX_ARG_TYPES)
            let arg = &arg_list[0];
            if arg.as_splat_node().is_some() || arg.as_assoc_splat_node().is_some() {
                return;
            }
            // kwsplat may also appear inside a keyword_hash_node wrapper
            if let Some(kh) = arg.as_keyword_hash_node() {
                if kh
                    .elements()
                    .iter()
                    .any(|e| e.as_assoc_splat_node().is_some())
                {
                    return;
                }
            }
        } else {
            // Unary operator with dot is also wrong but less common
            // Only flag binary operators
            return;
        }

        // Skip `foo.-(bar).baz` pattern and `expect(foo.==(bar))` pattern:
        // RuboCop's `method_call_with_parenthesized_arg?` skips when:
        // 1. The operator call is parenthesized AND chained (used as receiver), OR
        // 2. The operator call is parenthesized AND nested inside another method call's arguments
        // Without parent pointers, we detect these by checking what follows the closing paren:
        // - `.` or `&.` → chaining (case 1)
        // - `)` or `,` → nested inside another call's argument list (case 2)
        if call.opening_loc().is_some() {
            if let Some(close) = call.closing_loc() {
                let end_off = close.start_offset() + close.as_slice().len();
                let src = source.as_bytes();
                // Skip whitespace after closing paren
                let mut pos = end_off;
                while pos < src.len()
                    && (src[pos] == b' '
                        || src[pos] == b'\t'
                        || src[pos] == b'\n'
                        || src[pos] == b'\r')
                {
                    pos += 1;
                }
                if pos < src.len() {
                    let ch = src[pos];
                    // Dot/safe-nav → chaining: `foo.-(bar).baz`
                    if ch == b'.' || (pos + 1 < src.len() && ch == b'&' && src[pos + 1] == b'.') {
                        return;
                    }
                    // Closing paren or comma → nested in another call: `expect(foo.==(bar))`
                    if ch == b')' || ch == b',' {
                        return;
                    }
                }
            }
        }

        let (line, column) = source.offset_to_line_col(call_op.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Redundant dot detected.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OperatorMethodCall, "cops/style/operator_method_call");
}
