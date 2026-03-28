use crate::cop::node_type::{
    BLOCK_NODE, BLOCK_PARAMETERS_NODE, BREAK_NODE, CALL_NODE, CASE_NODE, ELSE_NODE,
    EMBEDDED_STATEMENTS_NODE, IF_NODE, INTERPOLATED_STRING_NODE, LOCAL_VARIABLE_READ_NODE,
    NEXT_NODE, NUMBERED_PARAMETERS_NODE, PARENTHESES_NODE, REQUIRED_PARAMETER_NODE,
    STATEMENTS_NODE, UNLESS_NODE, WHEN_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

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
///
/// ## Corpus investigation (2026-03-15)
///
/// Corpus oracle reported FP=4, FN=1.
///
/// FP=4: accumulator references nested inside splats, arrays, and keyword/hash
/// arguments were being missed. That made calls like `processor.call(*packet)`
/// and `scope.wrap(body: [body])` look like element-only returns even though the
/// accumulator is part of the returned value.
///
/// FN=1: single-expression block bodies (for example `{ |memo, item|
/// expect(item).to eq(...) }`) were ignored because Prism returns the body as a
/// direct expression node rather than a `StatementsNode`. Within those bodies,
/// chained calls rooted in a bare method call that only uses the element still
/// need to count as element-only returns.
///
/// ## Corpus investigation (2026-03-19)
///
/// Corpus oracle reported FP=0, FN=4. Two distinct root causes:
///
/// **FN root cause 1 (2 cases):** `next element` inside a conditional branch was
/// not flagged when the accumulator appeared in another return position (e.g.,
/// `item.process && all_ok` as the last expression). The `uses_acc` check used a
/// deep recursive visitor (`lvar_used`) that found the accumulator buried inside
/// complex expressions like `&&` nodes. RuboCop's `lvar_used?` is a shallow
/// node_matcher that only matches top-level patterns (bare lvar, lvasgn,
/// `acc << x`, string interpolation). Replaced with `lvar_used_shallow` to match
/// RuboCop's semantics: `acc += 1` and `el.foo && acc` as return values do NOT
/// count as "using the accumulator" for `returns_accumulator_anywhere?`.
///
/// **FN root cause 2 (2 cases):** `acc[k.to_sym]` and `acc[db[val]]` were not
/// flagged as accumulator index offenses. The element check in
/// `is_accumulator_index` used deep `lvar_used` to find the element variable
/// inside index arguments like `k.to_sym`. Since `k` (the element) was found,
/// the check treated it as `acc[el]` (acceptable). But RuboCop's `lvar_used?` is
/// shallow — `(send (lvar :k) :to_sym)` does NOT match `(lvar :k)`. Only bare
/// `result[key]` is acceptable; `result[key.to_sym]` is an offense. Replaced
/// with `lvar_used_shallow`.
///
/// ## Corpus investigation (2026-03-28)
///
/// Corpus oracle reported FP=6, FN=7. The FNs came from two narrow mismatches
/// with RuboCop:
///
/// 1. Bare method calls like `type(e)` and `record_flavor_usage(flavor)` were
///    incorrectly treated as "element modified" because the port only checked
///    whether the element appeared anywhere in the argument list. RuboCop only
///    treats bare calls as mutation-like when the element is passed as a bare
///    local variable alongside at least one additional argument (`method(el, ...)`).
///
/// 2. Returns like `k[:permitted_attributes] || {}` were missed because the port
///    only recognized simple element expressions, not boolean fallbacks whose
///    only variable-bearing branch still depends solely on the element. Added a
///    narrow `||` case that still preserves method-chain no-offenses like
///    `entity[:indices].last || []`.
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

        let body_stmts: Vec<ruby_prism::Node<'_>> = if let Some(stmts) = body.as_statements_node() {
            stmts.body().iter().collect()
        } else {
            vec![body]
        };

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
        // Use shallow check matching RuboCop's `lvar_used?` node_matcher:
        // only top-level patterns like bare lvar, lvasgn, send(lvar, :<<), dstr, etc.
        // Deep references (e.g. `el.foo && acc`) don't count for
        // `returns_accumulator_anywhere?`.
        uses_acc: lvar_used_shallow(node, acc_name),
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
                        // For acc[], check if any argument is directly the element
                        // variable. Uses shallow check matching RuboCop's `lvar_used?`
                        // node_matcher — `acc[el]` is acceptable but `acc[el.to_sym]`
                        // or `acc[foo[el]]` are offenses.
                        if let Some(args) = call.arguments() {
                            let has_el = args
                                .arguments()
                                .iter()
                                .any(|a| lvar_used_shallow(&a, el_name));
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
        // method(el, ...) — bare method with the element plus another argument
        if call.receiver().is_none() {
            if let Some(args) = call.arguments() {
                let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
                let has_bare_el = arg_list.iter().any(|a| bare_local_read(a, el_name));
                if has_bare_el && arg_list.len() > 1 {
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

/// Shallow check matching RuboCop's `lvar_used?` node_matcher.
/// Only matches top-level patterns:
///   (lvar name)                         — bare variable read
///   (lvasgn name ...)                   — assignment
///   (send (lvar name) :<< ...)          — shovel operator
///   (dstr (begin (lvar name)))          — string interpolation
///   (op_asgn (lvasgn name) ...)         — shorthand assignment (+=, etc.)
///
/// Does NOT recursively search into child nodes. This is used for
/// `returns_accumulator_anywhere?` and `returned_accumulator_index` element
/// checks where RuboCop only looks at the outermost node shape.
fn lvar_used_shallow(node: &ruby_prism::Node<'_>, var_name: &str) -> bool {
    // (lvar name)
    if let Some(read) = node.as_local_variable_read_node() {
        return std::str::from_utf8(read.name().as_slice()).unwrap_or("") == var_name;
    }

    // (lvasgn name ...)
    if let Some(write) = node.as_local_variable_write_node() {
        return std::str::from_utf8(write.name().as_slice()).unwrap_or("") == var_name;
    }

    // (send (lvar name) :<< ...)
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"<<" {
            if let Some(recv) = call.receiver() {
                if let Some(read) = recv.as_local_variable_read_node() {
                    return std::str::from_utf8(read.name().as_slice()).unwrap_or("") == var_name;
                }
            }
        }
    }

    // (dstr (begin (lvar name)))
    if let Some(dstr) = node.as_interpolated_string_node() {
        let parts: Vec<ruby_prism::Node<'_>> = dstr.parts().iter().collect();
        if parts.len() == 1 {
            if let Some(embedded) = parts[0].as_embedded_statements_node() {
                if let Some(stmts) = embedded.statements() {
                    let body: Vec<ruby_prism::Node<'_>> = stmts.body().iter().collect();
                    if body.len() == 1 {
                        if let Some(read) = body[0].as_local_variable_read_node() {
                            return std::str::from_utf8(read.name().as_slice()).unwrap_or("")
                                == var_name;
                        }
                    }
                }
            }
        }
    }

    // NOTE: Shorthand assignments (acc += 1, acc ||= x, acc &&= x) are
    // intentionally NOT matched here. RuboCop's `lvar_used?` pattern
    // `(%SHORTHAND_ASSIGNMENTS (lvasgn %1))` does not match op_asgn nodes
    // because they have extra children (operator, value) beyond the lvasgn.
    // This means `acc += 1` as a return value does NOT count as "using the
    // accumulator" for `returns_accumulator_anywhere?`, which is correct —
    // `next el` should still be flagged even when the last expression is
    // `acc += 1`.

    false
}

/// Check if a node contains a local variable reference or assignment for the
/// given name anywhere within the return expression.
fn lvar_used(node: &ruby_prism::Node<'_>, var_name: &str) -> bool {
    struct VarFinder<'a> {
        target: &'a str,
        found: bool,
    }

    impl<'pr> Visit<'pr> for VarFinder<'_> {
        fn visit_local_variable_read_node(
            &mut self,
            node: &ruby_prism::LocalVariableReadNode<'pr>,
        ) {
            if std::str::from_utf8(node.name().as_slice()).unwrap_or("") == self.target {
                self.found = true;
                return;
            }
            ruby_prism::visit_local_variable_read_node(self, node);
        }

        fn visit_local_variable_write_node(
            &mut self,
            node: &ruby_prism::LocalVariableWriteNode<'pr>,
        ) {
            if std::str::from_utf8(node.name().as_slice()).unwrap_or("") == self.target {
                self.found = true;
                return;
            }
            ruby_prism::visit_local_variable_write_node(self, node);
        }
    }

    let mut finder = VarFinder {
        target: var_name,
        found: false,
    };
    finder.visit(node);
    finder.found
}

/// Check if an expression references a variable by name anywhere in the node tree.
fn references_var(node: &ruby_prism::Node<'_>, var_name: &str) -> bool {
    lvar_used(node, var_name)
}

/// Check if the expression only references the element variable (and not the accumulator).
/// Only returns true for simple element expressions (bare `el` or `el.method` with no args
/// involving other variables). Method chains on the element (like `el[:key].bar`) are NOT
/// considered "only element" because they may return a transformed value that serves as
/// a valid new accumulator (matching RuboCop's behavior via expression_values).
fn is_only_element_expr(node: &ruby_prism::Node<'_>, acc_name: &str, el_name: &str) -> bool {
    if let Some(or_node) = node.as_or_node() {
        let left = or_node.left();
        let right = or_node.right();
        let left_is_only_element = is_only_element_expr(&left, acc_name, el_name);
        let right_is_only_element = is_only_element_expr(&right, acc_name, el_name);
        let left_is_literalish = !has_rubocop_expression_values(&left);
        let right_is_literalish = !has_rubocop_expression_values(&right);

        return (left_is_only_element || left_is_literalish)
            && (right_is_only_element || right_is_literalish)
            && (left_is_only_element || right_is_only_element);
    }

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
            if is_bare_call_chain_only_element(&recv, acc_name, el_name)
                && call
                    .arguments()
                    .is_none_or(|args| args_have_no_acc_or_other_vars(&args, acc_name, el_name))
            {
                return true;
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
                let has_other = arg_list
                    .iter()
                    .any(|a| has_any_local_var(a) && !references_var(a, el_name));
                if has_el && !has_acc && !has_other {
                    return true;
                }
            }
        }
    }

    false
}

fn bare_local_read(node: &ruby_prism::Node<'_>, var_name: &str) -> bool {
    node.as_local_variable_read_node()
        .is_some_and(|read| std::str::from_utf8(read.name().as_slice()).unwrap_or("") == var_name)
}

fn has_rubocop_expression_values(node: &ruby_prism::Node<'_>) -> bool {
    struct ExpressionValueFinder {
        found: bool,
    }

    impl<'pr> Visit<'pr> for ExpressionValueFinder {
        fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
            self.found = true;
            ruby_prism::visit_call_node(self, node);
        }

        fn visit_local_variable_read_node(
            &mut self,
            node: &ruby_prism::LocalVariableReadNode<'pr>,
        ) {
            self.found = true;
            ruby_prism::visit_local_variable_read_node(self, node);
        }

        fn visit_local_variable_write_node(
            &mut self,
            node: &ruby_prism::LocalVariableWriteNode<'pr>,
        ) {
            self.found = true;
            ruby_prism::visit_local_variable_write_node(self, node);
        }

        fn visit_instance_variable_read_node(
            &mut self,
            node: &ruby_prism::InstanceVariableReadNode<'pr>,
        ) {
            self.found = true;
            ruby_prism::visit_instance_variable_read_node(self, node);
        }

        fn visit_instance_variable_write_node(
            &mut self,
            node: &ruby_prism::InstanceVariableWriteNode<'pr>,
        ) {
            self.found = true;
            ruby_prism::visit_instance_variable_write_node(self, node);
        }

        fn visit_class_variable_read_node(
            &mut self,
            node: &ruby_prism::ClassVariableReadNode<'pr>,
        ) {
            self.found = true;
            ruby_prism::visit_class_variable_read_node(self, node);
        }

        fn visit_class_variable_write_node(
            &mut self,
            node: &ruby_prism::ClassVariableWriteNode<'pr>,
        ) {
            self.found = true;
            ruby_prism::visit_class_variable_write_node(self, node);
        }

        fn visit_global_variable_read_node(
            &mut self,
            node: &ruby_prism::GlobalVariableReadNode<'pr>,
        ) {
            self.found = true;
            ruby_prism::visit_global_variable_read_node(self, node);
        }

        fn visit_global_variable_write_node(
            &mut self,
            node: &ruby_prism::GlobalVariableWriteNode<'pr>,
        ) {
            self.found = true;
            ruby_prism::visit_global_variable_write_node(self, node);
        }
    }

    let mut finder = ExpressionValueFinder { found: false };
    finder.visit(node);
    finder.found
}

/// Check if an expression contains any local variable reference.
fn has_any_local_var(node: &ruby_prism::Node<'_>) -> bool {
    struct AnyVarFinder {
        found: bool,
    }

    impl<'pr> Visit<'pr> for AnyVarFinder {
        fn visit_local_variable_read_node(
            &mut self,
            node: &ruby_prism::LocalVariableReadNode<'pr>,
        ) {
            self.found = true;
            ruby_prism::visit_local_variable_read_node(self, node);
        }

        fn visit_local_variable_write_node(
            &mut self,
            node: &ruby_prism::LocalVariableWriteNode<'pr>,
        ) {
            self.found = true;
            ruby_prism::visit_local_variable_write_node(self, node);
        }

        fn visit_local_variable_operator_write_node(
            &mut self,
            node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
        ) {
            self.found = true;
            ruby_prism::visit_local_variable_operator_write_node(self, node);
        }

        fn visit_local_variable_or_write_node(
            &mut self,
            node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
        ) {
            self.found = true;
            ruby_prism::visit_local_variable_or_write_node(self, node);
        }

        fn visit_local_variable_and_write_node(
            &mut self,
            node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
        ) {
            self.found = true;
            ruby_prism::visit_local_variable_and_write_node(self, node);
        }
    }

    let mut finder = AnyVarFinder { found: false };
    finder.visit(node);
    finder.found
}

fn args_have_no_acc_or_other_vars(
    args: &ruby_prism::ArgumentsNode<'_>,
    acc_name: &str,
    el_name: &str,
) -> bool {
    args.arguments().iter().all(|arg| {
        (references_var(&arg, el_name) || !has_any_local_var(&arg))
            && !references_var(&arg, acc_name)
    })
}

fn is_bare_call_chain_only_element(
    node: &ruby_prism::Node<'_>,
    acc_name: &str,
    el_name: &str,
) -> bool {
    let call = match node.as_call_node() {
        Some(call) => call,
        None => return false,
    };

    if call.receiver().is_none() {
        let Some(args) = call.arguments() else {
            return false;
        };
        let has_el = args
            .arguments()
            .iter()
            .any(|arg| references_var(&arg, el_name));
        return has_el && args_have_no_acc_or_other_vars(&args, acc_name, el_name);
    }

    let Some(recv) = call.receiver() else {
        return false;
    };
    is_bare_call_chain_only_element(&recv, acc_name, el_name)
        && call
            .arguments()
            .is_none_or(|args| args_have_no_acc_or_other_vars(&args, acc_name, el_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        UnmodifiedReduceAccumulator,
        "cops/lint/unmodified_reduce_accumulator"
    );
}
