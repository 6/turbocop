use ruby_prism::Visit;

use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

const ARRAY_CONVERTER_METHODS: &[&[u8]] = &[
    b"assoc", b"chunk", b"flatten", b"rassoc", b"sort", b"sort_by", b"to_a",
];

/// Style/HashEachMethods mirrors RuboCop's narrow `handleable?` rules.
///
/// This cop now skips two recurring FP classes from the corpus:
/// `keys.each` / `values.each` calls where the block belongs to a later chain
/// (`keys.each.with_index`, `q.seplist hash.keys.each`) and `each { |k, v| }`
/// calls whose receiver was already converted to an array (`to_a.each`,
/// `sort.each`, `sort_by { ... }.each`). It also uses the true root receiver
/// for `[]=` mutation checks so `summary.urls.keys.each { summary.urls[...] = ... }`
/// still registers like RuboCop, while direct hash mutation on the root receiver
/// remains exempt.
///
/// For the existing FN fixture cases, it now accepts RuboCop's broader block
/// parameter shapes: destructured `MultiTargetNode`s such as
/// `|(_root_id, _instance_id), value|`, optional parameters like `options={}`,
/// and mixed destructured/value pairs such as `|line_num, (range, _last_col, meta)|`.
///
/// Fixed 12 FN and 5 FP:
/// - FN: Optional params (`options={}`, `key = name`) are always considered unused,
///   matching RuboCop's `source.delete_prefix('*')` check against lvar sources.
/// - FN: Block params (`&block`) are always considered unused and now included in
///   the positional params list via `ParametersNode.block()`.
/// - FN: Anonymous rest (`*`) is always considered unused.
/// - FN: Bare `keys.each` (variable, not `foo.keys.each`) no longer skipped by the
///   keys/values receiver guard in `check_each_block`.
/// - FP: `ImplicitRestNode` (trailing comma `|_m,|`) is excluded from the params
///   list, matching RuboCop's parser which doesn't count it as a separate arg.
/// - FP: Single bare-lvar body (`{ |k, v| k }`) now matches RuboCop's
///   `each_descendant(:lvar)` quirk where the body node itself is excluded from
///   lvar search, making both args "unused" and skipping the offense.
pub struct HashEachMethods;

impl Cop for HashEachMethods {
    fn name(&self) -> &'static str {
        "Style/HashEachMethods"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_bytes = call.name().as_slice();

        if method_bytes != b"each" {
            return;
        }

        let allowed_receivers = config
            .get_string_array("AllowedReceivers")
            .unwrap_or_default();

        // Must have a receiver
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Pattern 1: hash.keys.each / hash.values.each
        if let Some(recv_call) = receiver.as_call_node() {
            let recv_method = recv_call.name().as_slice();
            if (recv_method == b"keys" || recv_method == b"values")
                && recv_call.receiver().is_some()
                && recv_call.arguments().is_none()
            {
                self.check_kv_each(source, &call, &recv_call, &allowed_receivers, diagnostics);
                return;
            }
        }

        // Pattern 2: hash.each { |k, _unused_v| ... } — unused block arg
        self.check_each_block(source, &call, diagnostics);
    }
}

impl HashEachMethods {
    fn check_kv_each(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        recv_call: &ruby_prism::CallNode<'_>,
        allowed_receivers: &[String],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // RuboCop only registers when the block is attached to `each` itself
        // or when the block-pass is a symbol proc (`&:foo`).
        if !has_supported_kv_block(call) {
            return;
        }

        let Some(parent_receiver) = recv_call.receiver() else {
            return;
        };
        if is_allowed_receiver(&parent_receiver, allowed_receivers) {
            return;
        }
        let Some(root_recv) = root_receiver(parent_receiver) else {
            return;
        };
        if !is_handleable_root(&root_recv) || block_mutates_receiver(call, &root_recv) {
            return;
        }

        let is_keys = recv_call.name().as_slice() == b"keys";
        let replacement = if is_keys { "each_key" } else { "each_value" };
        let original = if is_keys { "keys.each" } else { "values.each" };

        let has_safe_nav = call
            .call_operator_loc()
            .is_some_and(|op| op.as_slice() == b"&.");
        let recv_has_safe_nav = recv_call
            .call_operator_loc()
            .is_some_and(|op| op.as_slice() == b"&.");

        let display_original = if has_safe_nav || recv_has_safe_nav {
            if is_keys {
                "keys&.each"
            } else {
                "values&.each"
            }
        } else {
            original
        };

        let msg_loc = recv_call
            .message_loc()
            .unwrap_or_else(|| recv_call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());

        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{}` instead of `{}`.", replacement, display_original),
        ));
    }

    /// Check `.each { |k, v| ... }` blocks where one argument is unused.
    /// RuboCop checks actual lvar usage in the body, not just `_` prefix.
    ///
    /// FP fix: RuboCop only matches `(call _ :each)` with no method arguments.
    /// Calls like `Failure.each(0, count, queue) { |_, item| }` pass arguments
    /// to `.each` and must be skipped — they are not Hash#each calls.
    fn check_each_block(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if call.name().as_slice() != b"each" {
            return;
        }

        let receiver = match call.receiver() {
            Some(receiver) => receiver,
            None => return,
        };

        // .each must have no arguments (only a block). Calls like
        // `.each(0, count)` are not Hash#each and should be skipped.
        if call.arguments().is_some() {
            return;
        }

        // Must NOT be `foo.keys.each` or `foo.values.each` (handled above). Also skip
        // array-converter receivers like `to_a.each` / `sort.each`.
        // Only skip when the keys/values call has its own receiver (e.g., `foo.keys`),
        // not bare `keys.each` where `keys` is a variable or method.
        if let Some(recv_call) = receiver.as_call_node() {
            let name = recv_call.name().as_slice();
            if recv_call.receiver().is_some()
                && (name == b"keys" || name == b"values" || is_array_converter_method(name))
            {
                return;
            }
            if recv_call.receiver().is_none() && is_array_converter_method(name) {
                return;
            }
        }

        let Some(root_recv) = root_receiver(receiver) else {
            return;
        };
        if !is_handleable_root(&root_recv) || block_mutates_receiver(call, &root_recv) {
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

        // Block must have exactly 2 parameters
        let params = match block_node.parameters() {
            Some(p) => p,
            None => return,
        };
        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return,
        };
        let params_node = match block_params.parameters() {
            Some(p) => p,
            None => return,
        };
        let params = positional_block_params(&params_node);
        if params.len() != 2 {
            return;
        }

        // RuboCop checks actual lvar usage in the block body, not just `_` prefix.
        // A `_`-prefixed param that IS referenced in the body is not considered unused.
        let body = match block_node.body() {
            Some(b) => b,
            None => return, // empty block body — RuboCop skips (nil body)
        };
        let key_unused = parameter_unused(&body, &params[0]);
        let value_unused = parameter_unused(&body, &params[1]);

        // Both unused — skip (RuboCop skips too)
        if key_unused && value_unused {
            return;
        }
        // Neither unused — skip
        if !key_unused && !value_unused {
            return;
        }

        let unused_code = if value_unused {
            parameter_display(&params[1])
        } else {
            parameter_display(&params[0])
        };

        let replacement = if value_unused {
            "each_key"
        } else {
            "each_value"
        };

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{replacement}` instead of `each` and remove the unused `{unused_code}` block argument."),
        ));
    }
}

/// Check if a block body references a local variable by name.
/// Used to determine actual usage vs. just `_` prefix convention.
///
/// Matches RuboCop's `each_descendant(:lvar)` behavior: in RuboCop (Parser gem),
/// a single-expression body IS the expression node itself, and `each_descendant`
/// does not include the node — so a bare lvar body like `{ |k, v| k }` finds no
/// lvar descendants and both args are considered unused (→ skip). In Prism, bodies
/// are always wrapped in StatementsNode, so we must explicitly exclude a lone
/// top-level bare lvar that matches the target name.
fn body_references_lvar(body: &ruby_prism::Node<'_>, name: &[u8]) -> bool {
    if let Some(stmts) = body.as_statements_node() {
        let children: Vec<_> = stmts.body().iter().collect();
        if children.len() == 1 {
            if let Some(lvar) = children[0].as_local_variable_read_node() {
                if lvar.name().as_slice() == name {
                    return false;
                }
            }
        }
    }

    let mut finder = LvarReferenceFinder { found: false, name };
    finder.visit(body);
    finder.found
}

fn parameter_unused(body: &ruby_prism::Node<'_>, param: &ruby_prism::Node<'_>) -> bool {
    // RuboCop checks `block_arg.source.delete_prefix('*')` against lvar sources.
    // Optional params (source "key = val"), block params (source "&block"), and
    // anonymous rest (source "*" → empty after delete_prefix) never match an lvar,
    // so RuboCop always considers them unused regardless of body content.
    if param.as_optional_parameter_node().is_some() {
        return true;
    }
    if param.as_block_parameter_node().is_some() {
        return true;
    }
    if let Some(rest) = param.as_rest_parameter_node() {
        if rest.name().is_none() {
            return true;
        }
    }

    let names = parameter_names(param);
    !names.is_empty()
        && names
            .iter()
            .all(|name| !body_references_lvar(body, name.as_slice()))
}

fn parameter_names(param: &ruby_prism::Node<'_>) -> Vec<Vec<u8>> {
    let mut names = Vec::new();
    collect_parameter_names(param, &mut names);
    names
}

fn collect_parameter_names(param: &ruby_prism::Node<'_>, names: &mut Vec<Vec<u8>>) {
    if let Some(required) = param.as_required_parameter_node() {
        names.push(required.name().as_slice().to_vec());
        return;
    }
    if let Some(optional) = param.as_optional_parameter_node() {
        names.push(optional.name().as_slice().to_vec());
        return;
    }
    if let Some(rest) = param.as_rest_parameter_node() {
        if let Some(name) = rest.name() {
            names.push(name.as_slice().to_vec());
        }
        return;
    }
    if let Some(multi_target) = param.as_multi_target_node() {
        for child in multi_target.lefts().iter() {
            collect_parameter_names(&child, names);
        }
        if let Some(rest) = multi_target.rest() {
            collect_parameter_names(&rest, names);
        }
        for child in multi_target.rights().iter() {
            collect_parameter_names(&child, names);
        }
    }
}

fn parameter_display(param: &ruby_prism::Node<'_>) -> String {
    if let Some(required) = param.as_required_parameter_node() {
        return std::str::from_utf8(required.name().as_slice())
            .unwrap_or("_")
            .to_string();
    }
    if let Some(rest) = param.as_rest_parameter_node() {
        if let Some(name) = rest.name() {
            return std::str::from_utf8(name.as_slice())
                .unwrap_or("_")
                .to_string();
        }
    }

    std::str::from_utf8(param.location().as_slice())
        .unwrap_or("_")
        .to_string()
}

fn positional_block_params<'pr>(
    params_node: &ruby_prism::ParametersNode<'pr>,
) -> Vec<ruby_prism::Node<'pr>> {
    if params_node.keyword_rest().is_some() {
        return Vec::new();
    }
    if params_node.keywords().iter().next().is_some() {
        return Vec::new();
    }

    let mut params = Vec::new();
    params.extend(params_node.requireds().iter());
    params.extend(params_node.optionals().iter());
    if let Some(rest) = params_node.rest() {
        // Skip ImplicitRestNode (trailing comma like |_m,|) — RuboCop's parser
        // does not create a separate parameter for it.
        if rest.as_implicit_rest_node().is_none() {
            params.push(rest);
        }
    }
    params.extend(params_node.posts().iter());
    // Include block parameter (&block) — RuboCop's `each_arguments` matcher
    // captures any two args regardless of type, including blockarg.
    if let Some(block) = params_node.block() {
        params.push(block.as_node());
    }
    params
}

/// Visitor that searches for `LocalVariableReadNode` matching a given name.
struct LvarReferenceFinder<'a> {
    found: bool,
    name: &'a [u8],
}

impl<'pr> Visit<'pr> for LvarReferenceFinder<'_> {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        if node.name().as_slice() == self.name {
            self.found = true;
        }
    }
}

/// Check if the block body of a `.keys.each` / `.values.each` call mutates
/// the root receiver with `[]=`. RuboCop's `handleable?` skips these cases.
fn block_mutates_receiver(
    call: &ruby_prism::CallNode<'_>,
    root_recv: &ruby_prism::Node<'_>,
) -> bool {
    // Get the block body
    let block = match call.block() {
        Some(b) => b,
        None => return false,
    };
    let block_node = match block.as_block_node() {
        Some(b) => b,
        None => return false,
    };
    let body = match block_node.body() {
        Some(b) => b,
        None => return false,
    };

    // Extract the receiver's source bytes for comparison
    let recv_source = root_recv.location().as_slice();

    let mut finder = BracketAssignFinder {
        found: false,
        recv_source,
    };
    finder.visit(&body);
    finder.found
}

fn has_supported_kv_block(call: &ruby_prism::CallNode<'_>) -> bool {
    let Some(block) = call.block() else {
        return false;
    };
    if block.as_block_node().is_some() {
        return true;
    }

    block
        .as_block_argument_node()
        .and_then(|block_arg| block_arg.expression())
        .is_some_and(|expr| expr.as_symbol_node().is_some())
}

fn root_receiver(node: ruby_prism::Node<'_>) -> Option<ruby_prism::Node<'_>> {
    if let Some(call) = node.as_call_node() {
        if let Some(receiver) = call.receiver() {
            if receiver
                .as_call_node()
                .is_some_and(|receiver_call| receiver_call.receiver().is_some())
            {
                return root_receiver(receiver);
            }
            return Some(receiver);
        }
    }

    Some(node)
}

fn is_array_converter_method(name: &[u8]) -> bool {
    ARRAY_CONVERTER_METHODS.contains(&name)
}

fn is_handleable_root(node: &ruby_prism::Node<'_>) -> bool {
    !is_literal(node) || node.as_hash_node().is_some() || node.as_keyword_hash_node().is_some()
}

fn is_literal(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_keyword_hash_node().is_some() {
        return true;
    }

    matches!(
        node,
        ruby_prism::Node::TrueNode { .. }
            | ruby_prism::Node::FalseNode { .. }
            | ruby_prism::Node::NilNode { .. }
            | ruby_prism::Node::IntegerNode { .. }
            | ruby_prism::Node::FloatNode { .. }
            | ruby_prism::Node::RationalNode { .. }
            | ruby_prism::Node::ImaginaryNode { .. }
            | ruby_prism::Node::StringNode { .. }
            | ruby_prism::Node::SymbolNode { .. }
            | ruby_prism::Node::RegularExpressionNode { .. }
            | ruby_prism::Node::ArrayNode { .. }
            | ruby_prism::Node::HashNode { .. }
            | ruby_prism::Node::RangeNode { .. }
            | ruby_prism::Node::InterpolatedStringNode { .. }
            | ruby_prism::Node::InterpolatedSymbolNode { .. }
            | ruby_prism::Node::InterpolatedRegularExpressionNode { .. }
            | ruby_prism::Node::XStringNode { .. }
            | ruby_prism::Node::InterpolatedXStringNode { .. }
    )
}

fn is_allowed_receiver(receiver: &ruby_prism::Node<'_>, allowed_receivers: &[String]) -> bool {
    if allowed_receivers.is_empty() {
        return false;
    }

    let recv_name = receiver_name(receiver);
    allowed_receivers.contains(&recv_name)
}

fn receiver_name(node: &ruby_prism::Node<'_>) -> String {
    if let Some(call) = node.as_call_node() {
        if let Some(receiver) = call.receiver() {
            if receiver.as_constant_read_node().is_some()
                || receiver.as_constant_path_node().is_some()
            {
                let const_src = std::str::from_utf8(receiver.location().as_slice()).unwrap_or("");
                let method = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
                return format!("{const_src}.{method}");
            }
            return receiver_name(&receiver);
        }

        return std::str::from_utf8(call.name().as_slice())
            .unwrap_or("")
            .to_string();
    }

    std::str::from_utf8(node.location().as_slice())
        .unwrap_or("")
        .to_string()
}

/// Visitor that searches for `[]=` calls on a receiver whose source text
/// matches the root receiver of the `keys.each` / `values.each` chain.
struct BracketAssignFinder<'a> {
    found: bool,
    recv_source: &'a [u8],
}

impl<'pr> Visit<'pr> for BracketAssignFinder<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() == b"[]=" {
            if let Some(recv) = node.receiver() {
                // Compare source text of the `[]=` receiver with the root
                // receiver of the `keys.each` chain. For patterns like
                // `hash.keys.each { |k| hash[k] = ... }`, both are `hash`.
                if recv.location().as_slice() == self.recv_source {
                    self.found = true;
                }
            }
        }
        // Continue visiting children
        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashEachMethods, "cops/style/hash_each_methods");
}
