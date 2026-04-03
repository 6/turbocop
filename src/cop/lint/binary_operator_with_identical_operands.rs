use crate::cop::shared::node_type::{AND_NODE, CALL_NODE, OR_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=29, FN=9.
///
/// FP=29: nitrocop was checking `&`, but RuboCop intentionally leaves that
/// operator to other cops. The main FP bucket was spec code such as
/// `false & false` and `mask & mask`.
///
/// FN=9: raw source comparison missed semantically identical literals written
/// with different surface syntax, including `:ruby == :"ruby"` and
/// `-0.0 <=> 0.0`. RuboCop compares operand nodes semantically, so nitrocop
/// needs a small amount of literal normalization instead of byte-for-byte text.
///
/// ## Corpus investigation update (2026-03-15)
///
/// Corpus oracle reported the remaining FN=1 on malformed operator sends such
/// as `1.<(1, 2)`. RuboCop still compares the receiver against the first
/// argument for these nodes, even when extra arguments make the call invalid.
///
/// ## Corpus investigation update (2026-03-27)
///
/// Corpus oracle reported FP=2 and FN=2.
///
/// FP=2: Prism parses heredocs as `StringNode`, while RuboCop's parser treats
/// these forms as a distinct node shape from regular quoted strings in this
/// context. Comparing by unescaped string contents alone caused false positives
/// for mixed regular-string vs heredoc comparisons such as
/// `"123\n456\n" == <<-TEXT`.
///
/// FN=2: `||` branches with semantically identical call trees were missed when
/// literal syntax differed inside the branch (e.g. `"\0REQ"` vs `"\000REQ"` or
/// `:_implicitBlockYield` vs `:"_implicitBlockYield"`). RuboCop compares full
/// AST operands, so this cop now recursively compares call receivers/arguments
/// with existing literal normalization.
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
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Handle `&&` and `||` (AndNode / OrNode)
        if let Some(and_node) = node.as_and_node() {
            if operands_match(source, &and_node.left(), &and_node.right()) {
                let loc = and_node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Binary operator `&&` has identical operands.".to_string(),
                ));
            }
            return;
        }

        if let Some(or_node) = node.as_or_node() {
            if operands_match(source, &or_node.left(), &or_node.right()) {
                let loc = or_node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Binary operator `||` has identical operands.".to_string(),
                ));
            }
            return;
        }

        // Handle binary send operators: ==, !=, ===, <=>, =~, >, >=, <, <=, |, ^, &
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method = call.name().as_slice();
        let is_binary_op = matches!(
            method,
            b"==" | b"!=" | b"===" | b"<=>" | b"=~" | b">" | b">=" | b"<" | b"<=" | b"|" | b"^"
        );
        if !is_binary_op {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args = arguments.arguments();
        if args.is_empty() {
            return;
        }

        let first_arg = args.iter().next().unwrap();
        if operands_match(source, &receiver, &first_arg) {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let op_str = std::str::from_utf8(method).unwrap_or("?");
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Binary operator `{op_str}` has identical operands."),
            ));
        }
    }
}

fn operands_match(
    source: &SourceFile,
    left: &ruby_prism::Node<'_>,
    right: &ruby_prism::Node<'_>,
) -> bool {
    let left_loc = left.location();
    let right_loc = right.location();
    let left_src = &source.as_bytes()[left_loc.start_offset()..left_loc.end_offset()];
    let right_src = &source.as_bytes()[right_loc.start_offset()..right_loc.end_offset()];

    if left_src == right_src {
        return true;
    }

    if let (Some(left_call), Some(right_call)) = (left.as_call_node(), right.as_call_node()) {
        return call_nodes_match(source, &left_call, &right_call);
    }

    if let (Some(left_sym), Some(right_sym)) = (left.as_symbol_node(), right.as_symbol_node()) {
        return left_sym.unescaped() == right_sym.unescaped();
    }

    if let (Some(left_str), Some(right_str)) = (left.as_string_node(), right.as_string_node()) {
        return string_nodes_match(source, &left_str, &right_str);
    }

    if let (Some(left_float), Some(right_float)) = (left.as_float_node(), right.as_float_node()) {
        return left_float.value() == right_float.value();
    }

    if let (Some(left_int), Some(right_int)) = (left.as_integer_node(), right.as_integer_node()) {
        return left_int.value().to_u32_digits() == right_int.value().to_u32_digits();
    }

    false
}

fn call_nodes_match(
    source: &SourceFile,
    left: &ruby_prism::CallNode<'_>,
    right: &ruby_prism::CallNode<'_>,
) -> bool {
    if left.name().as_slice() != right.name().as_slice() {
        return false;
    }

    if left.block().is_some() || right.block().is_some() {
        return false;
    }

    if !option_nodes_match(source, left.receiver().as_ref(), right.receiver().as_ref()) {
        return false;
    }

    match (left.arguments(), right.arguments()) {
        (Some(left_args), Some(right_args)) => {
            let left_args = left_args.arguments();
            let right_args = right_args.arguments();
            if left_args.len() != right_args.len() {
                return false;
            }

            left_args
                .iter()
                .zip(right_args.iter())
                .all(|(left_arg, right_arg)| operands_match(source, &left_arg, &right_arg))
        }
        (Some(left_args), None) => left_args.arguments().is_empty(),
        (None, Some(right_args)) => right_args.arguments().is_empty(),
        (None, None) => true,
    }
}

fn option_nodes_match(
    source: &SourceFile,
    left: Option<&ruby_prism::Node<'_>>,
    right: Option<&ruby_prism::Node<'_>>,
) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => operands_match(source, left, right),
        (None, None) => true,
        _ => false,
    }
}

fn string_nodes_match(
    source: &SourceFile,
    left: &ruby_prism::StringNode<'_>,
    right: &ruby_prism::StringNode<'_>,
) -> bool {
    let left_is_heredoc = string_node_is_heredoc(source, left);
    let right_is_heredoc = string_node_is_heredoc(source, right);

    if left_is_heredoc != right_is_heredoc {
        return false;
    }

    left.unescaped() == right.unescaped()
}

fn string_node_is_heredoc(source: &SourceFile, node: &ruby_prism::StringNode<'_>) -> bool {
    let Some(opening) = node.opening_loc() else {
        return false;
    };
    let opening_src = &source.as_bytes()[opening.start_offset()..opening.end_offset()];
    opening_src.starts_with(b"<<")
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        BinaryOperatorWithIdenticalOperands,
        "cops/lint/binary_operator_with_identical_operands"
    );
}
