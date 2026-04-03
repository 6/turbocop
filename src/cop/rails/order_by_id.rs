use crate::cop::shared::node_type::{CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE};
use crate::cop::shared::util::keyword_arg_value;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/OrderById: flags `order(:id)`, `order(id: dir)`, `order(primary_key)`,
/// and `order(primary_key => dir)`.
///
/// ## FP root cause (77 FP)
/// RuboCop's `order_by_id?` pattern only matches hash args with EXACTLY ONE pair:
/// `(hash (pair (sym :id) _))`. Nitrocop was calling `keyword_arg_value(&call, b"id")`
/// without checking pair count, so `order(id: :asc, name: :desc)` was incorrectly flagged.
/// Fix: verify hash/keyword_hash has exactly 1 element before checking the key.
///
/// ## FN root cause (210 FN)
/// RuboCop also matches `order(primary_key => value)` — a hash pair where the key is a
/// `primary_key` method call. Nitrocop only handled bare `primary_key` as a symbol-like
/// argument, not as a hash key. Fix: when processing single-pair hashes, also check if
/// the key is a CallNode with method `primary_key`.
///
/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=2, FN=0.
///
/// FP=2: Both from safe navigation chains (`&.order(:id)` and `&.order(id: :asc)`).
/// RuboCop's pattern uses `(send _ :order ...)` which excludes `csend` (safe navigation).
/// Fixed by checking `call_operator_loc()` for `&.` and skipping.
pub struct OrderById;

impl Cop for OrderById {
    fn name(&self) -> &'static str {
        "Rails/OrderById"
    }

    fn default_enabled(&self) -> bool {
        false
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
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"order" {
            return;
        }

        // RuboCop uses (send _ :order ...) which excludes csend (safe navigation).
        // Skip &.order(:id) chains.
        if call
            .call_operator_loc()
            .is_some_and(|op| op.as_slice() == b"&.")
        {
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

        // Check for the various patterns RuboCop matches:
        // 1. order(:id) — bare symbol
        // 2. order(id: dir) — single-pair keyword hash
        // 3. order(primary_key) — bare method call
        // 4. order(primary_key => dir) — single-pair hash with primary_key call as key
        let arg = &arg_list[0];

        let is_order_by_id = if let Some(sym) = arg.as_symbol_node() {
            // Pattern 1: order(:id)
            sym.unescaped() == b"id"
        } else if let Some(kw) = arg.as_keyword_hash_node() {
            // Pattern 2 & 4: keyword hash — must have exactly 1 pair
            if kw.elements().iter().count() != 1 {
                return;
            }
            if keyword_arg_value(&call, b"id").is_some() {
                true
            } else {
                // Check for primary_key => value
                hash_key_is_primary_key(kw.elements().iter().next())
            }
        } else if let Some(hash) = arg.as_hash_node() {
            // Pattern 2 & 4: explicit hash — must have exactly 1 pair
            if hash.elements().iter().count() != 1 {
                return;
            }
            if keyword_arg_value(&call, b"id").is_some() {
                true
            } else {
                hash_key_is_primary_key(hash.elements().iter().next())
            }
        } else if let Some(pk_call) = arg.as_call_node() {
            // Pattern 3: order(primary_key)
            pk_call.name().as_slice() == b"primary_key"
        } else {
            false
        };

        if !is_order_by_id {
            return;
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

/// Check if a hash element's key is a `primary_key` method call.
fn hash_key_is_primary_key(elem: Option<ruby_prism::Node<'_>>) -> bool {
    let elem = match elem {
        Some(e) => e,
        None => return false,
    };
    let assoc = match elem.as_assoc_node() {
        Some(a) => a,
        None => return false,
    };
    match assoc.key().as_call_node() {
        Some(c) => c.name().as_slice() == b"primary_key",
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OrderById, "cops/rails/order_by_id");
}
