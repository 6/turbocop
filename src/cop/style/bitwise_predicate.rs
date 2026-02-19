use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, INTEGER_NODE, PARENTHESES_NODE, STATEMENTS_NODE};

pub struct BitwisePredicate;

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
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");

        // Pattern: (variable & flags).positive? => variable.anybits?(flags)
        if method_name == "positive?" || method_name == "zero?" {
            if let Some(receiver) = call.receiver() {
                if let Some(paren) = receiver.as_parentheses_node() {
                    if let Some(body) = paren.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            let stmt_list: Vec<_> = stmts.body().iter().collect();
                            if stmt_list.len() == 1 {
                                if let Some(inner_call) = stmt_list[0].as_call_node() {
                                    let inner_method = std::str::from_utf8(inner_call.name().as_slice()).unwrap_or("");
                                    if inner_method == "&" {
                                        let predicate = if method_name == "positive?" {
                                            "anybits?"
                                        } else {
                                            "nobits?"
                                        };
                                        let loc = node.location();
                                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                                        return vec![self.diagnostic(
                                            source,
                                            line,
                                            column,
                                            format!("Replace with `{}` for comparison with bit flags.", predicate),
                                        )];
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Pattern: (variable & flags) > 0 / != 0 / == 0
        if matches!(method_name, ">" | "!=" | "==" | ">=") {
            if let Some(receiver) = call.receiver() {
                if let Some(paren) = receiver.as_parentheses_node() {
                    if let Some(body) = paren.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            let stmt_list: Vec<_> = stmts.body().iter().collect();
                            if stmt_list.len() == 1 {
                                if let Some(inner_call) = stmt_list[0].as_call_node() {
                                    let inner_method = std::str::from_utf8(inner_call.name().as_slice()).unwrap_or("");
                                    if inner_method == "&" {
                                        // Check that the comparison value is 0 or 1
                                        if let Some(args) = call.arguments() {
                                            let arg_list: Vec<_> = args.arguments().iter().collect();
                                            if arg_list.len() == 1 {
                                                if let Some(int_node) = arg_list[0].as_integer_node() {
                                                    let src = std::str::from_utf8(int_node.location().as_slice()).unwrap_or("");
                                                    if let Ok(v) = src.parse::<i64>() {
                                                        let is_zero = v == 0;
                                                        let is_one = v == 1;
                                                        if (method_name == ">" && is_zero)
                                                            || (method_name == "!=" && is_zero)
                                                            || (method_name == ">=" && is_one)
                                                        {
                                                            let loc = node.location();
                                                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                                                            return vec![self.diagnostic(
                                                                source,
                                                                line,
                                                                column,
                                                                "Replace with `anybits?` for comparison with bit flags.".to_string(),
                                                            )];
                                                        }
                                                        if method_name == "==" && is_zero {
                                                            let loc = node.location();
                                                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                                                            return vec![self.diagnostic(
                                                                source,
                                                                line,
                                                                column,
                                                                "Replace with `nobits?` for comparison with bit flags.".to_string(),
                                                            )];
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BitwisePredicate, "cops/style/bitwise_predicate");
}
