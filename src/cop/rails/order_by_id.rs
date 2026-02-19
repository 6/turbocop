use crate::cop::util::keyword_arg_value;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE};

pub struct OrderById;

impl Cop for OrderById {
    fn name(&self) -> &'static str {
        "Rails/OrderById"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"order" {
            return;
        }

        // Must have a receiver
        if call.receiver().is_none() {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        let is_order_by_id = if let Some(sym) = arg_list[0].as_symbol_node() {
            sym.unescaped() == b"id"
        } else if arg_list[0].as_hash_node().is_some() || arg_list[0].as_keyword_hash_node().is_some() {
            // order(id: :asc) or order(id: :desc)
            keyword_arg_value(&call, b"id").is_some()
        } else {
            false
        };

        if !is_order_by_id {
            // Also check: order(primary_key) - call to primary_key method
            if let Some(pk_call) = arg_list[0].as_call_node() {
                if pk_call.name().as_slice() != b"primary_key" {
                    return;
                }
            } else {
                return;
            }
        }

        let msg_loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Do not use the `id` column for ordering. Use a timestamp column to order chronologically.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OrderById, "cops/rails/order_by_id");
}
