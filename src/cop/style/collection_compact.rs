use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_ARGUMENT_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE, SYMBOL_NODE};

pub struct CollectionCompact;

impl Cop for CollectionCompact {
    fn name(&self) -> &'static str {
        "Style/CollectionCompact"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_ARGUMENT_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE, SYMBOL_NODE]
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

            // Check for block { |e| e.nil? } or { |k, v| v.nil? }
            if let Some(block) = call.block() {
                if let Some(block_node) = block.as_block_node() {
                    // Collect all block parameter names
                    let param_names: Vec<Vec<u8>> = block_node.parameters()
                        .and_then(|p| p.as_block_parameters_node())
                        .and_then(|bp| bp.parameters())
                        .map(|params| {
                            params.requireds().iter()
                                .filter_map(|r| r.as_required_parameter_node()
                                    .map(|rp| rp.name().as_slice().to_vec()))
                                .collect()
                        })
                        .unwrap_or_default();

                    if !param_names.is_empty() {
                        if let Some(body) = block_node.body() {
                            if let Some(stmts) = body.as_statements_node() {
                                let stmts_list: Vec<_> = stmts.body().iter().collect();
                                if stmts_list.len() == 1 {
                                    if let Some(inner_call) = stmts_list[0].as_call_node() {
                                        let inner_method = std::str::from_utf8(inner_call.name().as_slice()).unwrap_or("");
                                        if inner_method == "nil?" {
                                            // Verify the receiver is one of the block parameters directly
                                            // (not a method chain like `notification.target_status.nil?`)
                                            let receiver_is_param = inner_call.receiver()
                                                .and_then(|r| r.as_local_variable_read_node())
                                                .map(|lv| param_names.iter().any(|p| lv.name().as_slice() == p.as_slice()))
                                                .unwrap_or(false);

                                            if receiver_is_param {
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
