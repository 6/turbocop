use crate::cop::node_type::{
    BLOCK_NODE, BLOCK_PARAMETERS_NODE, BREAK_NODE, CALL_NODE, CASE_NODE, ELSE_NODE,
    EMBEDDED_STATEMENTS_NODE, IF_NODE, INTERPOLATED_STRING_NODE, LOCAL_VARIABLE_READ_NODE,
    NEXT_NODE, NUMBERED_PARAMETERS_NODE, PARENTHESES_NODE, REQUIRED_PARAMETER_NODE,
    STATEMENTS_NODE, UNLESS_NODE, WHEN_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Looks for `reduce` or `inject` blocks where the value returned (implicitly or
/// explicitly) does not include the accumulator. A block is considered valid as
/// long as at least one return value includes the accumulator.
///
/// Also catches instances where an index of the accumulator is returned, as
/// this may change the type of object being retained.
///
/// FP fix: `next <accumulator>` in any branch makes element-only returns in
/// other branches acceptable (matches RuboCop's `returns_accumulator_anywhere?`).
///
/// FN fix: Added accumulator index detection (`acc[foo]` and `acc[foo] = bar`)
/// matching RuboCop's `accumulator_index?` / MSG_INDEX pattern.
pub struct UnmodifiedReduceAccumulator;

impl Cop for UnmodifiedReduceAccumulator {
    fn name(&self) -> &'static str {
        "Lint/UnmodifiedReduceAccumulator"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            BREAK_NODE,
            CALL_NODE,
            CASE_NODE,
            ELSE_NODE,
            EMBEDDED_STATEMENTS_NODE,
            IF_NODE,
            INTERPOLATED_STRING_NODE,
            LOCAL_VARIABLE_READ_NODE,
            NEXT_NODE,
            NUMBERED_PARAMETERS_NODE,
            PARENTHESES_NODE,
            REQUIRED_PARAMETER_NODE,
            STATEMENTS_NODE,
            UNLESS_NODE,
            WHEN_NODE,
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
        if method_name != b"reduce" && method_name != b"inject" {
            return;
        }

        let method_str = std::str::from_utf8(method_name).unwrap_or("reduce");

        // Must have a block
        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return,
            },
            None => return,
        };

        // Get block parameters
        let params = match block.parameters() {
            Some(p) => p,
            None => return, // No block params
        };

        let (acc_name, el_name) = match extract_reduce_params(&params) {
            Some(names) => names,
            None => return,
        };

        // Get block body
        let body = match block.body() {
            Some(b) => b,
            None => return,
        };

        // Get the statements in the body
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let body_stmts: Vec<ruby_prism::Node<'_>> = stmts.body().iter().collect();
        if body_stmts.is_empty() {
            return;
        }

        // Check each return point (last expression, next, break)
        check_return_values(
            self,
            source,
            &body_stmts,
            &acc_name,
            &el_name,
            method_str,
            diagnostics,
        );
    }
}

fn extract_reduce_params(params_node: &ruby_prism::Node<'_>) -> Option<(String, String)> {
    if let Some(block_params) = params_node.as_block_parameters_node() {
        let inner = block_params.parameters()?;
        let requireds: Vec<ruby_prism::Node<'_>> = inner.requireds().iter().collect();

        if requireds.len() < 2 {
            return None;
        }

        // Check for splat argument
        if inner.rest().is_some() {
            return None;
        }

        let acc = requireds[0].as_required_parameter_node().map(|p| {
            std::str::from_utf8(p.name().as_slice())
                .unwrap_or("")
                .to_string()
        })?;
        let el = requireds[1].as_required_parameter_node().map(|p| {
            std::str::from_utf8(p.name().as_slice())
                .unwrap_or("")
                .to_string()
        });

        // The element might be a destructuring pattern
        let el_name = match el {
            Some(name) => name,
            None => {
                // Could be a MultiTargetNode for destructured args like |(el, index)|
                // Just use a placeholder
                return None; // We need at least a simple element name
            }
        };

        if acc.is_empty() || el_name.is_empty() {
            return None;
        }

        Some((acc, el_name))
    } else if let Some(numbered) = params_node.as_numbered_parameters_node() {
        if numbered.maximum() >= 2 {
            Some(("_1".to_string(), "_2".to_string()))
        } else {
            None
        }
    } else {
        None
    }
}

/// Analyzed info about a return value in a reduce block.
struct ReturnInfo {
    /// Byte offset start for diagnostic reporting
    start_offset: usize,
    /// Whether this return value uses the accumulator (lvar_used? match)
    uses_acc: bool,
    /// Whether this return value is an accumulator index access (acc[foo])
    is_acc_index: bool,
    /// Whether this return value is element-only (flaggable)
    is_element_only: bool,
}

fn check_return_values(
    cop: &UnmodifiedReduceAccumulator,
    source: &SourceFile,
    stmts: &[ruby_prism::Node<'_>],
    acc_name: &str,
    el_name: &str,
    method_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Collect ALL return values with their analysis: last expression + next/break arguments.
    // This matches RuboCop's return_values method which collects all return points first,
    // then checks if ANY return value references the accumulator before flagging.
    let mut return_infos: Vec<ReturnInfo> = Vec::new();

    // Collect next/break arguments from all statements (including nested conditionals)
    for stmt in stmts {
        collect_next_break_infos(stmt, acc_name, el_name, &mut return_infos);
    }

    // Add the last expression as implicit return (unless it's a next/break)
    if let Some(last) = stmts.last() {
        if last.as_next_node().is_none() && last.as_break_node().is_none() {
            return_infos.push(analyze_return_value(last, acc_name, el_name));
        }
    }

    if return_infos.is_empty() {
        return;
    }

    // Phase 1: Check for accumulator index returns (always an offense)
    // acc[foo] or acc[foo] = bar (except acc[el])
    for ri in &return_infos {
        if ri.is_acc_index {
            let (line, column) = source.offset_to_line_col(ri.start_offset);
            diagnostics.push(cop.diagnostic(
                source,
                line,
                column,
                format!(
                    "Do not return an element of the accumulator in `{}`.",
                    method_name
                ),
            ));
            return; // RuboCop only reports the first accumulator index offense
        }
    }

    // Phase 2: Check if element is modified in the body
    if element_modified_in_body(stmts, el_name) {
        return;
    }

    // Phase 3: Check if ANY return value references the accumulator
    if return_infos.iter().any(|ri| ri.uses_acc) {
        return;
    }

    // Phase 4: Flag individual return values that are element-only
    for ri in &return_infos {
        if ri.is_element_only {
            let (line, column) = source.offset_to_line_col(ri.start_offset);
            diagnostics.push(cop.diagnostic(
                source,
                line,
                column,
                format!(
                    "Ensure the accumulator `{}` will be modified by `{}`.",
                    acc_name, method_name
                ),
            ));
        }
    }
}

/// Analyze a single return value node and produce a ReturnInfo.
fn analyze_return_value(node: &ruby_prism::Node<'_>, acc_name: &str, el_name: &str) -> ReturnInfo {
    ReturnInfo {
        start_offset: node.location().start_offset(),
        uses_acc: lvar_used(node, acc_name),
        is_acc_index: is_accumulator_index(node, acc_name, el_name),
        is_element_only: !references_var(node, acc_name)
            && is_only_element_expr(node, acc_name, el_name),
    }
}

/// Collect next/break argument return infos from a node tree,
/// recursing into conditionals but NOT into inner blocks.
fn collect_next_break_infos(
    node: &ruby_prism::Node<'_>,
    acc_name: &str,
    el_name: &str,
    infos: &mut Vec<ReturnInfo>,
) {
    if let Some(next) = node.as_next_node() {
        if let Some(args) = next.arguments() {
            for arg in args.arguments().iter() {
                infos.push(analyze_return_value(&arg, acc_name, el_name));
            }
        }
        return;
    }

    if let Some(brk) = node.as_break_node() {
        if let Some(args) = brk.arguments() {
            for arg in args.arguments().iter() {
                infos.push(analyze_return_value(&arg, acc_name, el_name));
            }
        }
        return;
    }

    // Recurse into conditionals
    if let Some(if_node) = node.as_if_node() {
        if let Some(body) = if_node.statements() {
            for stmt in body.body().iter() {
                collect_next_break_infos(&stmt, acc_name, el_name, infos);
            }
        }
        if let Some(else_clause) = if_node.subsequent() {
            collect_next_break_infos(&else_clause, acc_name, el_name, infos);
        }
    }

    if let Some(unless_node) = node.as_unless_node() {
        if let Some(body) = unless_node.statements() {
            for stmt in body.body().iter() {
                collect_next_break_infos(&stmt, acc_name, el_name, infos);
            }
        }
    }

    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            for stmt in stmts.body().iter() {
                collect_next_break_infos(&stmt, acc_name, el_name, infos);
            }
        }
    }

    if let Some(case_node) = node.as_case_node() {
        for condition in case_node.conditions().iter() {
            if let Some(when_node) = condition.as_when_node() {
                if let Some(body) = when_node.statements() {
                    for stmt in body.body().iter() {
                        collect_next_break_infos(&stmt, acc_name, el_name, infos);
                    }
                }
            }
        }
        if let Some(else_clause) = case_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for stmt in stmts.body().iter() {
                    collect_next_break_infos(&stmt, acc_name, el_name, infos);
                }
            }
        }
    }
}

/// Check if a node is an accumulator index access: `acc[foo]` or `acc[foo] = bar`
/// Returns false for `acc[el]` (element used as index key).
fn is_accumulator_index(node: &ruby_prism::Node<'_>, acc_name: &str, el_name: &str) -> bool {
    if let Some(call) = node.as_call_node() {
        let method = call.name().as_slice();
        if method == b"[]" || method == b"[]=" {
            if let Some(recv) = call.receiver() {
                if let Some(read) = recv.as_local_variable_read_node() {
                    let name = std::str::from_utf8(read.name().as_slice()).unwrap_or("");
                    if name == acc_name {
                        // acc[el] = ... is always an offense
                        if method == b"[]=" {
                            return true;
                        }
                        // For acc[], check if any argument uses the element
                        if let Some(args) = call.arguments() {
                            let has_el = args.arguments().iter().any(|a| lvar_used(&a, el_name));
                            return !has_el;
                        }
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Check if the element variable is modified in the block body
/// (assigned, op-assigned, or mutated with other variables).
/// Matches RuboCop's element_modified? node search.
fn element_modified_in_body(stmts: &[ruby_prism::Node<'_>], el_name: &str) -> bool {
    for stmt in stmts {
        if element_modified_recursive(stmt, el_name) {
            return true;
        }
    }
    false
}

fn element_modified_recursive(node: &ruby_prism::Node<'_>, el_name: &str) -> bool {
    // el = ...
    if let Some(write) = node.as_local_variable_write_node() {
        let name = std::str::from_utf8(write.name().as_slice()).unwrap_or("");
        if name == el_name {
            return true;
        }
    }
    // el += ...
    if let Some(op_write) = node.as_local_variable_operator_write_node() {
        let name = std::str::from_utf8(op_write.name().as_slice()).unwrap_or("");
        if name == el_name {
            return true;
        }
    }
    // el ||= ...
    if let Some(or_write) = node.as_local_variable_or_write_node() {
        let name = std::str::from_utf8(or_write.name().as_slice()).unwrap_or("");
        if name == el_name {
            return true;
        }
    }
    // el &&= ...
    if let Some(and_write) = node.as_local_variable_and_write_node() {
        let name = std::str::from_utf8(and_write.name().as_slice()).unwrap_or("");
        if name == el_name {
            return true;
        }
    }

    // el.method(arg, ...) where args contain any local variable
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            if let Some(read) = recv.as_local_variable_read_node() {
                let name = std::str::from_utf8(read.name().as_slice()).unwrap_or("");
                if name == el_name {
                    if let Some(args) = call.arguments() {
                        for arg in args.arguments().iter() {
                            if has_any_local_var(&arg) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        // method(acc, foo, el) — bare method with el and other vars
        if call.receiver().is_none() {
            if let Some(args) = call.arguments() {
                let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
                let has_el = arg_list.iter().any(|a| references_var(a, el_name));
                let has_other = arg_list.iter().any(|a| has_any_local_var(a));
                if has_el && has_other {
                    return true;
                }
            }
        }
    }

    // Recurse into children via known container types (avoid inner blocks)
    if node.as_block_node().is_some() {
        return false;
    }

    if let Some(if_node) = node.as_if_node() {
        if let Some(body) = if_node.statements() {
            for stmt in body.body().iter() {
                if element_modified_recursive(&stmt, el_name) {
                    return true;
                }
            }
        }
        if let Some(sub) = if_node.subsequent() {
            if element_modified_recursive(&sub, el_name) {
                return true;
            }
        }
    }

    if let Some(unless_node) = node.as_unless_node() {
        if let Some(body) = unless_node.statements() {
            for stmt in body.body().iter() {
                if element_modified_recursive(&stmt, el_name) {
                    return true;
                }
            }
        }
    }

    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            for stmt in stmts.body().iter() {
                if element_modified_recursive(&stmt, el_name) {
                    return true;
                }
            }
        }
    }

    if let Some(stmts_node) = node.as_statements_node() {
        for stmt in stmts_node.body().iter() {
            if element_modified_recursive(&stmt, el_name) {
                return true;
            }
        }
    }

    false
}

/// Check if a node matches RuboCop's lvar_used? pattern (shallow, top-level only):
/// - `(lvar %1)` — bare variable read
/// - `(lvasgn %1 ...)` — variable assignment
/// - `(send (lvar %1) :<< ...)` — shovel operator
/// - `(dstr (begin (lvar %1)))` — interpolation
///
/// Does NOT match op/or/and-assignment due to NodePattern arity mismatch.
fn lvar_used(node: &ruby_prism::Node<'_>, var_name: &str) -> bool {
    // Direct variable read
    if let Some(read) = node.as_local_variable_read_node() {
        if std::str::from_utf8(read.name().as_slice()).unwrap_or("") == var_name {
            return true;
        }
    }
    // Variable assignment (lvasgn)
    if let Some(write) = node.as_local_variable_write_node() {
        if std::str::from_utf8(write.name().as_slice()).unwrap_or("") == var_name {
            return true;
        }
    }
    // acc << ... (shovel)
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"<<" {
            if let Some(recv) = call.receiver() {
                if let Some(read) = recv.as_local_variable_read_node() {
                    if std::str::from_utf8(read.name().as_slice()).unwrap_or("") == var_name {
                        return true;
                    }
                }
            }
        }
    }
    // Interpolation: "#{var}"
    if let Some(interp) = node.as_interpolated_string_node() {
        for part in interp.parts().iter() {
            if let Some(embedded) = part.as_embedded_statements_node() {
                if let Some(stmts) = embedded.statements() {
                    for stmt in stmts.body().iter() {
                        if let Some(read) = stmt.as_local_variable_read_node() {
                            if std::str::from_utf8(read.name().as_slice()).unwrap_or("") == var_name
                            {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    // NOTE: RuboCop's lvar_used? does NOT match op-assignment (acc += ...),
    // or-assignment (acc ||= ...), or and-assignment (acc &&= ...) due to
    // NodePattern arity mismatch with SHORTHAND_ASSIGNMENTS. These are 3-child
    // nodes but the pattern expects only 1 child: (%SHORTHAND_ASSIGNMENTS (lvasgn %1)).
    false
}

/// Check if an expression references a variable by name (deep recursive check).
fn references_var(node: &ruby_prism::Node<'_>, var_name: &str) -> bool {
    if let Some(read) = node.as_local_variable_read_node() {
        if std::str::from_utf8(read.name().as_slice()).unwrap_or("") == var_name {
            return true;
        }
    }

    // Check in compound expressions
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            if references_var(&recv, var_name) {
                return true;
            }
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if references_var(&arg, var_name) {
                    return true;
                }
            }
        }
    }

    // Check in interpolated strings
    if let Some(interp) = node.as_interpolated_string_node() {
        for part in interp.parts().iter() {
            if let Some(embedded) = part.as_embedded_statements_node() {
                if let Some(stmts) = embedded.statements() {
                    for stmt in stmts.body().iter() {
                        if references_var(&stmt, var_name) {
                            return true;
                        }
                    }
                }
            }
        }
    }

    // Check parenthesized expressions
    if let Some(parens) = node.as_parentheses_node() {
        if let Some(body) = parens.body() {
            if let Some(stmts) = body.as_statements_node() {
                for stmt in stmts.body().iter() {
                    if references_var(&stmt, var_name) {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Check if the expression only references the element variable (and not the accumulator).
/// Only returns true for simple element expressions (bare `el` or `el.method` with no args
/// involving other variables). Method chains on the element (like `el[:key].bar`) are NOT
/// considered "only element" because they may return a transformed value that serves as
/// a valid new accumulator (matching RuboCop's behavior via expression_values).
fn is_only_element_expr(node: &ruby_prism::Node<'_>, acc_name: &str, el_name: &str) -> bool {
    // Direct element read
    if let Some(read) = node.as_local_variable_read_node() {
        return std::str::from_utf8(read.name().as_slice()).unwrap_or("") == el_name;
    }

    // Expression involving the element — only flag simple one-level method calls
    // where the receiver is directly the element variable (not a chain)
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            // Only flag if the receiver IS the element variable directly (not a chain)
            if let Some(read) = recv.as_local_variable_read_node() {
                let recv_name = std::str::from_utf8(read.name().as_slice()).unwrap_or("");
                if recv_name == el_name {
                    // Check args don't reference accumulator or other variables
                    if let Some(args) = call.arguments() {
                        for arg in args.arguments().iter() {
                            if references_var(&arg, acc_name) {
                                return false;
                            }
                            // If args contain any local variable, it's a complex expression
                            if has_any_local_var(&arg) {
                                return false;
                            }
                        }
                    }
                    return true;
                }
            }
            // Receiver is a complex expression (method chain) — not "only element"
            return false;
        }
        // Bare method call with element as argument
        if call.receiver().is_none() {
            if let Some(args) = call.arguments() {
                let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
                let has_el = arg_list.iter().any(|a| references_var(a, el_name));
                let has_acc = arg_list.iter().any(|a| references_var(a, acc_name));
                if has_el && !has_acc {
                    return true;
                }
            }
        }
    }

    false
}

/// Check if an expression contains any local variable reference.
fn has_any_local_var(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_local_variable_read_node().is_some() {
        return true;
    }
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            if has_any_local_var(&recv) {
                return true;
            }
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if has_any_local_var(&arg) {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        UnmodifiedReduceAccumulator,
        "cops/lint/unmodified_reduce_accumulator"
    );
}
