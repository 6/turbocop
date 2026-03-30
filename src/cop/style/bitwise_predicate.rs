use crate::cop::node_type::{CALL_NODE, INTEGER_NODE, PARENTHESES_NODE, STATEMENTS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// RuboCop also flags `allbits?` comparisons where a parenthesized `&` expression
/// is compared with `==` to one of its own operands, including the reversed
/// operand order. nitrocop only handled the `positive?/zero?/0/1` forms, which
/// missed corpus cases like `(integer & constant_value) == constant_value` and
/// `(clauses.values & partial_clauses) == clauses.values`.
///
/// FN fix: RuboCop compares integer literals by parsed value, not source text.
/// nitrocop previously missed hex and value-equivalent forms like
/// `(v & 0x80) == 0x00` and `(dimensions >> 28 & 0x1) == 1` because `0x00`
/// was not parsed by `str::parse()` and `0x1` did not byte-match `1`.
/// Compare `IntegerNode` values directly so `nobits?` and `allbits?` handle
/// Ruby integer literal prefixes consistently.
pub struct BitwisePredicate;

fn method_name<'a>(call: &'a ruby_prism::CallNode<'a>) -> &'a str {
    std::str::from_utf8(call.name().as_slice()).unwrap_or("")
}

fn parenthesized_bit_operation<'a>(
    receiver: Option<ruby_prism::Node<'a>>,
) -> Option<ruby_prism::CallNode<'a>> {
    let paren = receiver?.as_parentheses_node()?;
    let body = paren.body()?.as_statements_node()?;
    let mut statements = body.body().iter();
    let statement = statements.next()?;

    if statements.next().is_some() {
        return None;
    }

    let bit_operation = statement.as_call_node()?;
    (method_name(&bit_operation) == "&").then_some(bit_operation)
}

fn single_argument<'a>(call: &ruby_prism::CallNode<'a>) -> Option<ruby_prism::Node<'a>> {
    let arguments = call.arguments()?;
    let mut args = arguments.arguments().iter();
    let argument = args.next()?;

    if args.next().is_some() {
        return None;
    }

    Some(argument)
}

fn integer_equals(node: &ruby_prism::Node<'_>, expected: u32) -> bool {
    let Some(int_node) = node.as_integer_node() else {
        return false;
    };
    let value = int_node.value();
    let (negative, digits) = value.to_u32_digits();

    !negative
        && digits.first().copied().unwrap_or(0) == expected
        && digits.iter().skip(1).all(|digit| *digit == 0)
}

fn node_source<'a>(node: &ruby_prism::Node<'a>) -> &'a str {
    std::str::from_utf8(node.location().as_slice()).unwrap_or("")
}

fn preferred_predicate(
    bit_operation: &ruby_prism::CallNode<'_>,
    predicate: &str,
) -> Option<String> {
    let lhs = bit_operation.receiver()?;
    let rhs = single_argument(bit_operation)?;
    Some(format!(
        "{}.{}({})",
        node_source(&lhs),
        predicate,
        node_source(&rhs)
    ))
}

fn same_source_or_integer_value(left: &ruby_prism::Node<'_>, right: &ruby_prism::Node<'_>) -> bool {
    left.location().as_slice() == right.location().as_slice()
        || match (left.as_integer_node(), right.as_integer_node()) {
            (Some(left_int), Some(right_int)) => {
                let left_value = left_int.value();
                let right_value = right_int.value();
                left_value.to_u32_digits() == right_value.to_u32_digits()
            }
            _ => false,
        }
}

fn preferred_allbits(
    call: &ruby_prism::CallNode<'_>,
    bit_operation: &ruby_prism::CallNode<'_>,
) -> Option<String> {
    let argument = single_argument(call)?;
    let lhs = bit_operation.receiver()?;
    let rhs = single_argument(bit_operation)?;

    if same_source_or_integer_value(&argument, &lhs) {
        Some(format!(
            "{}.allbits?({})",
            node_source(&rhs),
            node_source(&lhs)
        ))
    } else if same_source_or_integer_value(&argument, &rhs) {
        Some(format!(
            "{}.allbits?({})",
            node_source(&lhs),
            node_source(&rhs)
        ))
    } else {
        None
    }
}

impl Cop for BitwisePredicate {
    fn name(&self) -> &'static str {
        "Style/BitwisePredicate"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, INTEGER_NODE, PARENTHESES_NODE, STATEMENTS_NODE]
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

        let method_name = method_name(&call);

        // Pattern: (variable & flags).positive? => variable.anybits?(flags)
        if method_name == "positive?" || method_name == "zero?" {
            let predicate = if method_name == "positive?" {
                "anybits?"
            } else {
                "nobits?"
            };

            if let Some(bit_operation) = parenthesized_bit_operation(call.receiver()) {
                if let Some(preferred) = preferred_predicate(&bit_operation, predicate) {
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!(
                            "Replace with `{}` for comparison with bit flags.",
                            preferred
                        ),
                    ));
                }
            }
        }

        // Pattern: (variable & flags) > 0 / != 0 / == 0
        if matches!(method_name, ">" | "!=" | "==" | ">=") {
            if let Some(bit_operation) = parenthesized_bit_operation(call.receiver()) {
                if method_name == "==" {
                    if let Some(preferred) = preferred_allbits(&call, &bit_operation) {
                        let loc = node.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            format!(
                                "Replace with `{}` for comparison with bit flags.",
                                preferred
                            ),
                        ));
                        return;
                    }
                }

                if let Some(argument) = single_argument(&call) {
                    let is_zero = integer_equals(&argument, 0);
                    let is_one = integer_equals(&argument, 1);

                    if ((method_name == "!=" || method_name == ">") && is_zero)
                        || (method_name == ">=" && is_one)
                    {
                        if let Some(preferred) = preferred_predicate(&bit_operation, "anybits?") {
                            let loc = node.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                format!(
                                    "Replace with `{}` for comparison with bit flags.",
                                    preferred
                                ),
                            ));
                        }
                    }

                    if method_name == "==" && is_zero {
                        if let Some(preferred) = preferred_predicate(&bit_operation, "nobits?") {
                            let loc = node.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                format!(
                                    "Replace with `{}` for comparison with bit flags.",
                                    preferred
                                ),
                            ));
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
    crate::cop_fixture_tests!(BitwisePredicate, "cops/style/bitwise_predicate");
}
