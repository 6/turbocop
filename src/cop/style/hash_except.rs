use crate::cop::node_type::{
    BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE,
    REQUIRED_PARAMETER_NODE, STATEMENTS_NODE, STRING_NODE, SYMBOL_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Detects `Hash#reject`, `Hash#select`, and `Hash#filter` calls that can be
/// replaced with `Hash#except`.
///
/// Handles two families of patterns:
///
/// 1. **Comparison patterns** (`==` / `!=`):
///    - `hash.reject { |k, _| k == :sym }` → `hash.except(:sym)`
///    - `hash.select { |k, _| k != :sym }` → `hash.except(:sym)`
///      Only flags string/symbol comparands (mirrors RuboCop safety gate).
///
/// 2. **`include?` patterns**:
///    - `hash.reject { |k, _| COLLECTION.include?(k) }` → `hash.except(*COLLECTION)`
///    - `hash.select { |k, _| !COLLECTION.include?(k) }` → `hash.except(*COLLECTION)`
///      Works with array literals (`[:a, :b]`), constants, and variables.
///      Array literal receivers produce `except(:a, :b)` (expanded);
///      all others produce `except(*name)` (splatted).
pub struct HashExcept;

impl Cop for HashExcept {
    fn name(&self) -> &'static str {
        "Style/HashExcept"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            CALL_NODE,
            LOCAL_VARIABLE_READ_NODE,
            REQUIRED_PARAMETER_NODE,
            STATEMENTS_NODE,
            STRING_NODE,
            SYMBOL_NODE,
        ]
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

        let method_bytes = call.name().as_slice();

        // Only handle reject, select, filter
        if method_bytes != b"reject" && method_bytes != b"select" && method_bytes != b"filter" {
            return;
        }

        // Must have a receiver
        if call.receiver().is_none() {
            return;
        }

        // Must have a block
        let block = match call.block() {
            Some(b) => b,
            None => return,
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Must have exactly 2 block parameters (|k, v|)
        let params = match block_node.parameters() {
            Some(p) => p,
            None => return,
        };

        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return,
        };

        let parameters = match block_params.parameters() {
            Some(p) => p,
            None => return,
        };

        let requireds: Vec<_> = parameters.requireds().iter().collect();
        if requireds.len() != 2 {
            return;
        }

        // Get the key parameter name
        let key_param = match requireds[0].as_required_parameter_node() {
            Some(p) => p,
            None => return,
        };
        let key_name = key_param.name().as_slice();

        // Get the value parameter name (needed for include? checks)
        let value_param = match requireds[1].as_required_parameter_node() {
            Some(p) => p,
            None => return,
        };
        let value_name = value_param.name().as_slice();

        // Check the block body for a simple comparison pattern
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.len() != 1 {
            return;
        }

        let expr = &body_nodes[0];

        // Try to match the expression against known patterns
        if let Some(outer_call) = expr.as_call_node() {
            let outer_method = outer_call.name().as_slice();

            // Pattern: !SOMETHING.include?(key) — negation wrapper
            if outer_method == b"!" {
                if let Some(inner) = outer_call.receiver() {
                    if let Some(inner_call) = inner.as_call_node() {
                        if inner_call.name().as_slice() == b"include?" {
                            // Negated include? is except-like for select/filter only
                            if method_bytes == b"select" || method_bytes == b"filter" {
                                self.check_include_pattern(
                                    source,
                                    &call,
                                    &inner_call,
                                    key_name,
                                    value_name,
                                    diagnostics,
                                );
                            }
                        }
                    }
                }
                return;
            }

            // Pattern: SOMETHING.include?(key) — for reject only
            if outer_method == b"include?" {
                if method_bytes == b"reject" {
                    self.check_include_pattern(
                        source,
                        &call,
                        &outer_call,
                        key_name,
                        value_name,
                        diagnostics,
                    );
                }
                return;
            }

            // Pattern: k == :sym / k != :sym (existing logic)
            if outer_method == b"==" || outer_method == b"!=" {
                // For reject: k == :sym -> except(:sym)
                // For select/filter: k != :sym -> except(:sym)
                let is_matching = (method_bytes == b"reject" && outer_method == b"==")
                    || ((method_bytes == b"select" || method_bytes == b"filter")
                        && outer_method == b"!=");

                if !is_matching {
                    return;
                }

                let cmp_recv = match outer_call.receiver() {
                    Some(r) => r,
                    None => return,
                };

                let cmp_args = match outer_call.arguments() {
                    Some(a) => a,
                    None => return,
                };

                let cmp_arg_list: Vec<_> = cmp_args.arguments().iter().collect();
                if cmp_arg_list.len() != 1 {
                    return;
                }

                // One side must be the key param, other must be a literal
                let value_node = if let Some(lvar) = cmp_recv.as_local_variable_read_node() {
                    if lvar.name().as_slice() == key_name {
                        &cmp_arg_list[0]
                    } else {
                        return;
                    }
                } else if let Some(lvar) = cmp_arg_list[0].as_local_variable_read_node() {
                    if lvar.name().as_slice() == key_name {
                        &cmp_recv
                    } else {
                        return;
                    }
                } else {
                    return;
                };

                // Value must be a symbol or string literal
                let is_sym_or_str =
                    value_node.as_symbol_node().is_some() || value_node.as_string_node().is_some();

                if !is_sym_or_str {
                    return;
                }

                let value_src = &source.as_bytes()
                    [value_node.location().start_offset()..value_node.location().end_offset()];
                let value_str = String::from_utf8_lossy(value_src);

                let loc = call.message_loc().unwrap_or_else(|| call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `except({})` instead.", value_str),
                ));
            }
        }
    }
}

impl HashExcept {
    /// Check and emit an offense for the `include?` pattern.
    ///
    /// `include_call` is the `SOMETHING.include?(key)` CallNode.
    /// `outer_call` is the top-level `reject`/`select`/`filter` CallNode.
    fn check_include_pattern(
        &self,
        source: &SourceFile,
        outer_call: &ruby_prism::CallNode<'_>,
        include_call: &ruby_prism::CallNode<'_>,
        key_name: &[u8],
        value_name: &[u8],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // include? must have exactly one argument
        let args = match include_call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        // The argument must be the key block parameter
        let arg = &arg_list[0];
        let arg_is_key = if let Some(lvar) = arg.as_local_variable_read_node() {
            lvar.name().as_slice() == key_name
        } else {
            false
        };
        if !arg_is_key {
            return;
        }

        // The receiver of include? is the collection
        let collection = match include_call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Reject if the collection is a range
        if collection.as_range_node().is_some() {
            return;
        }
        // Also unwrap one level of parentheses for range check
        if let Some(parens) = collection.as_parentheses_node() {
            if let Some(inner_body) = parens.body() {
                if let Some(stmts) = inner_body.as_statements_node() {
                    let inner_nodes: Vec<_> = stmts.body().iter().collect();
                    if inner_nodes.len() == 1 && inner_nodes[0].as_range_node().is_some() {
                        return;
                    }
                }
            }
        }

        // Reject if the collection is the value block parameter
        if let Some(lvar) = collection.as_local_variable_read_node() {
            if lvar.name().as_slice() == value_name {
                return;
            }
        }

        // Generate the message based on the collection type
        let key_source = if let Some(arr) = collection.as_array_node() {
            // Array literal: list elements
            let elements: Vec<String> = arr
                .elements()
                .iter()
                .map(|elem| {
                    let src = &source.as_bytes()
                        [elem.location().start_offset()..elem.location().end_offset()];
                    String::from_utf8_lossy(src).to_string()
                })
                .collect();
            elements.join(", ")
        } else {
            // Variable/constant/expression: use splat
            let src = &source.as_bytes()
                [collection.location().start_offset()..collection.location().end_offset()];
            format!("*{}", String::from_utf8_lossy(src))
        };

        let loc = outer_call
            .message_loc()
            .unwrap_or_else(|| outer_call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `except({})` instead.", key_source),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashExcept, "cops/style/hash_except");
}
