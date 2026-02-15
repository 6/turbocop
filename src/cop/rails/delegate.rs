use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Delegate;

impl Cop for Delegate {
    fn name(&self) -> &'static str {
        "Rails/Delegate"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforce_for_prefixed = config.get_bool("EnforceForPrefixed", true);

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        // Skip class/module methods (def self.foo)
        if def_node.receiver().is_some() {
            return Vec::new();
        }

        // Collect parameter names (for argument forwarding check)
        let param_names: Vec<Vec<u8>> = if let Some(params) = def_node.parameters() {
            // Only support simple required positional parameters for forwarding
            let has_non_required = params.optionals().iter().next().is_some()
                || params.rest().is_some()
                || params.keywords().iter().next().is_some()
                || params.keyword_rest().is_some()
                || params.block().is_some();
            if has_non_required {
                return Vec::new();
            }
            params
                .requireds()
                .iter()
                .filter_map(|p| {
                    p.as_required_parameter_node()
                        .map(|rp| rp.name().as_slice().to_vec())
                })
                .collect()
        } else {
            Vec::new()
        };

        // Body must be a single call expression
        let body = match def_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.len() != 1 {
            return Vec::new();
        }

        let call = match body_nodes[0].as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // The delegated method name must match the defined method name
        // def foo; bar.foo; end → delegate :foo, to: :bar
        // def foo; bar.baz; end → NOT a delegation
        let def_name = def_node.name().as_slice();
        if call.name().as_slice() != def_name {
            return Vec::new();
        }

        // Must have a receiver (delegating to another object)
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Safe navigation (&.) is ignored — Rails' delegate with allow_nil
        // has different semantics than safe navigation
        if call.call_operator_loc().is_some_and(|op: ruby_prism::Location<'_>| op.as_slice() == b"&.") {
            return Vec::new();
        }

        // Receiver must be a delegatable target:
        // - Instance variable (@foo.bar → delegate :bar, to: :foo)
        // - Simple method/local variable (foo.bar → delegate :bar, to: :foo)
        // - Constant (Setting.bar → delegate :bar, to: :Setting)
        // NOT: literals, chained calls, self, etc.
        let is_delegatable_receiver = if receiver.as_instance_variable_read_node().is_some() {
            true
        } else if let Some(recv_call) = receiver.as_call_node() {
            // Simple receiverless method call (acts as a local variable)
            recv_call.receiver().is_none()
                && recv_call.arguments().is_none()
                && recv_call.block().is_none()
        } else if receiver.as_local_variable_read_node().is_some() {
            true
        } else if receiver.as_constant_read_node().is_some()
            || receiver.as_constant_path_node().is_some()
        {
            true
        } else {
            false
        };

        if !is_delegatable_receiver {
            return Vec::new();
        }

        // Check argument forwarding: call args must match def params 1:1
        let call_arg_names: Vec<Vec<u8>> = if let Some(args) = call.arguments() {
            args.arguments()
                .iter()
                .filter_map(|a| {
                    a.as_local_variable_read_node()
                        .map(|lv| lv.name().as_slice().to_vec())
                })
                .collect()
        } else {
            Vec::new()
        };

        // Argument count must match and all must be simple lvar forwards
        if call_arg_names.len() != param_names.len() {
            return Vec::new();
        }
        let call_arg_count = if let Some(args) = call.arguments() {
            args.arguments().iter().count()
        } else {
            0
        };
        if call_arg_count != param_names.len() {
            return Vec::new();
        }
        // Each param must forward to matching lvar in same order
        for (param, arg) in param_names.iter().zip(call_arg_names.iter()) {
            if param != arg {
                return Vec::new();
            }
        }

        // Should not have a block
        if call.block().is_some() {
            return Vec::new();
        }

        // When EnforceForPrefixed is false, skip prefixed delegations
        // (e.g., `def foo_bar; foo.bar; end` where method starts with receiver name)
        if !enforce_for_prefixed {
            if let Some(recv_call) = receiver.as_call_node() {
                let recv_name = recv_call.name().as_slice();
                let mut prefix = recv_name.to_vec();
                prefix.push(b'_');
                if def_name.starts_with(&prefix) {
                    return Vec::new();
                }
            }
        }

        // Skip private/protected methods — RuboCop only flags public methods.
        // Check if there's a `private` or `protected` declaration on the same line
        // or on a standalone line above this method.
        if is_private_or_protected(source, node.location().start_offset()) {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `delegate` to define delegations.".to_string(),
        )]
    }
}

/// Check if a method at the given offset is likely private or protected.
/// Looks for:
/// - `private def foo` (inline) on the same line
/// - Standalone `private` or `protected` on any preceding line (without a subsequent `public`)
fn is_private_or_protected(source: &SourceFile, def_offset: usize) -> bool {
    let bytes = source.as_bytes();
    let (def_line, _) = source.offset_to_line_col(def_offset);

    // Check inline: the same line might start with `private ` or `protected `
    let mut line_start = def_offset;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    let line_to_def = &bytes[line_start..def_offset];
    let trimmed = line_to_def.iter().copied().skip_while(|&b| b == b' ' || b == b'\t').collect::<Vec<u8>>();
    if trimmed.starts_with(b"private ") || trimmed.starts_with(b"protected ") {
        return true;
    }

    // Check preceding lines for standalone `private` or `protected`
    let lines: Vec<&[u8]> = source.lines().collect();
    let mut in_private = false;
    for line_idx in 0..def_line.saturating_sub(1) {
        let line = lines[line_idx];
        let trimmed: Vec<u8> = line.iter().copied().skip_while(|&b| b == b' ' || b == b'\t').collect();
        if trimmed == b"private" || trimmed.starts_with(b"private ") || trimmed.starts_with(b"private\n") {
            in_private = true;
        } else if trimmed == b"protected" || trimmed.starts_with(b"protected ") || trimmed.starts_with(b"protected\n") {
            in_private = true;
        } else if trimmed == b"public" || trimmed.starts_with(b"public ") || trimmed.starts_with(b"public\n") {
            in_private = false;
        }
    }

    in_private
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Delegate, "cops/rails/delegate");
}
