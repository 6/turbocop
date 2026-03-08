use crate::cop::node_type::{CLASS_NODE, DEF_NODE, STATEMENTS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Enforces consistent ordering of the standard Rails RESTful controller actions.
///
/// Root cause of FP=100: the cop iterated ALL `def` nodes in the class body without
/// tracking visibility scope. Private/protected CRUD actions (after `private`/`protected`
/// keywords or inline `private def foo`) were incorrectly flagged.
///
/// Root cause of FN=1: the cop only looked at direct children of the class StatementsNode,
/// missing `def` nodes nested inside `if`/`unless` blocks.
///
/// Fix: added visibility tracking while iterating class body statements. Bare `private`
/// or `protected` calls (CallNode with no receiver, no arguments) set the visibility state.
/// Inline `private def foo` / `protected def foo` (CallNode wrapping DefNode in arguments)
/// are skipped. Also walks into `if`/`unless` blocks to find nested `def` nodes.
pub struct ActionOrder;

const STANDARD_ORDER: &[&[u8]] = &[
    b"index", b"show", b"new", b"edit", b"create", b"update", b"destroy",
];

/// Check if a node is a bare visibility modifier (`private`, `protected`, `public`)
/// with no arguments — i.e., it changes visibility for all subsequent methods.
/// Returns true and sets `is_public` accordingly.
fn check_bare_visibility(node: &ruby_prism::Node<'_>, is_public: &mut bool) -> bool {
    if let Some(call) = node.as_call_node() {
        if call.receiver().is_none() && call.arguments().is_none() {
            let name = call.name().as_slice();
            match name {
                b"private" | b"protected" => {
                    *is_public = false;
                    return true;
                }
                b"public" => {
                    *is_public = true;
                    return true;
                }
                _ => {}
            }
        }
    }
    false
}

/// Check if a node is an inline visibility-modified def like `private def foo; end`.
fn is_inline_visibility_def(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        if call.receiver().is_none() {
            let name = call.name().as_slice();
            if matches!(name, b"private" | b"protected") {
                if let Some(args) = call.arguments() {
                    for arg in args.arguments().iter() {
                        if arg.as_def_node().is_some() {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// Collect public def nodes from a statement, including defs inside if/unless blocks.
/// Appends (method_name_bytes, def_keyword_offset) tuples to `out`.
fn collect_public_defs(
    node: &ruby_prism::Node<'_>,
    is_public: bool,
    out: &mut Vec<(Vec<u8>, usize)>,
) {
    // Direct def node
    if let Some(def_node) = node.as_def_node() {
        if is_public {
            out.push((
                def_node.name().as_slice().to_vec(),
                def_node.def_keyword_loc().start_offset(),
            ));
        }
        return;
    }

    // Walk into if/unless blocks to find nested defs (still inherits current visibility)
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            for child in stmts.body().iter() {
                collect_public_defs(&child, is_public, out);
            }
        }
        if let Some(subsequent) = if_node.subsequent() {
            collect_public_defs(&subsequent, is_public, out);
        }
        return;
    }
    if let Some(unless_node) = node.as_unless_node() {
        if let Some(stmts) = unless_node.statements() {
            for child in stmts.body().iter() {
                collect_public_defs(&child, is_public, out);
            }
        }
        if let Some(else_clause) = unless_node.else_clause() {
            if let Some(else_stmts) = else_clause.statements() {
                for child in else_stmts.body().iter() {
                    collect_public_defs(&child, is_public, out);
                }
            }
        }
    }
}

impl Cop for ActionOrder {
    fn name(&self) -> &'static str {
        "Rails/ActionOrder"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["app/controllers/**/*.rb"]
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_NODE, DEF_NODE, STATEMENTS_NODE]
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
        let class = match node.as_class_node() {
            Some(c) => c,
            None => return,
        };

        let body = match class.body() {
            Some(b) => b,
            None => return,
        };
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        // Use configured order if provided, otherwise use standard order
        let configured_order = config.get_string_array("ExpectedOrder");
        let order_list: Vec<&[u8]> = match &configured_order {
            Some(list) => list.iter().map(|s| s.as_bytes()).collect(),
            None => STANDARD_ORDER.to_vec(),
        };

        // Collect (method_name, order_index, offset) for public standard actions,
        // tracking visibility state as we iterate class body statements.
        let mut actions: Vec<(Vec<u8>, usize, usize)> = Vec::new();
        let mut is_public = true;

        for stmt in stmts.body().iter() {
            // Check for bare visibility modifier: `private` / `protected` / `public`
            if check_bare_visibility(&stmt, &mut is_public) {
                continue;
            }

            // Check for inline visibility-modified def: `private def foo; end`
            // These are always non-public, skip them regardless of current visibility.
            if is_inline_visibility_def(&stmt) {
                continue;
            }

            // Collect public defs from this statement (handles direct defs and if/unless)
            let mut defs = Vec::new();
            collect_public_defs(&stmt, is_public, &mut defs);

            for (name, offset) in defs {
                if let Some(idx) = order_list.iter().position(|&a| a == name.as_slice()) {
                    actions.push((name, idx, offset));
                }
            }
        }

        let mut max_seen_idx = 0;
        let mut max_seen_name: Vec<u8> = Vec::new();

        for (name, idx, offset) in &actions {
            let idx = *idx;
            let offset = *offset;
            if idx < max_seen_idx {
                let (line, column) = source.offset_to_line_col(offset);
                let name_str = String::from_utf8_lossy(name);
                let other_str = String::from_utf8_lossy(&max_seen_name);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Action `{name_str}` should appear before `{other_str}` in the controller."
                    ),
                ));
            }
            if idx >= max_seen_idx {
                max_seen_idx = idx;
                max_seen_name = name.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ActionOrder, "cops/rails/action_order");
}
