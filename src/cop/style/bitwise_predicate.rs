use crate::cop::node_type::{CALL_NODE, INTEGER_NODE, PARENTHESES_NODE, STATEMENTS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// RuboCop also flags `allbits?` comparisons where a parenthesized `&` expression
/// is compared with `==` to one of its own operands, including the reversed
/// operand order. nitrocop only handled the `positive?/zero?/0/1` forms, which
/// missed corpus cases like `(integer & constant_value) == constant_value` and
/// `(clauses.values & partial_clauses) == clauses.values`.
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

fn integer_value(node: &ruby_prism::Node<'_>) -> Option<i64> {
    let int_node = node.as_integer_node()?;
    let src = std::str::from_utf8(int_node.location().as_slice()).ok()?;
    src.parse::<i64>().ok()
}

fn node_source<'a>(node: &ruby_prism::Node<'a>) -> &'a str {
    std::str::from_utf8(node.location().as_slice()).unwrap_or("")
}

fn preferred_allbits(
    call: &ruby_prism::CallNode<'_>,
    bit_operation: &ruby_prism::CallNode<'_>,
) -> Option<String> {
    let argument = single_argument(call)?;
    let lhs = bit_operation.receiver()?;
    let rhs = single_argument(bit_operation)?;

    if argument.location().as_slice() == lhs.location().as_slice() {
        Some(format!(
            "{}.allbits?({})",
            node_source(&rhs),
            node_source(&lhs)
        ))
    } else if argument.location().as_slice() == rhs.location().as_slice() {
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
        if (method_name == "positive?" || method_name == "zero?")
            && parenthesized_bit_operation(call.receiver()).is_some()
        {
            let predicate = if method_name == "positive?" {
                "anybits?"
            } else {
                "nobits?"
            };
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!(
                    "Replace with `{}` for comparison with bit flags.",
                    predicate
                ),
            ));
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
                    if let Some(value) = integer_value(&argument) {
                        let is_zero = value == 0;
                        let is_one = value == 1;

                        if ((method_name == "!=" || method_name == ">") && is_zero)
                            || (method_name == ">=" && is_one)
                        {
                            let loc = node.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(
                                self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    "Replace with `anybits?` for comparison with bit flags."
                                        .to_string(),
                                ),
                            );
                        }

                        if method_name == "==" && is_zero {
                            let loc = node.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                "Replace with `nobits?` for comparison with bit flags.".to_string(),
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
