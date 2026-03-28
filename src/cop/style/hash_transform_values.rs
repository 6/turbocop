use crate::cop::node_type::{
    ARRAY_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, CONSTANT_PATH_NODE,
    CONSTANT_READ_NODE, HASH_NODE, LOCAL_VARIABLE_READ_NODE, MULTI_TARGET_NODE, STATEMENTS_NODE,
};
use crate::cop::util::is_simple_constant;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Style/HashTransformValues detects hash iteration patterns that can be
/// replaced with `transform_values`.
///
/// ## Patterns detected (matching RuboCop)
///
/// 1. `each_with_object({}) { |(k, v), h| h[k] = expr(v) }`
/// 2. `Hash[x.map { |k, v| [k, expr(v)] }]`
/// 3. `x.map { |k, v| [k, expr(v)] }.to_h`
/// 4. `x.to_h { |k, v| [k, expr(v)] }`
///
/// ## Investigation findings (corpus: 47 FP, 104 FN)
///
/// **FP root causes:**
/// - Missing destructured-params check: the cop fired on `|item, memo|` single-param
///   blocks (e.g. `items.each_with_object({}) { |item, result| result[item] = true }`).
///   RuboCop requires `|(k, v), h|` destructured params, confirming the receiver yields
///   key-value pairs (i.e., is a hash). Without this, array-to-hash patterns were falsely
///   flagged.
/// - Missing memo-variable check: value expressions referencing the memo hash (e.g.
///   `h[k] = h.size + v`) can't use transform_values.
///
/// **FN root causes:**
/// - Only `each_with_object` was implemented. The three other patterns
///   (`Hash[_.map]`, `_.map.to_h`, `_.to_h`) were completely missing.
///
/// **Fixes applied:**
/// - Added destructured block parameter validation (must be `|(k, v), h|` with
///   MultiTargetNode) for `each_with_object`.
/// - Added memo-variable check for `each_with_object` value expressions.
/// - Implemented `Hash[_.map/collect]`, `_.map/collect.to_h`, and `_.to_h` patterns.
/// - Added `array_receiver?` check to exclude array literals.
/// - All four patterns share common validation: key must pass through unchanged,
///   value must be transformed (not noop), value transformation must not reference the key.
/// - Fixed `::Hash[...]` (ConstantPathNode) not being recognized — the receiver check
///   compared raw source bytes which included the `::` prefix. Replaced with
///   `is_simple_constant` which handles both `Hash` and `::Hash`.
/// - Multi-line blocks and `do...end` syntax already worked correctly with the
///   existing Prism-based detection (no code change needed for those patterns).
/// - Replaced text-based `contains_identifier` with AST-based `node_contains_lvar_read`
///   for checking whether value expressions reference the key or memo parameter.
///   The text-based approach falsely matched key param names appearing as Ruby symbols
///   (`:name`, `&:label`) or keyword arguments (`name: nil`), causing false negatives
///   in patterns like `x.map { |name, attr| [name, Param.new(name: nil)] }.to_h`
///   where `name:` is a keyword arg, not a local variable reference to `name`.
pub struct HashTransformValues;

impl Cop for HashTransformValues {
    fn name(&self) -> &'static str {
        "Style/HashTransformValues"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ARRAY_NODE,
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            HASH_NODE,
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

        let method_name = call.name().as_slice();

        match method_name {
            b"each_with_object" => {
                self.check_each_with_object(source, &call, diagnostics);
            }
            b"[]" => {
                // Hash[x.map { |k, v| [k, expr(v)] }]
                self.check_hash_brackets_map(source, &call, diagnostics);
            }
            b"to_h" => {
                // Two sub-patterns:
                // 1. x.map { |k, v| [k, expr(v)] }.to_h  (call on a block result)
                // 2. x.to_h { |k, v| [k, expr(v)] }  (to_h with its own block)
                self.check_map_to_h(source, &call, diagnostics);
                self.check_to_h_with_block(source, &call, diagnostics);
            }
            _ => {}
        }
    }
}

impl HashTransformValues {
    /// Pattern 1: `x.each_with_object({}) { |(k, v), h| h[k] = expr(v) }`
    fn check_each_with_object(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Must have a block
        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Check receiver is not an array literal
        if is_array_receiver(call) {
            return;
        }

        // Argument must be an empty hash literal
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }
        if !is_empty_hash(&arg_list[0]) {
            return;
        }

        // Block params must be destructured: |(k, v), h|
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
        // First param must be destructured (MultiTargetNode) with exactly 2 targets
        let multi_target = match reqs[0].as_multi_target_node() {
            Some(mt) => mt,
            None => return,
        };
        let targets: Vec<_> = multi_target.lefts().iter().collect();
        if targets.len() != 2 {
            return;
        }

        // Extract key and value parameter names
        let key_param_name = match targets[0].as_required_parameter_node() {
            Some(p) => p.name(),
            None => return,
        };
        let value_param_name = match targets[1].as_required_parameter_node() {
            Some(p) => p.name(),
            None => return,
        };

        // Extract memo parameter name
        let memo_param_name = match reqs[1].as_required_parameter_node() {
            Some(p) => p.name(),
            None => return,
        };

        // Body must be a single statement: h[k] = expr(v)
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

        // Must be h[k] = expr pattern (CallNode with name []=)
        let assign_call = match body_nodes[0].as_call_node() {
            Some(c) => c,
            None => return,
        };
        if assign_call.name().as_slice() != b"[]=" {
            return;
        }

        // Receiver of []= must be the memo variable
        if let Some(recv) = assign_call.receiver() {
            if let Some(lvar) = recv.as_local_variable_read_node() {
                if lvar.name().as_slice() != memo_param_name.as_slice() {
                    return;
                }
            } else {
                return;
            }
        } else {
            return;
        }

        let assign_args = match assign_call.arguments() {
            Some(a) => a,
            None => return,
        };
        let aargs: Vec<_> = assign_args.arguments().iter().collect();
        if aargs.len() != 2 {
            return;
        }

        // Key argument must be a simple local variable matching the key param
        let key_lvar = match aargs[0].as_local_variable_read_node() {
            Some(l) => l,
            None => return,
        };
        if key_lvar.name().as_slice() != key_param_name.as_slice() {
            return;
        }

        // Value must NOT be a simple pass-through of the value param (noop check)
        if let Some(val_lvar) = aargs[1].as_local_variable_read_node() {
            if val_lvar.name().as_slice() == value_param_name.as_slice() {
                return; // noop: h[k] = v
            }
        }

        // Value expression must actually use the value parameter
        if !node_contains_lvar_read(&aargs[1], value_param_name.as_slice()) {
            return;
        }

        // Value expression must NOT reference the key variable
        if node_contains_lvar_read(&aargs[1], key_param_name.as_slice()) {
            return;
        }

        // Value expression must NOT reference the memo variable
        if node_contains_lvar_read(&aargs[1], memo_param_name.as_slice()) {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Prefer `transform_values` over `each_with_object`.".to_string(),
        ));
    }

    /// Pattern 2: `Hash[x.map { |k, v| [k, expr(v)] }]`
    fn check_hash_brackets_map(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Receiver must be `Hash` or `::Hash` constant
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        if !is_simple_constant(&recv, b"Hash") {
            return;
        }

        // Argument must be a single block expression: x.map { |k, v| [k, expr] }
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        // The argument should be a block_node wrapping a map/collect call
        // In Prism, `x.map { ... }` as an argument is a CallNode with a block
        let inner_call = match arg_list[0].as_call_node() {
            Some(c) => c,
            None => return,
        };

        let inner_method = inner_call.name().as_slice();
        if inner_method != b"map" && inner_method != b"collect" {
            return;
        }

        // Must have a block
        let block = match inner_call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Check receiver is not an array literal
        if is_array_receiver(&inner_call) {
            return;
        }

        // Validate block params and body as [k, expr(v)]
        if self.validate_map_block(source, &block_node) {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Prefer `transform_values` over `Hash[_.map {...}]`.".to_string(),
            ));
        }
    }

    /// Pattern 3: `x.map { |k, v| [k, expr(v)] }.to_h`
    fn check_map_to_h(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // The receiver of .to_h should be a map/collect call with a block
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let map_call = match recv.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let map_method = map_call.name().as_slice();
        if map_method != b"map" && map_method != b"collect" {
            return;
        }

        // Must have a block
        let block = match map_call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Check receiver is not an array literal
        if is_array_receiver(&map_call) {
            return;
        }

        // .to_h must NOT have arguments
        if call.arguments().is_some() {
            return;
        }

        // .to_h must NOT have its own block
        if call.block().is_some() {
            return;
        }

        if self.validate_map_block(source, &block_node) {
            // Report from the map call start through the .to_h
            let loc = map_call.location();
            let end_loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let _ = end_loc; // use call location for the full span
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Prefer `transform_values` over `map {...}.to_h`.".to_string(),
            ));
        }
    }

    /// Pattern 4: `x.to_h { |k, v| [k, expr(v)] }`
    fn check_to_h_with_block(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Must have a block
        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Check receiver is not an array literal
        if is_array_receiver(call) {
            return;
        }

        // Must NOT have arguments
        if call.arguments().is_some() {
            return;
        }

        if self.validate_map_block(source, &block_node) {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Prefer `transform_values` over `to_h {...}`.".to_string(),
            ));
        }
    }

    /// Validates a block for map/collect/to_h patterns:
    /// - Block params must be `|k, v|` (two required params)
    /// - Body must be `[k, expr(v)]` where k passes through unchanged
    ///   and expr(v) references v but not k, and is not a noop
    fn validate_map_block(
        &self,
        source: &SourceFile,
        block_node: &ruby_prism::BlockNode<'_>,
    ) -> bool {
        // Block params must be |k, v| (two simple params, NOT destructured)
        let params = match block_node.parameters() {
            Some(p) => p,
            None => return false,
        };
        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return false,
        };
        let bp_params = match block_params.parameters() {
            Some(p) => p,
            None => return false,
        };

        let reqs: Vec<_> = bp_params.requireds().iter().collect();
        if reqs.len() != 2 {
            return false;
        }

        // Both params must be simple required parameters (not destructured)
        let key_param_name = match reqs[0].as_required_parameter_node() {
            Some(p) => p.name(),
            None => return false,
        };
        let value_param_name = match reqs[1].as_required_parameter_node() {
            Some(p) => p.name(),
            None => return false,
        };

        // Body must be a single statement that's an array with 2 elements
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

        // First element must be the key param unchanged
        let key_elem = match elements[0].as_local_variable_read_node() {
            Some(l) => l,
            None => return false,
        };
        if key_elem.name().as_slice() != key_param_name.as_slice() {
            return false;
        }

        // Second element: the value expression
        // Must NOT be a noop (just passing v through)
        if let Some(val_lvar) = elements[1].as_local_variable_read_node() {
            if val_lvar.name().as_slice() == value_param_name.as_slice() {
                return false; // noop: [k, v]
            }
        }

        // Value expression must reference the value param
        if !node_contains_lvar_read(&elements[1], value_param_name.as_slice()) {
            return false;
        }

        // Value expression must NOT reference the key param (AST-based to avoid
        // false matches on symbols like `:name` or keyword args like `name:`)
        if node_contains_lvar_read(&elements[1], key_param_name.as_slice()) {
            return false;
        }

        let _ = source; // used for byte access if needed
        true
    }
}

/// Check if the receiver of a call is an array literal.
/// RuboCop's `!#array_receiver?` excludes array literals, `each_with_index`,
/// `with_index`, and `zip` receivers.
fn is_array_receiver(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(recv) = call.receiver() {
        if recv.as_array_node().is_some() {
            return true;
        }
        // Also check for each_with_index, with_index, zip receivers
        if let Some(recv_call) = recv.as_call_node() {
            let name = recv_call.name().as_slice();
            if name == b"each_with_index" || name == b"with_index" || name == b"zip" {
                return true;
            }
        }
    }
    false
}

/// Check if a node is an empty hash literal.
/// Handles both `as_hash_node` (`{}`) and `as_keyword_hash_node` (bare keyword args).
/// In practice, `each_with_object({})` always parses as a HashNode, but we handle
/// both for completeness and to satisfy the prism_pitfalls check.
fn is_empty_hash(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(hash) = node.as_hash_node() {
        let hash_src = hash.location().as_slice();
        let trimmed: Vec<u8> = hash_src
            .iter()
            .filter(|&&b| b != b' ' && b != b'{' && b != b'}')
            .copied()
            .collect();
        trimmed.is_empty()
    } else if let Some(kw_hash) = node.as_keyword_hash_node() {
        // KeywordHashNode with no elements is empty
        kw_hash.elements().iter().next().is_none()
    } else {
        false
    }
}

/// Check if a node's subtree contains a `LocalVariableReadNode` with the given name.
/// Unlike `contains_identifier`, this uses AST traversal so it won't match
/// symbols (`:name`), keyword arguments (`name:`), or other non-variable occurrences.
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
    crate::cop_fixture_tests!(HashTransformValues, "cops/style/hash_transform_values");
}
