use crate::cop::shared::node_type::{
    BLOCK_ARGUMENT_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE,
    REQUIRED_PARAMETER_NODE, STATEMENTS_NODE, SYMBOL_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/CollectionCompact
///
/// ## Investigation findings (2026-03-20)
///
/// **FP root cause:** The cop was checking if the nil-checked variable was ANY block
/// parameter, but RuboCop only flags when it's the LAST parameter. For multi-param
/// blocks like `reject { |key, _builds| key.nil? }` or `reject { |_, val, _| val.nil? }`,
/// the nil check is on a non-last parameter which means it's destructuring, not a
/// simple element check — not equivalent to `.compact`.
///
/// **FN root cause:** The cop only handled `reject`/`reject!` patterns but was missing
/// `select`/`select!`/`filter`/`filter!` with `!param.nil?` negation patterns.
///
/// **Fix:** Added last-parameter check for reject blocks, implemented select/filter
/// support with `!param.nil?` negation detection.
///
/// ## Investigation findings (2026-03-21)
///
/// **FP root cause (AllowedReceivers traversal):** `is_allowed_receiver` was comparing
/// the full receiver source text against `AllowedReceivers`, but RuboCop's
/// `AllowedReceivers` module recursively walks the receiver chain to find the root
/// receiver name. For `params.map { |k,v| ... }.reject(&:nil?)`, RuboCop resolves
/// the root receiver as `"params"`. In Prism AST, blocks are children of the CallNode
/// (not wrappers), so the receiver chain is: `reject` → `map` CallNode → `params`
/// CallNode. Added `receiver_name()` method that recursively walks CallNode receivers
/// to find the root name, matching RuboCop's behavior.
pub struct CollectionCompact;

impl Cop for CollectionCompact {
    fn name(&self) -> &'static str {
        "Style/CollectionCompact"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_ARGUMENT_NODE,
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            CALL_NODE,
            LOCAL_VARIABLE_READ_NODE,
            REQUIRED_PARAMETER_NODE,
            STATEMENTS_NODE,
            SYMBOL_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allowed_receivers = config
            .get_string_array("AllowedReceivers")
            .unwrap_or_default();
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");

        match method_name {
            "reject" | "reject!" => {
                self.check_reject(source, &call, method_name, &allowed_receivers, diagnostics);
            }
            "select" | "select!" | "filter" | "filter!" => {
                self.check_select_filter(
                    source,
                    &call,
                    method_name,
                    &allowed_receivers,
                    diagnostics,
                );
            }
            _ => {}
        }
    }
}

impl CollectionCompact {
    /// Check reject { |e| e.nil? } and reject(&:nil?) patterns
    fn check_reject(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        method_name: &str,
        allowed_receivers: &[String],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if call.receiver().is_none() {
            return;
        }

        if self.is_allowed_receiver(call, allowed_receivers) {
            return;
        }

        let bang = if method_name.ends_with('!') { "!" } else { "" };

        // Check for block pass &:nil?
        if let Some(block_arg) = call.block() {
            if let Some(block_arg_node) = block_arg.as_block_argument_node() {
                if let Some(expr) = block_arg_node.expression() {
                    if let Some(sym) = expr.as_symbol_node() {
                        let sym_name = std::str::from_utf8(sym.unescaped()).unwrap_or("");
                        if sym_name == "nil?" {
                            let loc = call.message_loc().unwrap_or(call.location());
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                format!("Use `compact{bang}` instead of `{method_name}(&:nil?)`."),
                            ));
                        }
                    }
                }
            }
        }

        // Check for block { |e| e.nil? } — receiver of nil? must be the LAST block param
        if let Some(block) = call.block() {
            if let Some(block_node) = block.as_block_node() {
                if let Some((last_param_name, _param_count)) = self.get_last_param_name(&block_node)
                {
                    if let Some(body) = block_node.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            let stmts_list: Vec<_> = stmts.body().iter().collect();
                            if stmts_list.len() == 1 {
                                if let Some(inner_call) = stmts_list[0].as_call_node() {
                                    if self.is_nil_check_on_var(&inner_call, &last_param_name) {
                                        let loc = call.message_loc().unwrap_or(call.location());
                                        let (line, column) =
                                            source.offset_to_line_col(loc.start_offset());
                                        diagnostics.push(self.diagnostic(
                                            source,
                                            line,
                                            column,
                                            format!(
                                                "Use `compact{bang}` instead of `{method_name} {{ |e| e.nil? }}`."
                                            ),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Check select/filter { |e| !e.nil? } patterns (negated nil check)
    fn check_select_filter(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        method_name: &str,
        allowed_receivers: &[String],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if call.receiver().is_none() {
            return;
        }

        if self.is_allowed_receiver(call, allowed_receivers) {
            return;
        }

        let bang = if method_name.ends_with('!') { "!" } else { "" };

        // Check for block { |e| !e.nil? } — the body is a `!` call on `e.nil?`
        if let Some(block) = call.block() {
            if let Some(block_node) = block.as_block_node() {
                if let Some((last_param_name, _param_count)) = self.get_last_param_name(&block_node)
                {
                    if let Some(body) = block_node.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            let stmts_list: Vec<_> = stmts.body().iter().collect();
                            if stmts_list.len() == 1 {
                                // Pattern: !e.nil? is parsed as a CallNode with method `!`
                                // whose receiver is `e.nil?`
                                if let Some(not_call) = stmts_list[0].as_call_node() {
                                    let not_method =
                                        std::str::from_utf8(not_call.name().as_slice())
                                            .unwrap_or("");
                                    if not_method == "!" {
                                        // The receiver of `!` should be `e.nil?`
                                        if let Some(receiver) = not_call.receiver() {
                                            if let Some(nil_call) = receiver.as_call_node() {
                                                if self.is_nil_check_on_var(
                                                    &nil_call,
                                                    &last_param_name,
                                                ) {
                                                    let loc = call
                                                        .message_loc()
                                                        .unwrap_or(call.location());
                                                    let (line, column) = source
                                                        .offset_to_line_col(loc.start_offset());
                                                    diagnostics.push(self.diagnostic(
                                                        source,
                                                        line,
                                                        column,
                                                        format!(
                                                            "Use `compact{bang}` instead of `{method_name} {{ |e| !e.nil? }}`."
                                                        ),
                                                    ));
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

    /// Get the name of the last block parameter. Returns (name, param_count).
    fn get_last_param_name(
        &self,
        block_node: &ruby_prism::BlockNode<'_>,
    ) -> Option<(Vec<u8>, usize)> {
        let params = block_node
            .parameters()
            .and_then(|p| p.as_block_parameters_node())
            .and_then(|bp| bp.parameters())?;

        let requireds: Vec<_> = params
            .requireds()
            .iter()
            .filter_map(|r| r.as_required_parameter_node())
            .collect();

        if requireds.is_empty() {
            return None;
        }

        let last = requireds.last().unwrap();
        Some((last.name().as_slice().to_vec(), requireds.len()))
    }

    /// Check if a call node is `var.nil?` where var is a local variable read
    /// matching the given name (not a method chain like `obj.field.nil?`)
    fn is_nil_check_on_var(&self, call: &ruby_prism::CallNode<'_>, var_name: &[u8]) -> bool {
        let method = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if method != "nil?" {
            return false;
        }
        call.receiver()
            .and_then(|r| r.as_local_variable_read_node())
            .map(|lv| lv.name().as_slice() == var_name)
            .unwrap_or(false)
    }

    fn is_allowed_receiver(
        &self,
        call: &ruby_prism::CallNode<'_>,
        allowed_receivers: &[String],
    ) -> bool {
        if allowed_receivers.is_empty() {
            return false;
        }
        if let Some(receiver) = call.receiver() {
            let recv_name = self.receiver_name(&receiver);
            if allowed_receivers.contains(&recv_name) {
                return true;
            }
        }
        false
    }

    /// Recursively resolve the root receiver name, matching RuboCop's
    /// `AllowedReceivers#receiver_name` behavior.  Walks through method
    /// chains (CallNode receivers) to find the originating identifier.
    ///
    /// Examples:
    ///   `params`                        → "params"
    ///   `params.merge(key: val)`        → "params"
    ///   `params.map { |k,v| ... }`      → "params"  (block on map, Prism keeps CallNode as receiver)
    ///   `Foo::Bar`                       → source text (constant paths are not traversed)
    fn receiver_name(&self, node: &ruby_prism::Node<'_>) -> String {
        if let Some(call) = node.as_call_node() {
            if let Some(recv) = call.receiver() {
                // Stop recursing through constant receivers (matches RuboCop)
                if recv.as_constant_read_node().is_some() || recv.as_constant_path_node().is_some()
                {
                    let const_src = std::str::from_utf8(recv.location().as_slice()).unwrap_or("");
                    let method = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
                    return format!("{const_src}.{method}");
                }
                return self.receiver_name(&recv);
            }
            // No receiver — bare method call like `params`
            let method = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
            return method.to_string();
        }
        // Fallback: use the node's source text
        std::str::from_utf8(node.location().as_slice())
            .unwrap_or("")
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CollectionCompact, "cops/style/collection_compact");

    #[test]
    fn allowed_receivers_traverses_method_chains() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let mut options = HashMap::new();
        options.insert(
            "AllowedReceivers".to_string(),
            serde_yml::Value::Sequence(vec![serde_yml::Value::String("params".to_string())]),
        );
        let config = CopConfig {
            options,
            ..CopConfig::default()
        };

        // Direct receiver: params.reject(&:nil?) — should be suppressed
        let code = b"params.reject(&:nil?)";
        let diags =
            crate::testutil::run_cop_full_with_config(&CollectionCompact, code, config.clone());
        assert!(
            diags.is_empty(),
            "params.reject(&:nil?) should be suppressed by AllowedReceivers"
        );

        // Chained through map block: params.map { ... }.reject(&:nil?) — should be suppressed
        let code = b"Hash[params.map { |k, v|\n  next if k == \"name\"\n  [k.to_sym, v]\n}.reject(&:nil?)]";
        let diags =
            crate::testutil::run_cop_full_with_config(&CollectionCompact, code, config.clone());
        assert!(
            diags.is_empty(),
            "params.map {{ ... }}.reject(&:nil?) should be suppressed by AllowedReceivers"
        );

        // Chained method: params.merge(key: val).reject(&:nil?) — should be suppressed
        let code = b"params.merge(key: val).reject(&:nil?)";
        let diags =
            crate::testutil::run_cop_full_with_config(&CollectionCompact, code, config.clone());
        assert!(
            diags.is_empty(),
            "params.merge(...).reject(&:nil?) should be suppressed by AllowedReceivers"
        );

        // Non-allowed receiver: foo.reject(&:nil?) — should NOT be suppressed
        let code = b"foo.reject(&:nil?)";
        let diags =
            crate::testutil::run_cop_full_with_config(&CollectionCompact, code, config.clone());
        assert!(
            !diags.is_empty(),
            "foo.reject(&:nil?) should NOT be suppressed by AllowedReceivers"
        );
    }
}
