use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct StringConcatenation;

impl StringConcatenation {
    fn is_string_literal(node: &ruby_prism::Node<'_>) -> bool {
        // Only match plain StringNode (str_type? in RuboCop), NOT InterpolatedStringNode (dstr).
        // RuboCop's node matcher uses str_type? which excludes dstr, so `foo + "#{bar}"`
        // is not flagged when neither side is a plain string literal.
        node.as_string_node().is_some()
    }

    /// Check if the + call spans multiple lines (line-end concatenation)
    fn is_multiline(source: &SourceFile, node: &ruby_prism::CallNode<'_>) -> bool {
        if let Some(receiver) = node.receiver() {
            let (recv_line, _) = source.offset_to_line_col(receiver.location().start_offset());
            if let Some(args) = node.arguments() {
                let args_list: Vec<_> = args.arguments().iter().collect();
                if !args_list.is_empty() {
                    let (arg_line, _) = source.offset_to_line_col(args_list[0].location().start_offset());
                    return recv_line != arg_line;
                }
            }
        }
        false
    }

    /// Walk up the tree to find the topmost + node in a chain
    /// Since we can't walk up from a node, we just report on each + individually
    fn is_string_concat(call: &ruby_prism::CallNode<'_>) -> bool {
        if call.name().as_slice() != b"+" {
            return false;
        }
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() != 1 {
                return false;
            }
            if let Some(receiver) = call.receiver() {
                // Either side must be a string literal
                return Self::is_string_literal(&receiver) || Self::is_string_literal(&arg_list[0]);
            }
        }
        false
    }
}

impl Cop for StringConcatenation {
    fn name(&self) -> &'static str {
        "Style/StringConcatenation"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if !Self::is_string_concat(&call) {
            return Vec::new();
        }

        let mode = config.get_str("Mode", "aggressive");

        if mode == "conservative" {
            // In conservative mode, only flag if the receiver (LHS) is a string literal
            if let Some(receiver) = call.receiver() {
                if !Self::is_string_literal(&receiver) {
                    return Vec::new();
                }
            }
        }

        // Skip line-end concatenation (handled by Style/LineEndConcatenation)
        if Self::is_multiline(source, &call) {
            if let Some(receiver) = call.receiver() {
                if let Some(args) = call.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if !arg_list.is_empty() && Self::is_string_literal(&receiver) && Self::is_string_literal(&arg_list[0]) {
                        return Vec::new();
                    }
                }
            }
        }

        // Skip if this node's receiver is already a + call with string
        // (avoid duplicate reports for chains; only report the topmost)
        if let Some(receiver) = call.receiver() {
            if let Some(recv_call) = receiver.as_call_node() {
                if Self::is_string_concat(&recv_call) {
                    return Vec::new();
                }
            }
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Prefer string interpolation to string concatenation.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(StringConcatenation, "cops/style/string_concatenation");
}
