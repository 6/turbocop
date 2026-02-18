use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct IndexWith;

/// Check if the block body is an array literal `[block_param, value_expr]`
/// where the first element is a local variable reference matching the block parameter name.
fn is_index_with_block(block_node: &ruby_prism::BlockNode<'_>) -> bool {
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
    // Ensure there are no extra parameters (rest, optional, keyword, etc.)
    if param_list.rest().is_some()
        || !param_list.optionals().is_empty()
        || !param_list.posts().is_empty()
        || !param_list.keywords().is_empty()
        || param_list.keyword_rest().is_some()
    {
        return false;
    }
    let param_node = match requireds[0].as_required_parameter_node() {
        Some(p) => p,
        None => return false,
    };
    let param_name = param_node.name().as_slice();

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

    let array = match body_nodes[0].as_array_node() {
        Some(a) => a,
        None => return false,
    };
    let elements: Vec<_> = array.elements().iter().collect();
    if elements.len() != 2 {
        return false;
    }

    // First element must be a local variable read matching the block param
    let first = match elements[0].as_local_variable_read_node() {
        Some(lv) => lv,
        None => return false,
    };
    if first.name().as_slice() != param_name {
        return false;
    }
    // Second element (value) must be derived from the element (a method call),
    // not the element itself.
    if let Some(second_lvar) = elements[1].as_local_variable_read_node() {
        if second_lvar.name().as_slice() == param_name {
            return false;
        }
    }
    true
}

/// Check if the block is `each_with_object({}) { |el, memo| memo[el] = value }`
fn is_each_with_object_index_with(
    call: &ruby_prism::CallNode<'_>,
    block_node: &ruby_prism::BlockNode<'_>,
) -> bool {
    if call.name().as_slice() != b"each_with_object" {
        return false;
    }
    if let Some(args) = call.arguments() {
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return false;
        }
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
    // Arguments: [key, value] where key is el
    if let Some(args) = assign.arguments() {
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 2 {
            return false;
        }
        // Key must be the element
        let key = match arg_list[0].as_local_variable_read_node() {
            Some(lv) => lv,
            None => return false,
        };
        if key.name().as_slice() != el_name {
            return false;
        }
        // Value must NOT be the element itself
        if let Some(val_lvar) = arg_list[1].as_local_variable_read_node() {
            if val_lvar.name().as_slice() == el_name {
                return false;
            }
        }
        true
    } else {
        false
    }
}

impl Cop for IndexWith {
    fn name(&self) -> &'static str {
        "Rails/IndexWith"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Pattern 1: items.map { |e| [e, value] }.to_h
        if let Some(chain) = util::as_method_chain(node) {
            if chain.outer_method == b"to_h"
                && (chain.inner_method == b"map" || chain.inner_method == b"collect")
            {
                if let Some(block) = chain.inner_call.block() {
                    if let Some(block_node) = block.as_block_node() {
                        if is_index_with_block(&block_node) {
                            let loc = node.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            return vec![self.diagnostic(
                                source,
                                line,
                                column,
                                "Use `index_with` instead of `map { ... }.to_h`.".to_string(),
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

        // Pattern 2: items.to_h { |e| [e, value] }
        if call.name().as_slice() == b"to_h" {
            if let Some(block) = call.block() {
                if let Some(block_node) = block.as_block_node() {
                    if is_index_with_block(&block_node) {
                        let loc = node.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        return vec![self.diagnostic(
                            source,
                            line,
                            column,
                            "Use `index_with` instead of `to_h { ... }`.".to_string(),
                        )];
                    }
                }
            }
        }

        // Pattern 3: items.each_with_object({}) { |el, memo| memo[el] = value }
        if call.name().as_slice() == b"each_with_object" {
            if let Some(block) = call.block() {
                if let Some(block_node) = block.as_block_node() {
                    if is_each_with_object_index_with(&call, &block_node) {
                        let loc = node.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        return vec![self.diagnostic(
                            source,
                            line,
                            column,
                            "Use `index_with` instead of `each_with_object`.".to_string(),
                        )];
                    }
                }
            }
        }

        // Pattern 4: Hash[items.map { |e| [e, value] }]
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
                                            if is_index_with_block(&block_node) {
                                                let loc = node.location();
                                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                                return vec![self.diagnostic(
                                                    source,
                                                    line,
                                                    column,
                                                    "Use `index_with` instead of `Hash[map { ... }]`.".to_string(),
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
    crate::cop_fixture_tests!(IndexWith, "cops/rails/index_with");
}
