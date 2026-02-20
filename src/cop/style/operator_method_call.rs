use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct OperatorMethodCall;

const OPERATOR_METHODS: &[&[u8]] = &[
    b"+", b"-", b"*", b"/", b"%", b"**",
    b"==", b"!=", b"<", b">", b"<=", b">=", b"<=>",
    b"<<", b">>", b"|", b"&", b"^",
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
        if !OPERATOR_METHODS.iter().any(|&m| m == method_bytes) {
            return;
        }

        // Must have a receiver
        if call.receiver().is_none() {
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
        } else {
            // Unary operator with dot is also wrong but less common
            // Only flag binary operators
            return;
        }

        // Skip `foo.-(bar).baz` pattern: if the call is parenthesized and
        // the result is chained (used as receiver of another call), converting
        // would change semantics. RuboCop's `method_call_with_parenthesized_arg?`.
        if call.opening_loc().is_some() {
            // The call has parentheses; check if it's chained by looking at
            // source after the closing paren — if there's a dot/method, skip.
            if let Some(close) = call.closing_loc() {
                let end_off = close.start_offset() + close.as_slice().len();
                let src = source.as_bytes();
                // Check if there's a dot immediately after the closing paren
                // (possibly with whitespace/newlines)
                let mut pos = end_off;
                while pos < src.len() && (src[pos] == b' ' || src[pos] == b'\t' || src[pos] == b'\n' || src[pos] == b'\r') {
                    pos += 1;
                }
                if pos < src.len() && (src[pos] == b'.' || (pos + 1 < src.len() && src[pos] == b'&' && src[pos + 1] == b'.')) {
                    return;
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
