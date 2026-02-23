use crate::cop::node_type::{
    CALL_NODE, CLASS_VARIABLE_READ_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, DEF_NODE,
    GLOBAL_VARIABLE_READ_NODE, INSTANCE_VARIABLE_READ_NODE, LOCAL_VARIABLE_READ_NODE,
    REQUIRED_PARAMETER_NODE, SELF_NODE, STATEMENTS_NODE,
};
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

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CLASS_VARIABLE_READ_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            DEF_NODE,
            GLOBAL_VARIABLE_READ_NODE,
            INSTANCE_VARIABLE_READ_NODE,
            LOCAL_VARIABLE_READ_NODE,
            REQUIRED_PARAMETER_NODE,
            SELF_NODE,
            STATEMENTS_NODE,
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
        let enforce_for_prefixed = config.get_bool("EnforceForPrefixed", true);

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        // Skip class/module methods (def self.foo)
        if def_node.receiver().is_some() {
            return;
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
                return;
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

        let call = match body_nodes[0].as_call_node() {
            Some(c) => c,
            None => return,
        };

        // The delegated method name must match the defined method name
        // def foo; bar.foo; end → delegate :foo, to: :bar
        // def foo; bar.baz; end → NOT a delegation
        let def_name = def_node.name().as_slice();
        if call.name().as_slice() != def_name {
            return;
        }

        // Must have a receiver (delegating to another object)
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Safe navigation (&.) is ignored — Rails' delegate with allow_nil
        // has different semantics than safe navigation
        if call
            .call_operator_loc()
            .is_some_and(|op: ruby_prism::Location<'_>| op.as_slice() == b"&.")
        {
            return;
        }

        // Receiver must be a delegatable target:
        // - Instance variable (@foo.bar → delegate :bar, to: :foo)
        // - Simple method/local variable (foo.bar → delegate :bar, to: :foo)
        // - Constant (Setting.bar → delegate :bar, to: :Setting)
        // - self (self.bar → delegate :bar, to: :self)
        // - self.class (self.class.bar → delegate :bar, to: :class)
        // - Class/global variable (@@var.bar, $var.bar)
        // NOT: literals, arbitrary chained calls, etc.
        let is_delegatable_receiver = if receiver.as_instance_variable_read_node().is_some()
            || receiver.as_self_node().is_some()
            || receiver.as_class_variable_read_node().is_some()
            || receiver.as_global_variable_read_node().is_some()
        {
            true
        } else if let Some(recv_call) = receiver.as_call_node() {
            // self.class → delegate to :class
            if recv_call.name().as_slice() == b"class"
                && recv_call
                    .receiver()
                    .is_some_and(|r| r.as_self_node().is_some())
                && recv_call.arguments().is_none()
            {
                true
            } else {
                // Simple receiverless method call (acts as a local variable)
                recv_call.receiver().is_none()
                    && recv_call.arguments().is_none()
                    && recv_call.block().is_none()
            }
        } else if receiver.as_local_variable_read_node().is_some() {
            true
        } else {
            receiver.as_constant_read_node().is_some() || receiver.as_constant_path_node().is_some()
        };

        if !is_delegatable_receiver {
            return;
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
            return;
        }
        let call_arg_count = if let Some(args) = call.arguments() {
            args.arguments().iter().count()
        } else {
            0
        };
        if call_arg_count != param_names.len() {
            return;
        }
        // Each param must forward to matching lvar in same order
        for (param, arg) in param_names.iter().zip(call_arg_names.iter()) {
            if param != arg {
                return;
            }
        }

        // Should not have a block
        if call.block().is_some() {
            return;
        }

        // When EnforceForPrefixed is false, skip prefixed delegations
        // (e.g., `def foo_bar; foo.bar; end` where method starts with receiver name)
        if !enforce_for_prefixed {
            if let Some(recv_call) = receiver.as_call_node() {
                let recv_name = recv_call.name().as_slice();
                let mut prefix = recv_name.to_vec();
                prefix.push(b'_');
                if def_name.starts_with(&prefix) {
                    return;
                }
            }
        }

        // Skip private/protected methods — RuboCop only flags public methods.
        // Check if there's a `private` or `protected` declaration on the same line
        // or on a standalone line above this method.
        if is_private_or_protected(source, node.location().start_offset()) {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `delegate` to define delegations.".to_string(),
        ));
    }
}

/// Check if a method at the given offset is likely private or protected.
/// Looks for:
/// - `private def foo` (inline) on the same line
/// - Standalone `private` or `protected` on any preceding line at the SAME indentation
///   scope (without a subsequent `public`)
fn is_private_or_protected(source: &SourceFile, def_offset: usize) -> bool {
    let bytes = source.as_bytes();
    let (def_line, def_col) = source.offset_to_line_col(def_offset);

    // Check inline: the same line might start with `private ` or `protected `
    let mut line_start = def_offset;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    let line_to_def = &bytes[line_start..def_offset];
    let trimmed = line_to_def
        .iter()
        .copied()
        .skip_while(|&b| b == b' ' || b == b'\t')
        .collect::<Vec<u8>>();
    if trimmed.starts_with(b"private ") || trimmed.starts_with(b"protected ") {
        return true;
    }

    // Check preceding lines for standalone `private` or `protected`.
    // Only consider lines at the same indentation level as the def.
    // When we see `class`, `module`, or `end` at lower indentation, reset state
    // (those indicate scope boundaries).
    let lines: Vec<&[u8]> = source.lines().collect();
    let mut in_private = false;
    for line in &lines[..def_line] {
        let indent = line
            .iter()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count();
        let trimmed: Vec<u8> = line[indent..].to_vec();

        // Scope boundary: class/module at same or lower indent resets private state.
        // `end` only resets at STRICTLY lower indent — method `end` keywords share
        // the same indent as `private`/`def` and must not reset the state.
        if indent <= def_col && (trimmed.starts_with(b"class ") || trimmed.starts_with(b"module "))
        {
            in_private = false;
        }
        if indent < def_col
            && (trimmed == b"end"
                || trimmed.starts_with(b"end ")
                || trimmed.starts_with(b"end\n")
                || trimmed.starts_with(b"end\r"))
        {
            in_private = false;
        }

        // Only consider private/protected/public at the same indent level
        if indent == def_col {
            if trimmed == b"private"
                || trimmed.starts_with(b"private\n")
                || trimmed.starts_with(b"private\r")
                || trimmed.starts_with(b"private #")
                || trimmed == b"protected"
                || trimmed.starts_with(b"protected\n")
                || trimmed.starts_with(b"protected\r")
                || trimmed.starts_with(b"protected #")
            {
                in_private = true;
            } else if trimmed == b"public"
                || trimmed.starts_with(b"public\n")
                || trimmed.starts_with(b"public\r")
                || trimmed.starts_with(b"public #")
            {
                in_private = false;
            }
        }
    }

    in_private
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Delegate, "cops/rails/delegate");
}
