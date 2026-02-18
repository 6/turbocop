use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct CollectionQuerying;

impl Cop for CollectionQuerying {
    fn name(&self) -> &'static str {
        "Style/CollectionQuerying"
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

        // Pattern: x.count.positive? => x.any?
        // Pattern: x.count.zero? => x.none?
        if method_name == "positive?" || method_name == "zero?" {
            if call.arguments().is_some() {
                return Vec::new();
            }

            if let Some(receiver) = call.receiver() {
                if let Some(recv_call) = receiver.as_call_node() {
                    let recv_method = std::str::from_utf8(recv_call.name().as_slice()).unwrap_or("");
                    if matches!(recv_method, "count" | "length" | "size") {
                        if recv_call.receiver().is_some() {
                            let suggestion = if method_name == "positive?" {
                                "any?"
                            } else {
                                "none?"
                            };

                            let loc = recv_call.message_loc().unwrap_or(recv_call.location());
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            return vec![self.diagnostic(
                                source,
                                line,
                                column,
                                format!("Use `{}` instead.", suggestion),
                            )];
                        }
                    }
                }
            }
        }

        // Pattern: x.count > 0 => x.any?
        // Pattern: x.count == 0 => x.none?
        if matches!(method_name, ">" | "==" | "!=") {
            if let Some(receiver) = call.receiver() {
                if let Some(recv_call) = receiver.as_call_node() {
                    let recv_method = std::str::from_utf8(recv_call.name().as_slice()).unwrap_or("");
                    if matches!(recv_method, "count" | "length" | "size") {
                        if recv_call.receiver().is_some() {
                            if let Some(args) = call.arguments() {
                                let arg_list: Vec<_> = args.arguments().iter().collect();
                                if arg_list.len() == 1 {
                                    if let Some(int_node) = arg_list[0].as_integer_node() {
                                        let src = std::str::from_utf8(int_node.location().as_slice()).unwrap_or("");
                                        if let Ok(v) = src.parse::<i64>() {
                                            if v == 0 {
                                                let suggestion = if method_name == ">" || method_name == "!=" {
                                                    "any?"
                                                } else {
                                                    "none?"
                                                };
                                                let loc = recv_call.message_loc().unwrap_or(recv_call.location());
                                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                                return vec![self.diagnostic(
                                                    source,
                                                    line,
                                                    column,
                                                    format!("Use `{}` instead.", suggestion),
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

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CollectionQuerying, "cops/style/collection_querying");
}
