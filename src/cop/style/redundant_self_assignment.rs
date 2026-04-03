use crate::cop::shared::node_type::{
    CALL_NODE, CLASS_VARIABLE_READ_NODE, CLASS_VARIABLE_WRITE_NODE, GLOBAL_VARIABLE_READ_NODE,
    GLOBAL_VARIABLE_WRITE_NODE, INSTANCE_VARIABLE_READ_NODE, INSTANCE_VARIABLE_WRITE_NODE,
    LOCAL_VARIABLE_READ_NODE, LOCAL_VARIABLE_WRITE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks for redundant self-assignments where an in-place modification method's
/// return value is assigned back to the same variable.
///
/// ## Investigation notes (2026-03)
///
/// **Root cause of FPs (15):** nitrocop's method list included bang methods that
/// return `nil` when no modification occurs (`sub!`, `gsub!`, `chomp!`, `chop!`,
/// `strip!`, `lstrip!`, `rstrip!`, `squeeze!`, `tr!`, `tr_s!`, `delete!`,
/// `downcase!`, `upcase!`, `swapcase!`, `capitalize!`, `encode!`,
/// `unicode_normalize!`, `scrub!`, `compact!`, `flatten!`, `uniq!`, `reject!`,
/// `select!`, `filter!`, `collect_concat!`, `flat_map!`, `slice!`). For these
/// methods, `x = x.method!(...)` captures a potentially different value (`nil`)
/// so the assignment is NOT redundant. RuboCop correctly excludes them.
///
/// **Root cause of FNs (48):** nitrocop only handled local variable writes.
/// RuboCop also handles instance variables (`@foo = @foo.concat(...)`), class
/// variables (`@@foo = @@foo.concat(...)`), global variables (`$foo = $foo.concat(...)`),
/// and attribute assignments (`other.foo = other.foo.concat(...)`).
/// Also missing methods from RuboCop's list: `compare_by_identity`, `fill`,
/// `initialize_copy`, `insert`, `rehash`, `unshift`.
///
/// **Fix:** Replaced method list with exact RuboCop `METHODS_RETURNING_SELF` set.
/// Added ivar/cvar/gvar write+read handling. Added attribute assignment detection
/// via `CallNode` with `name.ends_with(b"=")`.
///
/// **FP fix (2026-03, corpus FP=1):** Attribute assignments where the RHS method
/// call has a block (e.g., `config.roles = config.roles.transform_values! { ... }`)
/// were incorrectly flagged. RuboCop's `redundant_self_assignment?` node matcher
/// for `on_send` expects `(call (call %1 %2) ...)` as the argument, which doesn't
/// match block-wrapped sends in the Parser gem AST (blocks wrap sends as
/// `(block (send ...) ...)`). In Prism, blocks are stored in `CallNode.block()`,
/// so we explicitly skip attribute assignments when `rhs_call.block().is_some()`.
/// Note: variable assignments (`on_lvasgn`) DO handle blocks in RuboCop (line 61
/// checks `rhs.type?(:any_block, :call)`), so we only skip blocks for Pattern 2.
pub struct RedundantSelfAssignment;

/// Methods that always return `self` (never `nil`), matching RuboCop's
/// `METHODS_RETURNING_SELF`. Methods like `sub!`, `gsub!`, `compact!` etc.
/// are intentionally excluded because they return `nil` when no change occurs.
const METHODS_RETURNING_SELF: &[&[u8]] = &[
    b"append",
    b"clear",
    b"collect!",
    b"compare_by_identity",
    b"concat",
    b"delete_if",
    b"fill",
    b"initialize_copy",
    b"insert",
    b"keep_if",
    b"map!",
    b"merge!",
    b"prepend",
    b"push",
    b"rehash",
    b"replace",
    b"reverse!",
    b"rotate!",
    b"shuffle!",
    b"sort!",
    b"sort_by!",
    b"transform_keys!",
    b"transform_values!",
    b"unshift",
    b"update",
];

fn method_returning_self(name: &[u8]) -> bool {
    METHODS_RETURNING_SELF.contains(&name)
}

/// Check if a CallNode is an assignment method call (e.g., `other.foo=`).
fn is_assignment_call(call: &ruby_prism::CallNode<'_>) -> bool {
    let name = call.name().as_slice();
    name.ends_with(b"=") && name != b"==" && name != b"!=" && name != b"==="
}

/// Extract the variable name from a write node (lvar, ivar, cvar, gvar).
fn get_write_var_name<'a>(node: &'a ruby_prism::Node<'a>) -> Option<(&'a [u8], u8)> {
    if let Some(w) = node.as_local_variable_write_node() {
        return Some((w.name().as_slice(), LOCAL_VARIABLE_READ_NODE));
    }
    if let Some(w) = node.as_instance_variable_write_node() {
        return Some((w.name().as_slice(), INSTANCE_VARIABLE_READ_NODE));
    }
    if let Some(w) = node.as_class_variable_write_node() {
        return Some((w.name().as_slice(), CLASS_VARIABLE_READ_NODE));
    }
    if let Some(w) = node.as_global_variable_write_node() {
        return Some((w.name().as_slice(), GLOBAL_VARIABLE_READ_NODE));
    }
    None
}

/// Get the value being assigned in a write node.
fn get_write_value<'a>(node: &'a ruby_prism::Node<'a>) -> Option<ruby_prism::Node<'a>> {
    if let Some(w) = node.as_local_variable_write_node() {
        return Some(w.value());
    }
    if let Some(w) = node.as_instance_variable_write_node() {
        return Some(w.value());
    }
    if let Some(w) = node.as_class_variable_write_node() {
        return Some(w.value());
    }
    if let Some(w) = node.as_global_variable_write_node() {
        return Some(w.value());
    }
    None
}

/// Check if a node is a variable read of the expected type and name.
fn is_matching_read(node: &ruby_prism::Node<'_>, expected_type: u8, var_name: &[u8]) -> bool {
    if expected_type == LOCAL_VARIABLE_READ_NODE {
        node.as_local_variable_read_node()
            .is_some_and(|r| r.name().as_slice() == var_name)
    } else if expected_type == INSTANCE_VARIABLE_READ_NODE {
        node.as_instance_variable_read_node()
            .is_some_and(|r| r.name().as_slice() == var_name)
    } else if expected_type == CLASS_VARIABLE_READ_NODE {
        node.as_class_variable_read_node()
            .is_some_and(|r| r.name().as_slice() == var_name)
    } else if expected_type == GLOBAL_VARIABLE_READ_NODE {
        node.as_global_variable_read_node()
            .is_some_and(|r| r.name().as_slice() == var_name)
    } else {
        false
    }
}

/// Compare two nodes for structural equality (for attribute assignment receivers).
fn nodes_equal(a: &ruby_prism::Node<'_>, b: &ruby_prism::Node<'_>, source: &[u8]) -> bool {
    // Simple byte comparison of source text
    let a_loc = a.location();
    let b_loc = b.location();
    let a_slice = &source[a_loc.start_offset()..a_loc.end_offset()];
    let b_slice = &source[b_loc.start_offset()..b_loc.end_offset()];
    a_slice == b_slice
}

impl Cop for RedundantSelfAssignment {
    fn name(&self) -> &'static str {
        "Style/RedundantSelfAssignment"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            LOCAL_VARIABLE_READ_NODE,
            LOCAL_VARIABLE_WRITE_NODE,
            INSTANCE_VARIABLE_READ_NODE,
            INSTANCE_VARIABLE_WRITE_NODE,
            CLASS_VARIABLE_READ_NODE,
            CLASS_VARIABLE_WRITE_NODE,
            GLOBAL_VARIABLE_READ_NODE,
            GLOBAL_VARIABLE_WRITE_NODE,
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
        // Pattern 1: Variable assignment — x = x.method (lvar, ivar, cvar, gvar)
        if let Some((var_name, read_type)) = get_write_var_name(node) {
            if let Some(value) = get_write_value(node) {
                if let Some(call) = value.as_call_node() {
                    if let Some(receiver) = call.receiver() {
                        if is_matching_read(&receiver, read_type, var_name) {
                            let method_name = call.name().as_slice();
                            if method_returning_self(method_name) {
                                let loc = node.location();
                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                diagnostics.push(self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    format!(
                                        "Redundant self-assignment. `{}` modifies `{}` in place.",
                                        String::from_utf8_lossy(method_name),
                                        String::from_utf8_lossy(var_name),
                                    ),
                                ));
                                return;
                            }
                        }
                    }
                }
            }
            return;
        }

        // Pattern 2: Attribute assignment — other.foo = other.foo.method(...)
        // This is a CallNode with name ending in `=` (e.g., `foo=`)
        if let Some(call) = node.as_call_node() {
            if !is_assignment_call(&call) {
                return;
            }

            // Skip self.foo = ... (RuboCop doesn't flag these)
            let lhs_receiver = match call.receiver() {
                Some(r) => r,
                None => return,
            };
            if lhs_receiver.as_self_node().is_some() {
                return;
            }

            // Get the attribute name being assigned (strip trailing `=`)
            let assign_name = call.name().as_slice();
            let attr_name = &assign_name[..assign_name.len() - 1];

            // The RHS is the first (only) argument
            let args = match call.arguments() {
                Some(a) => a,
                None => return,
            };
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() != 1 {
                return;
            }
            let rhs = &arg_list[0];

            // RHS must be a method call
            let rhs_call = match rhs.as_call_node() {
                Some(c) => c,
                None => return,
            };

            let method_name = rhs_call.name().as_slice();
            if !method_returning_self(method_name) {
                return;
            }

            // Skip when the RHS method call has a block — RuboCop's node matcher
            // for attribute assignment doesn't match block-wrapped sends.
            if rhs_call.block().is_some() {
                return;
            }

            // RHS receiver must be a method call on the same object with the same attribute
            let rhs_receiver = match rhs_call.receiver() {
                Some(r) => r,
                None => return,
            };

            // rhs_receiver should be `other.foo` or `other&.foo`
            let rhs_recv_call = match rhs_receiver.as_call_node() {
                Some(c) => c,
                None => return,
            };

            // Check attribute name matches
            if rhs_recv_call.name().as_slice() != attr_name {
                return;
            }

            // Check the object receiver matches
            let rhs_obj = match rhs_recv_call.receiver() {
                Some(r) => r,
                None => return,
            };

            if nodes_equal(&lhs_receiver, &rhs_obj, source.as_bytes()) {
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Redundant self-assignment. `{}` modifies `{}` in place.",
                        String::from_utf8_lossy(method_name),
                        String::from_utf8_lossy(attr_name),
                    ),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantSelfAssignment,
        "cops/style/redundant_self_assignment"
    );
}
