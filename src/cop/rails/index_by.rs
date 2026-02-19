use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ARRAY_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE, LOCAL_VARIABLE_READ_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE};

pub struct IndexBy;

/// Check if the block body is an array literal `[key_expr, block_param]`
/// where the second element is a local variable reference matching the block parameter name.
fn is_index_by_block(block_node: &ruby_prism::BlockNode<'_>) -> bool {
    // Get the block parameter name
    let params = match block_node.parameters() {
        Some(p) => p,
        None => return false,
    };
    let block_params = match params.as_block_parameters_node() {
        Some(bp) => bp,
        None => return false,
    };
    let param_list = match block_params.parameters() {
        Some(pl) => pl,
        None => return false,
    };
    let requireds: Vec<_> = param_list.requireds().iter().collect();
    if requireds.len() != 1 {
        return false;
    }
    let param_node = match requireds[0].as_required_parameter_node() {
        Some(p) => p,
        None => return false,
    };
    let param_name = param_node.name().as_slice();

    // Get block body
    let body = match block_node.body() {
        Some(b) => b,
        None => return false,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return false,
    };
    let body_nodes: Vec<_> = stmts.body().iter().collect();
    if body_nodes.len() != 1 {
        return false;
    }

    // Body must be an array literal with exactly 2 elements
    let array = match body_nodes[0].as_array_node() {
        Some(a) => a,
        None => return false,
    };
    let elements: Vec<_> = array.elements().iter().collect();
    if elements.len() != 2 {
        return false;
    }

    // Second element must be a local variable read matching the block param
    let second = match elements[1].as_local_variable_read_node() {
        Some(lv) => lv,
        None => return false,
    };
    if second.name().as_slice() != param_name {
        return false;
    }
    // First element (key) must be derived from the element (a method call),
    // not the element itself. `[el, el]` is identity, not index_by.
    if let Some(first_lvar) = elements[0].as_local_variable_read_node() {
        if first_lvar.name().as_slice() == param_name {
            return false;
        }
    }
    true
}

/// Check if the block is `each_with_object({}) { |el, memo| memo[key] = el }`
fn is_each_with_object_index(
    call: &ruby_prism::CallNode<'_>,
    block_node: &ruby_prism::BlockNode<'_>,
) -> bool {
    if call.name().as_slice() != b"each_with_object" {
        return false;
    }
    // Argument should be an empty hash literal
    if let Some(args) = call.arguments() {
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return false;
        }
        // Check for empty hash literal: {} is HashNode, but keyword hash is also possible
        let is_empty_hash = if let Some(hash) = arg_list[0].as_hash_node() {
            hash.elements().iter().count() == 0
        } else if let Some(kw_hash) = arg_list[0].as_keyword_hash_node() {
            kw_hash.elements().iter().count() == 0
        } else {
            false
        };
        if !is_empty_hash {
            return false;
        }
    } else {
        return false;
    }

    // Block params: (el, memo)
    let params = match block_node.parameters() {
        Some(p) => p,
        None => return false,
    };
    let block_params = match params.as_block_parameters_node() {
        Some(bp) => bp,
        None => return false,
    };
    let param_list = match block_params.parameters() {
        Some(pl) => pl,
        None => return false,
    };
    let requireds: Vec<_> = param_list.requireds().iter().collect();
    if requireds.len() != 2 {
        return false;
    }
    let el_param = match requireds[0].as_required_parameter_node() {
        Some(p) => p,
        None => return false,
    };
    let memo_param = match requireds[1].as_required_parameter_node() {
        Some(p) => p,
        None => return false,
    };
    let el_name = el_param.name().as_slice();
    let memo_name = memo_param.name().as_slice();

    // Block body: memo[key] = el
    let body = match block_node.body() {
        Some(b) => b,
        None => return false,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return false,
    };
    let body_nodes: Vec<_> = stmts.body().iter().collect();
    if body_nodes.len() != 1 {
        return false;
    }
    let assign = match body_nodes[0].as_call_node() {
        Some(c) => c,
        None => return false,
    };
    if assign.name().as_slice() != b"[]=" {
        return false;
    }
    // Receiver must be memo
    let recv = match assign.receiver() {
        Some(r) => r,
        None => return false,
    };
    let recv_lvar = match recv.as_local_variable_read_node() {
        Some(lv) => lv,
        None => return false,
    };
    if recv_lvar.name().as_slice() != memo_name {
        return false;
    }
    // Last argument (value) must be el
    if let Some(args) = assign.arguments() {
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 2 {
            return false;
        }
        // arg_list[0] is the key, arg_list[1] is the value
        let val = match arg_list[1].as_local_variable_read_node() {
            Some(lv) => lv,
            None => return false,
        };
        if val.name().as_slice() != el_name {
            return false;
        }
        // Key must be derived from the element (a method call on it),
        // not the element itself. `memo[el] = el` is identity, not index_by.
        let key = &arg_list[0];
        if let Some(key_lvar) = key.as_local_variable_read_node() {
            if key_lvar.name().as_slice() == el_name {
                return false; // key IS the element — not an index_by pattern
            }
        }
        true
    } else {
        false
    }
}

impl Cop for IndexBy {
    fn name(&self) -> &'static str {
        "Rails/IndexBy"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE, LOCAL_VARIABLE_READ_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Pattern 1: items.map { |e| [key, e] }.to_h
        if let Some(chain) = util::as_method_chain(node) {
            if chain.outer_method == b"to_h"
                && (chain.inner_method == b"map" || chain.inner_method == b"collect")
            {
                if let Some(block) = chain.inner_call.block() {
                    if let Some(block_node) = block.as_block_node() {
                        if is_index_by_block(&block_node) {
                            let loc = node.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            return vec![self.diagnostic(
                                source,
                                line,
                                column,
                                "Use `index_by` instead of `map { ... }.to_h`.".to_string(),
                            )];
                        }
                    }
                }
            }
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Pattern 2: items.to_h { |e| [key, e] }
        if call.name().as_slice() == b"to_h" {
            if let Some(block) = call.block() {
                if let Some(block_node) = block.as_block_node() {
                    if is_index_by_block(&block_node) {
                        let loc = node.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        return vec![self.diagnostic(
                            source,
                            line,
                            column,
                            "Use `index_by` instead of `to_h { ... }`.".to_string(),
                        )];
                    }
                }
            }
        }

        // Pattern 3: items.each_with_object({}) { |el, memo| memo[key] = el }
        if call.name().as_slice() == b"each_with_object" {
            if let Some(block) = call.block() {
                if let Some(block_node) = block.as_block_node() {
                    if is_each_with_object_index(&call, &block_node) {
                        let loc = node.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        return vec![self.diagnostic(
                            source,
                            line,
                            column,
                            "Use `index_by` instead of `each_with_object`.".to_string(),
                        )];
                    }
                }
            }
        }

        // Pattern 4: Hash[items.map { |e| [key, e] }]
        if call.name().as_slice() == b"[]" {
            if let Some(recv) = call.receiver() {
                if util::constant_name(&recv) == Some(b"Hash") {
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if arg_list.len() == 1 {
                            if let Some(inner_call) = arg_list[0].as_call_node() {
                                let name = inner_call.name().as_slice();
                                if name == b"map" || name == b"collect" {
                                    if let Some(block) = inner_call.block() {
                                        if let Some(block_node) = block.as_block_node() {
                                            if is_index_by_block(&block_node) {
                                                let loc = node.location();
                                                let (line, column) =
                                                    source.offset_to_line_col(loc.start_offset());
                                                return vec![self.diagnostic(
                                                    source,
                                                    line,
                                                    column,
                                                    "Use `index_by` instead of `Hash[map { ... }]`."
                                                        .to_string(),
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
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(IndexBy, "cops/rails/index_by");

    #[test]
    fn identity_each_with_object_not_flagged() {
        // memo[record] = record — key is the element itself, not a method on it
        let source = b"records.each_with_object({}) { |record, h| h[record] = record }\n";
        let diags = run_cop_full(&IndexBy, source);
        assert!(diags.is_empty(), "identity each_with_object should not be flagged: {:?}", diags);
    }

    #[test]
    fn identity_hash_map_not_flagged() {
        // Hash[map { |name| [name, name] }] — key is the element itself
        let source = b"Hash[columns.map { |name| [name, name] }]\n";
        let diags = run_cop_full(&IndexBy, source);
        assert!(diags.is_empty(), "identity Hash[map] should not be flagged: {:?}", diags);
    }

    #[test]
    fn method_key_each_with_object_flagged() {
        // memo[record.id] = record — key is a method call on element
        let source = b"records.each_with_object({}) { |record, h| h[record.id] = record }\n";
        let diags = run_cop_full(&IndexBy, source);
        assert_eq!(diags.len(), 1, "method-key each_with_object should be flagged");
    }

    #[test]
    fn method_key_hash_map_flagged() {
        // Hash[map { |e| [e.id, e] }] — key is a method call on element
        let source = b"Hash[items.map { |e| [e.id, e] }]\n";
        let diags = run_cop_full(&IndexBy, source);
        assert_eq!(diags.len(), 1, "method-key Hash[map] should be flagged");
    }
}
