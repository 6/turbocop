use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use crate::cop::node_type::{BLOCK_NODE, BLOCK_PARAMETERS_NODE, BREAK_NODE, CALL_NODE, CASE_NODE, ELSE_NODE, EMBEDDED_STATEMENTS_NODE, IF_NODE, INTERPOLATED_STRING_NODE, LOCAL_VARIABLE_READ_NODE, NEXT_NODE, NUMBERED_PARAMETERS_NODE, PARENTHESES_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE, UNLESS_NODE, WHEN_NODE};

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
        &[BLOCK_NODE, BLOCK_PARAMETERS_NODE, BREAK_NODE, CALL_NODE, CASE_NODE, ELSE_NODE, EMBEDDED_STATEMENTS_NODE, IF_NODE, INTERPOLATED_STRING_NODE, LOCAL_VARIABLE_READ_NODE, NEXT_NODE, NUMBERED_PARAMETERS_NODE, PARENTHESES_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE, UNLESS_NODE, WHEN_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        if method_name != b"reduce" && method_name != b"inject" {
            return Vec::new();
        }

        let method_str = std::str::from_utf8(method_name).unwrap_or("reduce");

        // Must have a block
        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        // Get block parameters
        let params = match block.parameters() {
            Some(p) => p,
            None => return Vec::new(), // No block params
        };

        let (acc_name, el_name) = match extract_reduce_params(&params) {
            Some(names) => names,
            None => return Vec::new(),
        };

        // Get block body
        let body = match block.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Get the statements in the body
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let body_stmts: Vec<ruby_prism::Node<'_>> = stmts.body().iter().collect();
        if body_stmts.is_empty() {
            return Vec::new();
        }

        let mut diagnostics = Vec::new();

        // Check each return point (last expression, next, break)
        check_return_values(
            self,
            source,
            &body_stmts,
            &acc_name,
            &el_name,
            method_str,
            &mut diagnostics,
        );

        diagnostics
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

        let acc = requireds[0]
            .as_required_parameter_node()
            .map(|p| std::str::from_utf8(p.name().as_slice()).unwrap_or("").to_string())?;
        let el = requireds[1]
            .as_required_parameter_node()
            .map(|p| std::str::from_utf8(p.name().as_slice()).unwrap_or("").to_string());

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

fn check_return_values(
    cop: &UnmodifiedReduceAccumulator,
    source: &SourceFile,
    stmts: &[ruby_prism::Node<'_>],
    acc_name: &str,
    el_name: &str,
    method_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Check `next` and `break` statements
    for stmt in stmts {
        check_next_break(cop, source, stmt, acc_name, el_name, method_name, diagnostics);
    }

    // Check the last expression (implicit return value)
    if let Some(last) = stmts.last() {
        // Skip if it's a next/break (already handled)
        if last.as_next_node().is_some() || last.as_break_node().is_some() {
            return;
        }

        // Check if any return point (next/break, including inside conditionals)
        // returns the accumulator.
        let has_acc_return = stmts.iter().any(|s| returns_accumulator(s, acc_name));

        if has_acc_return {
            // If some branch returns the accumulator, the element return is OK
            return;
        }

        if !references_var(last, acc_name) && is_only_element_expr(last, acc_name, el_name) {
            let loc = last.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
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

fn check_next_break(
    cop: &UnmodifiedReduceAccumulator,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    acc_name: &str,
    el_name: &str,
    method_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(next) = node.as_next_node() {
        if let Some(args) = next.arguments() {
            let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
            for arg in &arg_list {
                if !references_var(arg, acc_name) && is_only_element_expr(arg, acc_name, el_name) {
                    let loc = arg.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
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
    }

    if let Some(brk) = node.as_break_node() {
        if let Some(args) = brk.arguments() {
            let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
            for arg in &arg_list {
                if !references_var(arg, acc_name) && is_only_element_expr(arg, acc_name, el_name) {
                    let loc = arg.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
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
    }

    // Check if nodes inside conditionals
    if let Some(if_node) = node.as_if_node() {
        if let Some(body) = if_node.statements() {
            for stmt in body.body().iter() {
                check_next_break(cop, source, &stmt, acc_name, el_name, method_name, diagnostics);
            }
        }
        if let Some(else_clause) = if_node.subsequent() {
            check_next_break(cop, source, &else_clause, acc_name, el_name, method_name, diagnostics);
        }
    }

    if let Some(unless_node) = node.as_unless_node() {
        if let Some(body) = unless_node.statements() {
            for stmt in body.body().iter() {
                check_next_break(cop, source, &stmt, acc_name, el_name, method_name, diagnostics);
            }
        }
    }
}

/// Check if a statement or any nested conditional contains a next/break
/// that returns the accumulator.
fn returns_accumulator(node: &ruby_prism::Node<'_>, acc_name: &str) -> bool {
    if let Some(next) = node.as_next_node() {
        if let Some(args) = next.arguments() {
            let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
            if arg_list.iter().any(|a| references_var(a, acc_name)) {
                return true;
            }
        }
    }
    if let Some(brk) = node.as_break_node() {
        if let Some(args) = brk.arguments() {
            let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
            if arg_list.iter().any(|a| references_var(a, acc_name)) {
                return true;
            }
        }
    }
    // Check inside conditionals
    if let Some(if_node) = node.as_if_node() {
        if let Some(body) = if_node.statements() {
            if body.body().iter().any(|s| returns_accumulator(&s, acc_name)) {
                return true;
            }
        }
        if let Some(else_clause) = if_node.subsequent() {
            if returns_accumulator(&else_clause, acc_name) {
                return true;
            }
        }
    }
    if let Some(unless_node) = node.as_unless_node() {
        if let Some(body) = unless_node.statements() {
            if body.body().iter().any(|s| returns_accumulator(&s, acc_name)) {
                return true;
            }
        }
    }
    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            if stmts.body().iter().any(|s| returns_accumulator(&s, acc_name)) {
                return true;
            }
        }
    }
    // Check inside case/when
    if let Some(case_node) = node.as_case_node() {
        for condition in case_node.conditions().iter() {
            if let Some(when_node) = condition.as_when_node() {
                if let Some(body) = when_node.statements() {
                    if body.body().iter().any(|s| returns_accumulator(&s, acc_name)) {
                        return true;
                    }
                }
            }
        }
        if let Some(else_clause) = case_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                if stmts.body().iter().any(|s| returns_accumulator(&s, acc_name)) {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if an expression references a variable by name.
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
    crate::cop_fixture_tests!(UnmodifiedReduceAccumulator, "cops/lint/unmodified_reduce_accumulator");
}
