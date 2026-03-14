use crate::cop::node_type::{
    CALL_NODE, CLASS_VARIABLE_READ_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, DEF_NODE,
    GLOBAL_VARIABLE_READ_NODE, INSTANCE_VARIABLE_READ_NODE, LOCAL_VARIABLE_READ_NODE,
    REQUIRED_PARAMETER_NODE, SELF_NODE, STATEMENTS_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/Delegate cop detects method definitions that simply delegate to another object,
/// suggesting the use of Rails' `delegate` macro instead.
///
/// ## Investigation findings (2026-03-10)
///
/// **FP root causes (49 FP):**
/// - Missing `module_function` check: RuboCop skips methods in modules that declare
///   `module_function`. Our cop was flagging these methods incorrectly.
/// - Missing `private :method_name` handling: The `is_private_or_protected` utility
///   only checked for standalone `private` keyword and inline `private def`, not
///   the `private :method_name` form that makes a specific method private after definition.
///
/// **FN root causes (136 FN):**
/// - Missing prefixed delegation detection: When `EnforceForPrefixed: true` (default),
///   `def bar_foo; bar.foo; end` should be flagged as a delegation that can use
///   `delegate :foo, to: :bar, prefix: true`. Our cop only matched exact method names.
///
/// **Fixes applied:**
/// - Added `module_function` detection via line scanning in enclosing scope
/// - Added `private :method_name` form detection
/// - Added prefixed delegation matching when `EnforceForPrefixed: true`
/// - Extended prefix skip (for `EnforceForPrefixed: false`) to all receiver types
///
/// ## Investigation (2026-03-14): FP=20
///
/// **FP root cause**: `is_in_module_function_scope` only scanned BACKWARDS from the def
/// for `module_function`. Patterns like `end; module_function :adapters` (inline after
/// the def's `end`) and `module_function :method_name` declared later in the module body
/// were missed. RuboCop's `module_function_declared?` walks ALL descendants of the
/// ancestor module — both before and after the def.
///
/// Fix: Added forward scan from the def line that checks if any subsequent line in the
/// same scope contains `module_function` (including `module_function :name` symbol forms).
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

        // Check method name matching:
        // 1. Direct match: def foo; bar.foo; end
        // 2. Prefixed match (when EnforceForPrefixed): def bar_foo; bar.foo; end
        let def_name = def_node.name().as_slice();
        let call_name = call.name().as_slice();

        // Must have a receiver (delegating to another object)
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let name_matches_directly = call_name == def_name;
        let name_matches_prefixed = if enforce_for_prefixed && !name_matches_directly {
            // Check if def_name == receiver_name + "_" + call_name
            // Skip prefix check for `self` receiver (RuboCop returns '' for self prefix)
            if receiver.as_self_node().is_some() {
                false
            } else {
                let recv_name = get_receiver_name(&receiver);
                if let Some(rn) = recv_name {
                    let mut expected = rn;
                    expected.push(b'_');
                    expected.extend_from_slice(call_name);
                    expected == def_name
                } else {
                    false
                }
            }
        } else {
            false
        };

        if !name_matches_directly && !name_matches_prefixed {
            return;
        }

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
        // Must check all receiver types, not just CallNode.
        if !enforce_for_prefixed && !name_matches_directly {
            // If the name only matched via prefix, skip it
            return;
        }

        // Skip private/protected methods — RuboCop only flags public methods.
        if crate::cop::util::is_private_or_protected(source, node.location().start_offset()) {
            return;
        }

        // Skip methods marked private via `private :method_name` after the def
        if is_private_symbol_arg(source, def_name, node.location().start_offset()) {
            return;
        }

        // Skip methods inside modules with `module_function` declared
        if is_in_module_function_scope(source, node.location().start_offset()) {
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

/// Extract the receiver name as bytes for prefix checking.
/// Returns None if the receiver type doesn't support prefix matching.
fn get_receiver_name(receiver: &ruby_prism::Node<'_>) -> Option<Vec<u8>> {
    if let Some(recv_call) = receiver.as_call_node() {
        if recv_call.receiver().is_none() {
            return Some(recv_call.name().as_slice().to_vec());
        }
    }
    if let Some(lv) = receiver.as_local_variable_read_node() {
        return Some(lv.name().as_slice().to_vec());
    }
    if let Some(iv) = receiver.as_instance_variable_read_node() {
        // ivar name includes @, e.g. @foo → prefix is "@foo"
        return Some(iv.name().as_slice().to_vec());
    }
    if let Some(cv) = receiver.as_class_variable_read_node() {
        return Some(cv.name().as_slice().to_vec());
    }
    if let Some(gv) = receiver.as_global_variable_read_node() {
        return Some(gv.name().as_slice().to_vec());
    }
    if let Some(cr) = receiver.as_constant_read_node() {
        return Some(cr.name().as_slice().to_vec());
    }
    if receiver.as_constant_path_node().is_some() {
        // For ConstantPathNode, extract source text
        let loc = receiver.location();
        return Some(loc.as_slice().to_vec());
    }
    None
}

/// Check if the method name appears as an argument to `private :method_name`
/// or `protected :method_name` after the method definition.
fn is_private_symbol_arg(source: &SourceFile, method_name: &[u8], def_offset: usize) -> bool {
    let (def_line, def_col) = source.offset_to_line_col(def_offset);
    let lines: Vec<&[u8]> = source.lines().collect();

    // Build the patterns: `private :method_name` and `protected :method_name`
    let mut private_pattern = b"private :".to_vec();
    private_pattern.extend_from_slice(method_name);
    let mut protected_pattern = b"protected :".to_vec();
    protected_pattern.extend_from_slice(method_name);

    // Search lines after the def for `private :method_name` or `protected :method_name`
    // Look within the same scope (stop at class/module boundary at lower indent).
    // `private :foo` typically appears right after the method's `end`, so we must
    // scan past `end` keywords at the same indent level.
    for line in lines.iter().skip(def_line) {
        let indent = line
            .iter()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count();
        let trimmed: Vec<u8> = line[indent..].to_vec();

        // Check for exact match or match followed by separator (newline, space, comma)
        for pattern in [&private_pattern, &protected_pattern] {
            if trimmed.starts_with(pattern) {
                let rest = &trimmed[pattern.len()..];
                if rest.is_empty()
                    || rest[0] == b'\n'
                    || rest[0] == b'\r'
                    || rest[0] == b' '
                    || rest[0] == b','
                    || rest[0] == b'#'
                {
                    return true;
                }
            }
        }

        // Stop at scope boundary (class/module at same or lower indent)
        if indent <= def_col && (trimmed.starts_with(b"class ") || trimmed.starts_with(b"module "))
        {
            break;
        }
    }
    false
}

/// Check if the def is inside a module that has `module_function` declared.
/// This matches RuboCop's `module_function_declared?` which checks ancestors
/// for any `module_function` call (both standalone and inline) — BEFORE OR AFTER
/// the def. The key difference from the original: we scan both backwards AND
/// forwards for `module_function :method_name` (with symbol arg, appearing after).
///
/// Patterns detected:
/// - Standalone `module_function` (makes all following methods module functions)
/// - `module_function def method_name` (inline on same line)
/// - `module_function :method_name` (applies to specific method, often after the def)
/// - `end; module_function :name` (inline after def's `end`)
fn is_in_module_function_scope(source: &SourceFile, def_offset: usize) -> bool {
    let (def_line, def_col) = source.offset_to_line_col(def_offset);
    let lines: Vec<&[u8]> = source.lines().collect();

    /// Check if a trimmed line is any module_function form.
    fn is_module_function_line(trimmed: &[u8]) -> bool {
        trimmed == b"module_function"
            || trimmed.starts_with(b"module_function\n")
            || trimmed.starts_with(b"module_function\r")
            || trimmed.starts_with(b"module_function ")
            || trimmed.starts_with(b"module_function#")
    }

    // Scan backwards from the def line looking for `module_function` at the same
    // or lower indentation within the same module scope.
    for line in lines[..def_line].iter().rev() {
        let indent = line
            .iter()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count();
        let trimmed: Vec<u8> = line[indent..].to_vec();

        if indent <= def_col && is_module_function_line(&trimmed) {
            return true;
        }

        // Stop at module/class boundary at lower indentation
        if indent < def_col && (trimmed.starts_with(b"module ") || trimmed.starts_with(b"class ")) {
            break;
        }
    }

    // Also check inline: the def line itself might have `module_function def foo`
    if let Some(line) = lines.get(def_line.saturating_sub(1)) {
        let trimmed: Vec<u8> = line
            .iter()
            .copied()
            .skip_while(|&b| b == b' ' || b == b'\t')
            .collect();
        if trimmed.starts_with(b"module_function def ") {
            return true;
        }
    }

    // RuboCop's `module_function_declared?` searches ALL descendants of the ancestor
    // module, including nodes that appear AFTER the def. Scan forward from the def's
    // line for any `module_function` reference, stopping at the enclosing scope boundary.
    //
    // This catches patterns like:
    //   `end; module_function :method_name`  (inline on same line as end)
    //   `module_function :method_name`        (after the def on its own line)
    for line in lines[def_line..].iter() {
        let indent = line
            .iter()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count();
        let trimmed: &[u8] = &line[indent..];

        // Stop at module/class boundary at lower indentation (scope ends)
        if indent < def_col && (trimmed.starts_with(b"module ") || trimmed.starts_with(b"class ")) {
            break;
        }

        // Check if this line contains `module_function` at all (as a word boundary).
        // Handles `module_function :name`, `end; module_function :name`, etc.
        if line
            .windows(b"module_function".len())
            .any(|w| w == b"module_function")
        {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Delegate, "cops/rails/delegate");
}
