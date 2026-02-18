use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct CollectionCompact;

impl Cop for CollectionCompact {
    fn name(&self) -> &'static str {
        "Style/CollectionCompact"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allowed_receivers = config.get_string_array("AllowedReceivers").unwrap_or_default();
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");

        // Pattern: array.reject { |e| e.nil? } or array.reject(&:nil?)
        if method_name == "reject" || method_name == "reject!" {
            if call.receiver().is_none() {
                return Vec::new();
            }

            // Check AllowedReceivers
            if let Some(receiver) = call.receiver() {
                let recv_src = std::str::from_utf8(receiver.location().as_slice()).unwrap_or("");
                if allowed_receivers.iter().any(|ar| recv_src == ar.as_str()) {
                    return Vec::new();
                }
            }

            // Check for block pass &:nil?
            if let Some(block_arg) = call.block() {
                if let Some(block_arg_node) = block_arg.as_block_argument_node() {
                    if let Some(expr) = block_arg_node.expression() {
                        if let Some(sym) = expr.as_symbol_node() {
                            let sym_name = std::str::from_utf8(sym.unescaped()).unwrap_or("");
                            if sym_name == "nil?" {
                                let bang = if method_name == "reject!" { "!" } else { "" };
                                let loc = call.message_loc().unwrap_or(call.location());
                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                return vec![self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    format!("Use `compact{}` instead of `{}(&:nil?)`.", bang, method_name),
                                )];
                            }
                        }
                    }
                }
            }

            // Check for block { |e| e.nil? }
            if let Some(block) = call.block() {
                if let Some(block_node) = block.as_block_node() {
                    if let Some(body) = block_node.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            let stmts_list: Vec<_> = stmts.body().iter().collect();
                            if stmts_list.len() == 1 {
                                if let Some(inner_call) = stmts_list[0].as_call_node() {
                                    let inner_method = std::str::from_utf8(inner_call.name().as_slice()).unwrap_or("");
                                    if inner_method == "nil?" {
                                        let bang = if method_name == "reject!" { "!" } else { "" };
                                        let loc = call.message_loc().unwrap_or(call.location());
                                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                                        return vec![self.diagnostic(
                                            source,
                                            line,
                                            column,
                                            format!("Use `compact{}` instead of `{} {{ |e| e.nil? }}`.", bang, method_name),
                                        )];
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
    crate::cop_fixture_tests!(CollectionCompact, "cops/style/collection_compact");
}
