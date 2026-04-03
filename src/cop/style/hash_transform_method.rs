//! Shared infrastructure for Style/HashTransformKeys and Style/HashTransformValues.
//!
//! Mirrors RuboCop's `HashTransformMethod` mixin. Both cops detect four patterns
//! that can be replaced by `transform_keys` or `transform_values`:
//! 1. `each_with_object({}) { |(k, v), h| h[expr] = identity }`
//! 2. `Hash[_.map { |k, v| [expr, identity] }]`
//! 3. `_.map { |k, v| [expr, identity] }.to_h`
//! 4. `_.to_h { |k, v| [expr, identity] }`
//!
//! The only difference is which array position (0=key, 1=value) is the
//! "identity" (passes through unchanged) vs the "transform" (is modified).

use crate::cop::Cop;
use crate::cop::shared::constant_predicates;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Which hash transformation to suggest.
#[derive(Clone, Copy)]
pub enum TransformMode {
    Keys,
    Values,
}

impl TransformMode {
    fn method_name(self) -> &'static str {
        match self {
            Self::Keys => "transform_keys",
            Self::Values => "transform_values",
        }
    }

    /// Index of the array element that must pass through unchanged.
    fn identity_index(self) -> usize {
        match self {
            Self::Keys => 1,   // value passes through
            Self::Values => 0, // key passes through
        }
    }

    /// Index of the array element that is transformed.
    fn transform_index(self) -> usize {
        match self {
            Self::Keys => 0,   // key is transformed
            Self::Values => 1, // value is transformed
        }
    }
}

/// Shared check_node implementation for both HashTransformKeys and HashTransformValues.
pub fn check_hash_transform(
    cop: &dyn Cop,
    mode: TransformMode,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return,
    };

    let call_name = call.name().as_slice();

    if call_name == b"each_with_object" {
        check_each_with_object(cop, mode, source, &call, diagnostics);
    } else if call_name == b"[]" {
        check_hash_brackets_map(cop, mode, source, &call, diagnostics);
    } else if call_name == b"to_h" {
        check_map_to_h(cop, mode, source, &call, diagnostics);
        check_to_h_with_block(cop, mode, source, &call, diagnostics);
    }
}

// ── Pattern checks ─────────────────────────────────────────────────────

/// Pattern 1: `each_with_object({}) { |(k, v), h| h[key_or_identity] = val_or_identity }`
fn check_each_with_object(
    cop: &dyn Cop,
    mode: TransformMode,
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let block = match call.block().and_then(|b| b.as_block_node()) {
        Some(b) => b,
        None => return,
    };

    if is_array_receiver(call) {
        return;
    }

    // Argument must be a single empty hash
    let args = match call.arguments() {
        Some(a) => a,
        None => return,
    };
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.len() != 1 || !is_empty_hash(&arg_list[0]) {
        return;
    }

    // Block params must be destructured: |(k, v), h|
    let params = match block
        .parameters()
        .and_then(|p| p.as_block_parameters_node())
    {
        Some(bp) => bp,
        None => return,
    };
    let bp_params = match params.parameters() {
        Some(p) => p,
        None => return,
    };

    let reqs: Vec<_> = bp_params.requireds().iter().collect();
    if reqs.len() != 2 {
        return;
    }

    // First param must be destructured (MultiTargetNode) with exactly 2 targets, no rest
    let multi_target = match reqs[0].as_multi_target_node() {
        Some(mt) => mt,
        None => return,
    };
    if multi_target.rest().is_some() {
        return;
    }
    let targets: Vec<_> = multi_target.lefts().iter().collect();
    if targets.len() != 2 {
        return;
    }

    let key_param_name = match targets[0].as_required_parameter_node() {
        Some(p) => p.name(),
        None => return,
    };
    let value_param_name = match targets[1].as_required_parameter_node() {
        Some(p) => p.name(),
        None => return,
    };
    let memo_param_name = match reqs[1].as_required_parameter_node() {
        Some(p) => p.name(),
        None => return,
    };

    let param_names = [key_param_name.as_slice(), value_param_name.as_slice()];
    let identity_param = param_names[mode.identity_index()];
    let transform_param = param_names[mode.transform_index()];

    // Body must be a single `h[...] = ...` statement
    let body = match block.body().and_then(|b| b.as_statements_node()) {
        Some(s) => s,
        None => return,
    };
    let body_nodes: Vec<_> = body.body().iter().collect();
    if body_nodes.len() != 1 {
        return;
    }

    let assign_call = match body_nodes[0].as_call_node() {
        Some(c) if c.name().as_slice() == b"[]=" => c,
        _ => return,
    };

    // Receiver must be the memo variable
    let receiver_ok = assign_call
        .receiver()
        .and_then(|r| r.as_local_variable_read_node())
        .is_some_and(|lv| lv.name().as_slice() == memo_param_name.as_slice());
    if !receiver_ok {
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

    // For each_with_object: aargs[0] is the index key, aargs[1] is the assigned value.
    // The "identity" side (Keys: aargs[1], Values: aargs[0]) must be the corresponding param.
    // The "transform" side must be transformed (not noop), use its param, not the other or memo.
    let identity_arg = &aargs[mode.identity_index()];
    let transform_arg = &aargs[mode.transform_index()];

    // Identity arg must be a simple read of the identity param
    let identity_ok = identity_arg
        .as_local_variable_read_node()
        .is_some_and(|lv| lv.name().as_slice() == identity_param);
    if !identity_ok {
        return;
    }

    // Transform arg must NOT be a noop (simple read of transform param)
    if transform_arg
        .as_local_variable_read_node()
        .is_some_and(|lv| lv.name().as_slice() == transform_param)
    {
        return;
    }

    // Must contain a reference to the transform param
    if !node_contains_lvar_read(transform_arg, transform_param) {
        return;
    }

    // Must NOT reference the identity param or memo param
    if node_contains_lvar_read(transform_arg, identity_param) {
        return;
    }
    if node_contains_lvar_read(transform_arg, memo_param_name.as_slice()) {
        return;
    }

    let method = mode.method_name();
    let loc = call.location();
    let (line, column) = source.offset_to_line_col(loc.start_offset());
    diagnostics.push(cop.diagnostic(
        source,
        line,
        column,
        format!("Prefer `{method}` over `each_with_object`."),
    ));
}

/// Pattern 2: `Hash[_.map { |k, v| [...] }]`
fn check_hash_brackets_map(
    cop: &dyn Cop,
    mode: TransformMode,
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let receiver = match call.receiver() {
        Some(r) => r,
        None => return,
    };
    if !constant_predicates::is_simple_constant(&receiver, b"Hash") {
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

    let map_call = match arg_list[0].as_call_node() {
        Some(c) => c,
        None => return,
    };
    let map_name = map_call.name().as_slice();
    if map_name != b"map" && map_name != b"collect" {
        return;
    }

    let block_node = match map_call.block().and_then(|b| b.as_block_node()) {
        Some(b) => b,
        None => return,
    };

    if is_array_receiver(&map_call) {
        return;
    }

    if validate_transform_block(mode, &block_node) {
        let method = mode.method_name();
        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            format!("Prefer `{method}` over `Hash[_.map {{...}}]`."),
        ));
    }
}

/// Pattern 3: `_.map { |k, v| [...] }.to_h`
fn check_map_to_h(
    cop: &dyn Cop,
    mode: TransformMode,
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let receiver = match call.receiver() {
        Some(r) => r,
        None => return,
    };

    let map_call = match receiver.as_call_node() {
        Some(c) => c,
        None => return,
    };
    let map_name = map_call.name().as_slice();
    if map_name != b"map" && map_name != b"collect" {
        return;
    }

    if call.arguments().is_some() || call.block().is_some() {
        return;
    }

    let block_node = match map_call.block().and_then(|b| b.as_block_node()) {
        Some(b) => b,
        None => return,
    };

    if is_array_receiver(&map_call) {
        return;
    }

    if validate_transform_block(mode, &block_node) {
        let method = mode.method_name();
        let loc = map_call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            format!("Prefer `{method}` over `map {{...}}.to_h`."),
        ));
    }
}

/// Pattern 4: `_.to_h { |k, v| [...] }`
fn check_to_h_with_block(
    cop: &dyn Cop,
    mode: TransformMode,
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if call.arguments().is_some() || is_array_receiver(call) {
        return;
    }

    let block_node = match call.block().and_then(|b| b.as_block_node()) {
        Some(b) => b,
        None => return,
    };

    if validate_transform_block(mode, &block_node) {
        let method = mode.method_name();
        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            format!("Prefer `{method}` over `to_h {{...}}`."),
        ));
    }
}

// ── Block validation ───────────────────────────────────────────────────

/// Validate a block for map/collect/to_h patterns:
/// - Block params must be `|k, v|` (two required params, not destructured)
/// - Body must be `[elem0, elem1]` where the identity position matches
///   its param unchanged and the transform position is modified.
fn validate_transform_block(mode: TransformMode, block_node: &ruby_prism::BlockNode<'_>) -> bool {
    let params = match block_node
        .parameters()
        .and_then(|p| p.as_block_parameters_node())
    {
        Some(bp) => bp,
        None => return false,
    };
    let bp_params = match params.parameters() {
        Some(p) => p,
        None => return false,
    };
    let reqs: Vec<_> = bp_params.requireds().iter().collect();
    if reqs.len() != 2 {
        return false;
    }

    let key_param = match reqs[0].as_required_parameter_node() {
        Some(p) => p,
        None => return false,
    };
    let val_param = match reqs[1].as_required_parameter_node() {
        Some(p) => p,
        None => return false,
    };

    // Body must be a single ArrayNode with exactly 2 elements
    let body = match block_node.body().and_then(|b| b.as_statements_node()) {
        Some(s) => s,
        None => return false,
    };
    let body_nodes: Vec<_> = body.body().iter().collect();
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

    let param_names = [key_param.name().as_slice(), val_param.name().as_slice()];
    let identity_param = param_names[mode.identity_index()];
    let transform_param = param_names[mode.transform_index()];

    // Identity element must be a simple local variable read of the identity param
    let identity_ok = elements[mode.identity_index()]
        .as_local_variable_read_node()
        .is_some_and(|lv| lv.name().as_slice() == identity_param);
    if !identity_ok {
        return false;
    }

    let transform_elem = &elements[mode.transform_index()];

    // Transform element must NOT be a noop (simple read of transform param)
    if transform_elem
        .as_local_variable_read_node()
        .is_some_and(|lv| lv.name().as_slice() == transform_param)
    {
        return false;
    }

    // Must contain the transform param
    if !node_contains_lvar_read(transform_elem, transform_param) {
        return false;
    }

    // Must NOT contain the identity param
    if node_contains_lvar_read(transform_elem, identity_param) {
        return false;
    }

    true
}

// ── Shared helpers ─────────────────────────────────────────────────────

/// Check if the receiver is an array literal or array-like method
/// (each_with_index, with_index, zip).
fn is_array_receiver(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(receiver) = call.receiver() {
        if receiver.as_array_node().is_some() {
            return true;
        }
        if let Some(receiver_call) = receiver.as_call_node() {
            let name = receiver_call.name().as_slice();
            if name == b"each_with_index" || name == b"with_index" || name == b"zip" {
                return true;
            }
        }
    }
    false
}

/// Check if a node is an empty hash literal.
fn is_empty_hash(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(hash) = node.as_hash_node() {
        let hash_src = hash.location().as_slice();
        let trimmed: Vec<u8> = hash_src
            .iter()
            .filter(|&&b| b != b' ' && b != b'{' && b != b'}')
            .copied()
            .collect();
        trimmed.is_empty()
    } else if let Some(keyword_hash) = node.as_keyword_hash_node() {
        keyword_hash.elements().iter().next().is_none()
    } else {
        false
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
