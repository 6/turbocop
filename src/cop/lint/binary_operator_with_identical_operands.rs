use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{AND_NODE, CALL_NODE, OR_NODE};

pub struct BinaryOperatorWithIdenticalOperands;

impl Cop for BinaryOperatorWithIdenticalOperands {
    fn name(&self) -> &'static str {
        "Lint/BinaryOperatorWithIdenticalOperands"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[AND_NODE, CALL_NODE, OR_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Handle `&&` and `||` (AndNode / OrNode)
        if let Some(and_node) = node.as_and_node() {
            let left_loc = and_node.left().location();
            let right_loc = and_node.right().location();
            let left_src = &source.as_bytes()[left_loc.start_offset()..left_loc.end_offset()];
            let right_src = &source.as_bytes()[right_loc.start_offset()..right_loc.end_offset()];
            if left_src == right_src {
                let loc = and_node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Binary operator `&&` has identical operands.".to_string(),
                )];
            }
            return Vec::new();
        }

        if let Some(or_node) = node.as_or_node() {
            let left_loc = or_node.left().location();
            let right_loc = or_node.right().location();
            let left_src = &source.as_bytes()[left_loc.start_offset()..left_loc.end_offset()];
            let right_src = &source.as_bytes()[right_loc.start_offset()..right_loc.end_offset()];
            if left_src == right_src {
                let loc = or_node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Binary operator `||` has identical operands.".to_string(),
                )];
            }
            return Vec::new();
        }

        // Handle binary send operators: ==, !=, ===, <=>, =~, >, >=, <, <=, |, ^, &
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method = call.name().as_slice();
        let is_binary_op = matches!(
            method,
            b"==" | b"!=" | b"===" | b"<=>" | b"=~" | b">" | b">=" | b"<" | b"<=" | b"|" | b"^" | b"&"
        );
        if !is_binary_op {
            return Vec::new();
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args = arguments.arguments();
        if args.len() != 1 {
            return Vec::new();
        }

        let first_arg = args.iter().next().unwrap();
        let recv_loc = receiver.location();
        let arg_loc = first_arg.location();
        let recv_src = &source.as_bytes()[recv_loc.start_offset()..recv_loc.end_offset()];
        let arg_src = &source.as_bytes()[arg_loc.start_offset()..arg_loc.end_offset()];

        if recv_src == arg_src {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let op_str = std::str::from_utf8(method).unwrap_or("?");
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!("Binary operator `{op_str}` has identical operands."),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BinaryOperatorWithIdenticalOperands, "cops/lint/binary_operator_with_identical_operands");
}
