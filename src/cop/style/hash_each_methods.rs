use ruby_prism::Visit;

use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/HashEachMethods: suggest `each_key`/`each_value` instead of `keys.each`/`values.each`
/// or `each { |k, _v| }` / `each { |_k, v| }`.
///
/// FP=125 root cause: nitrocop was flagging `.each` calls that had method arguments
/// (e.g., `Resque::Failure.each(0, count, queue) { |_, item| }`). RuboCop's NodePattern
/// `(call _ :each)` only matches when `.each` has no arguments — just a block. Fixed by
/// checking `call.arguments().is_none()` before entering the unused-block-arg path.
///
/// Additional FP fixes:
/// - `keys.each(&block)` / `values.each(&method(:x))`: RuboCop's `kv_each_with_block_pass`
///   only matches `(block_pass (sym _))` — symbol-to-proc. Non-symbol block_pass args like
///   `&blk`, `&block`, `&method(:x)` are skipped. Fixed by checking that block_pass expression
///   is a symbol node before flagging.
/// - `hash.keys.each { |k| hash[k] = ... }`: RuboCop's `handleable?` skips when the hash
///   receiver is mutated with `[]=` inside the block. Fixed by walking the block body to detect
///   `[]=` calls on the root receiver.
/// - `.each { |k, _v| use(_v) }`: RuboCop checks actual lvar usage in the body, not just `_`
///   prefix. A `_`-prefixed param that IS referenced in the body is not considered unused.
///   Fixed by walking the block body for `LocalVariableReadNode` matching the param name.
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

        let _allowed_receivers = config.get_string_array("AllowedReceivers");

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
                // If the .each call has a block_pass (e.g., `keys.each(&blk)`),
                // RuboCop only flags it when the block_pass wraps a symbol
                // (`keys.each(&:to_s)`). Skip non-symbol block_pass args like
                // `&block`, `&blk`, `&method(:x)`.
                if let Some(block) = call.block() {
                    if let Some(block_arg) = block.as_block_argument_node() {
                        let is_symbol = block_arg
                            .expression()
                            .is_some_and(|e| e.as_symbol_node().is_some());
                        if !is_symbol {
                            return;
                        }
                    }
                }

                // RuboCop's `handleable?` checks for hash mutation (`[]=`) on the
                // root receiver inside the block body. For patterns like
                // `hash.keys.each { |k| hash[k] = ... }`, the iteration is done
                // over keys specifically to allow safe mutation, so skip these.
                if let Some(root_recv) = recv_call.receiver() {
                    if block_mutates_receiver(&call, &root_recv) {
                        return;
                    }
                }

                let is_keys = recv_method == b"keys";
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
                return;
            }
        }

        // Pattern 2: hash.each { |k, _unused_v| ... } — unused block arg
        self.check_each_block(source, &call, config, diagnostics);
    }
}

impl HashEachMethods {
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
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if call.name().as_slice() != b"each" {
            return;
        }

        // Must have a receiver
        if call.receiver().is_none() {
            return;
        }

        // .each must have no arguments (only a block). Calls like
        // `.each(0, count)` are not Hash#each and should be skipped.
        if call.arguments().is_some() {
            return;
        }

        // Receiver must not be a hash/array literal
        if let Some(recv) = call.receiver() {
            if recv.as_array_node().is_some() {
                return;
            }
        }

        // Must NOT be `keys.each` or `values.each` (handled above)
        if let Some(recv) = call.receiver() {
            if let Some(rc) = recv.as_call_node() {
                let name = rc.name().as_slice();
                if name == b"keys" || name == b"values" {
                    return;
                }
            }
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
        let requireds: Vec<_> = params_node.requireds().iter().collect();
        if requireds.len() != 2 {
            return;
        }

        let key_param = match requireds[0].as_required_parameter_node() {
            Some(p) => p,
            None => return,
        };
        let value_param = match requireds[1].as_required_parameter_node() {
            Some(p) => p,
            None => return,
        };

        let key_name = key_param.name().as_slice();
        let value_name = value_param.name().as_slice();

        // RuboCop checks actual lvar usage in the block body, not just `_` prefix.
        // A `_`-prefixed param that IS referenced in the body is not considered unused.
        let body = match block_node.body() {
            Some(b) => b,
            None => return, // empty block body — RuboCop skips (nil body)
        };
        let key_unused = !body_references_lvar(&body, key_name);
        let value_unused = !body_references_lvar(&body, value_name);

        // Both unused — skip (RuboCop skips too)
        if key_unused && value_unused {
            return;
        }
        // Neither unused — skip
        if !key_unused && !value_unused {
            return;
        }

        let unused_code = if value_unused {
            std::str::from_utf8(value_name).unwrap_or("_")
        } else {
            std::str::from_utf8(key_name).unwrap_or("_")
        };

        let replacement = if value_unused {
            "each_key"
        } else {
            "each_value"
        };

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        let _ = config.get_string_array("AllowedReceivers");

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
fn body_references_lvar(body: &ruby_prism::Node<'_>, name: &[u8]) -> bool {
    let mut finder = LvarReferenceFinder { found: false, name };
    finder.visit(body);
    finder.found
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
