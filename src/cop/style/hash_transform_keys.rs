use crate::cop::node_type::{
    ARRAY_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE,
    LOCAL_VARIABLE_READ_NODE, MULTI_TARGET_NODE, STATEMENTS_NODE,
};
use crate::cop::util::is_simple_constant;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Detects hash key transformations that can use `transform_keys` instead.
///
/// Handles two patterns:
/// - `each_with_object({}) { |(k, v), h| h[expr(k)] = v }` → `transform_keys`
/// - `Hash[_.map { |k, v| [expr(k), v] }]` → `transform_keys`
///
/// The `Hash[_.map {...}]` pattern matches `Hash.[]` calls where the sole
/// argument is a `.map` or `.collect` call with a block that returns a
/// two-element array `[key_expr, val]`, where `val` is the unchanged value
/// parameter and `key_expr` is a transformation of the key.
pub struct HashTransformKeys;

impl Cop for HashTransformKeys {
    fn name(&self) -> &'static str {
        "Style/HashTransformKeys"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ARRAY_NODE,
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            CALL_NODE,
            HASH_NODE,
            KEYWORD_HASH_NODE,
            LOCAL_VARIABLE_READ_NODE,
            MULTI_TARGET_NODE,
            STATEMENTS_NODE,
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

        let call_name = call.name().as_slice();

        if call_name == b"each_with_object" {
            self.check_each_with_object(source, &call, diagnostics);
        } else if call_name == b"[]" {
            self.check_hash_brackets_map(source, &call, diagnostics);
        }
    }
}

impl HashTransformKeys {
    /// Check `each_with_object({}) { |(k, v), h| h[expr(k)] = v }` pattern.
    fn check_each_with_object(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let block = match call.block() {
            Some(b) => b,
            None => return,
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Check that the argument to each_with_object is an empty hash
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1
            || (arg_list[0].as_hash_node().is_none()
                && arg_list[0].as_keyword_hash_node().is_none())
        {
            return;
        }

        // Check empty hash by looking at source between { and }
        if let Some(hash) = arg_list[0].as_hash_node() {
            let hash_src = hash.location().as_slice();
            let trimmed: Vec<u8> = hash_src
                .iter()
                .filter(|&&b| b != b' ' && b != b'{' && b != b'}')
                .copied()
                .collect();
            if !trimmed.is_empty() {
                return;
            }
        }

        // RuboCop requires destructured block parameters: |(k, v), h|
        // This ensures the receiver is iterated as key-value pairs (i.e. a hash).
        // Simple params like |klass, classes| indicate an array/enumerable, not a hash.
        let params = match block_node.parameters() {
            Some(p) => p,
            None => return,
        };
        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return,
        };
        let bp_params = match block_params.parameters() {
            Some(p) => p,
            None => return,
        };

        // Need exactly 2 params: first must be destructured (mlhs), second is the hash accumulator
        let reqs: Vec<_> = bp_params.requireds().iter().collect();
        if reqs.len() != 2 {
            return;
        }
        // First param must be destructured (MultiTargetNode) with exactly 2 targets
        let multi_target = match reqs[0].as_multi_target_node() {
            Some(mt) => mt,
            None => return,
        };
        let targets: Vec<_> = multi_target.lefts().iter().collect();
        if targets.len() != 2 {
            return;
        }

        // Extract the value parameter name (second element of the destructured pair)
        // e.g., in |(k, v), h|, the value param is `v`
        let value_param_name = match targets[1].as_required_parameter_node() {
            Some(p) => p.name(),
            None => return,
        };

        // Check body has a single statement that looks like h[expr] = v
        // where expr is NOT a simple variable (key is transformed)
        // and v is specifically the VALUE parameter from the destructured pair
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

        // Check for h[key_expr] = v pattern (CallNode with name []=)
        if let Some(assign_call) = body_nodes[0].as_call_node() {
            if assign_call.name().as_slice() == b"[]=" {
                if let Some(assign_args) = assign_call.arguments() {
                    let aargs: Vec<_> = assign_args.arguments().iter().collect();
                    if aargs.len() == 2 {
                        let key_is_simple = aargs[0].as_local_variable_read_node().is_some();
                        if key_is_simple {
                            return;
                        }
                        // The assigned value must be a local variable matching
                        // the VALUE parameter from the destructured pair.
                        // This prevents flagging hash-inversion patterns like
                        // |(id, attrs), h| h[attrs[:code]] = id
                        // where `id` is the KEY param, not the VALUE param.
                        if let Some(val_lvar) = aargs[1].as_local_variable_read_node() {
                            if val_lvar.name().as_slice() == value_param_name.as_slice() {
                                let loc = call.location();
                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                diagnostics.push(self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    "Prefer `transform_keys` over `each_with_object`.".to_string(),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Check `Hash[_.map { |k, v| [key_expr, v] }]` pattern.
    fn check_hash_brackets_map(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Receiver must be `Hash` or `::Hash`.
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        if !is_simple_constant(&receiver, b"Hash") {
            return;
        }

        // Must have exactly one argument: the map/collect call
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        // The argument must be a CallNode for .map or .collect with a block
        let map_call = match arg_list[0].as_call_node() {
            Some(c) => c,
            None => return,
        };
        let map_name = map_call.name().as_slice();
        if map_name != b"map" && map_name != b"collect" {
            return;
        }

        let block = match map_call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Block must have exactly 2 simple (non-destructured) params: |k, v|
        let params = match block_node.parameters() {
            Some(p) => p,
            None => return,
        };
        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return,
        };
        let bp_params = match block_params.parameters() {
            Some(p) => p,
            None => return,
        };
        let reqs: Vec<_> = bp_params.requireds().iter().collect();
        if reqs.len() != 2 {
            return;
        }
        // Both params must be simple RequiredParameterNode (not destructured)
        let key_param = match reqs[0].as_required_parameter_node() {
            Some(p) => p,
            None => return,
        };
        let val_param = match reqs[1].as_required_parameter_node() {
            Some(p) => p,
            None => return,
        };

        // Block body must be a single ArrayNode with exactly 2 elements
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
        let array = match body_nodes[0].as_array_node() {
            Some(a) => a,
            None => return,
        };
        let elements: Vec<_> = array.elements().iter().collect();
        if elements.len() != 2 {
            return;
        }

        // Second element must be a local variable read matching the value param
        let val_elem = match elements[1].as_local_variable_read_node() {
            Some(lv) => lv,
            None => return,
        };
        if val_elem.name().as_slice() != val_param.name().as_slice() {
            return;
        }

        // First element must NOT be a simple local variable read of the key param
        // (if key is passed through unchanged, it's not a key transformation)
        if let Some(key_lvar) = elements[0].as_local_variable_read_node() {
            if key_lvar.name().as_slice() == key_param.name().as_slice() {
                return;
            }
        }

        // The key expression must actually reference the key parameter.
        // Without this check, patterns like `Hash[x.map { |_, v| [v.foo, v] }]`
        // would be falsely flagged — the key expression derives from the value,
        // not the key, so this isn't a key transformation.
        if !node_contains_lvar_read(&elements[0], key_param.name().as_slice()) {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Prefer `transform_keys` over `Hash[_.map {...}]`.".to_string(),
        ));
    }
}

/// Check if a node's subtree contains a `LocalVariableReadNode` with the given name.
fn node_contains_lvar_read(node: &ruby_prism::Node<'_>, name: &[u8]) -> bool {
    let mut finder = LvarFinder { name, found: false };
    finder.visit(node);
    finder.found
}

struct LvarFinder<'a> {
    name: &'a [u8],
    found: bool,
}

impl<'pr> Visit<'pr> for LvarFinder<'_> {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        if node.name().as_slice() == self.name {
            self.found = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashTransformKeys, "cops/style/hash_transform_keys");
}
