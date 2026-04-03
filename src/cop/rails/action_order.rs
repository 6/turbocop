use crate::cop::shared::node_type::{CLASS_NODE, DEF_NODE, STATEMENTS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Enforces consistent ordering of the standard Rails RESTful controller actions.
///
/// Root cause of original FP batch: the cop iterated ALL `def` nodes in the class body
/// without tracking visibility scope. Private/protected CRUD actions (after `private`/
/// `protected` keywords or inline `private def foo`) were incorrectly flagged.
///
/// Root cause of second FP batch (100 FPs): the cop didn't handle `private :method_name`
/// or `protected :method_name` (symbol-argument visibility modifiers). RuboCop's
/// VisibilityHelp mixin checks right-siblings of a def node for `private :method_name`
/// calls, treating those methods as non-public. Fix: pre-collect all symbol arguments
/// to `private`/`protected` calls in the class body, then exclude matching def names.
///
/// Root cause of third FP batch (100 FPs): the max-seen-idx algorithm reported offenses
/// for ALL actions that fall below the maximum index seen so far. RuboCop uses `each_cons(2)`
/// (consecutive pair comparison): only compare each action to its IMMEDIATELY PRECEDING action.
/// Example: `update(5), new(2), edit(3)` — the old approach flagged both `new` and `edit`
/// (both < max=5), but RuboCop only flags `new` (new < update), not `edit` (edit >= new).
/// Fix: changed from max-seen tracking to `windows(2)` consecutive pair comparison.
///
/// Root cause of FN=1: the cop only looked at direct children of the class StatementsNode,
/// missing `def` nodes nested inside `if`/`unless` blocks.
///
/// Fix: added visibility tracking while iterating class body statements. Bare `private`
/// or `protected` calls (CallNode with no receiver, no arguments) set the visibility state.
/// Inline `private def foo` / `protected def foo` (CallNode wrapping DefNode in arguments)
/// are skipped. Symbol-arg visibility modifiers (`private :index, :show`) are collected
/// in a pre-pass and used to exclude those methods. Also walks into `if`/`unless` blocks
/// to find nested `def` nodes.
///
/// Root cause of FN=1 (second): the cop only checked ClassNode, not ModuleNode. Rails
/// controller concerns define action methods inside modules (e.g.,
/// `module Trestle::Resource::Controller::Actions`). RuboCop's `def_node_search` recursively
/// finds `def` nodes regardless of whether the enclosing scope is a class or module.
/// Fix: added MODULE_NODE to interested_node_types and handle ModuleNode in check_node
/// with the same logic as ClassNode.
///
/// Root cause of FN=1 (third): `collect_public_defs` didn't recurse into `ModuleNode`
/// bodies. In TrestleAdmin/trestle, actions are defined inside `module Actions` nested
/// within `class Resource`. RuboCop's `def_node_search` is recursive and finds `def` nodes
/// at any depth. Fix: added `ModuleNode` recursion in `collect_public_defs` so that `def`
/// nodes inside nested modules within a class are also collected.
///
/// ## Corpus investigation (2026-03-18)
///
/// Corpus oracle reported FP=5, FN=0.
///
/// FP=5: All 5 FPs were in `app/controllers/concerns/` module files. RuboCop's
/// ActionOrder cop only checks classes, not modules. Controller concern modules
/// define actions mixed into multiple controllers — ordering within the concern
/// is irrelevant. Fix: removed MODULE_NODE from interested_node_types.
///
/// Root cause of FN=1 (fourth, extended corpus): `collect_public_defs` didn't recurse
/// into `DefNode` bodies. In globocom/GloboDNS `domains_controller.rb`, a missing `end`
/// in `def create` causes Prism's error recovery to nest `def destroy` and
/// `def update_domain_owner` inside `def create`. The cop only saw `create` and `update`
/// at the class level (which are in order), missing `destroy` entirely. RuboCop's
/// `def_node_search` is recursive and finds `def` nodes at any depth, including nested
/// defs. Fix: after processing a `DefNode`, recurse into its body to find nested defs.
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

/// Collect method names made non-public via symbol arguments:
/// `private :index, :show` or `protected :create`.
/// Returns a set of method name byte vectors.
fn collect_symbol_visibility_methods(
    stmts: &ruby_prism::StatementsNode<'_>,
) -> std::collections::HashSet<Vec<u8>> {
    let mut non_public = std::collections::HashSet::new();
    for stmt in stmts.body().iter() {
        if let Some(call) = stmt.as_call_node() {
            if call.receiver().is_none() {
                let name = call.name().as_slice();
                if matches!(name, b"private" | b"protected") {
                    if let Some(args) = call.arguments() {
                        for arg in args.arguments().iter() {
                            if let Some(sym) = arg.as_symbol_node() {
                                non_public.insert(sym.unescaped().to_vec());
                            }
                        }
                    }
                }
            }
        }
    }
    non_public
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
        // Recurse into def body to find nested defs. This handles syntax-error cases
        // where Prism's error recovery nests def nodes inside other def nodes (e.g.,
        // a missing `end` causes `def destroy` to be parsed inside `def create`).
        // RuboCop's `def_node_search` is recursive and finds defs at any depth.
        if let Some(body) = def_node.body() {
            if let Some(stmts) = body.as_statements_node() {
                for child in stmts.body().iter() {
                    collect_public_defs(&child, is_public, out);
                }
            }
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
        return;
    }

    // Walk into nested modules to find defs (RuboCop's def_node_search is recursive)
    if let Some(module_node) = node.as_module_node() {
        if let Some(body) = module_node.body() {
            if let Some(stmts) = body.as_statements_node() {
                for child in stmts.body().iter() {
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
        // Only check ClassNode — RuboCop does not check ModuleNode (controller concerns).
        let body = if let Some(class) = node.as_class_node() {
            class.body()
        } else {
            return;
        };

        let body = match body {
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

        // Pre-collect methods made non-public via symbol args (e.g. `private :index, :show`)
        let symbol_non_public = collect_symbol_visibility_methods(&stmts);

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
                // Skip methods explicitly made non-public via symbol args
                if symbol_non_public.contains(&name) {
                    continue;
                }
                if let Some(idx) = order_list.iter().position(|&a| a == name.as_slice()) {
                    actions.push((name, idx, offset));
                }
            }
        }

        // Check each consecutive pair of actions (matching RuboCop's each_cons(2) behavior).
        // Only compare each action to its IMMEDIATELY PRECEDING action in the collected list.
        // This prevents over-reporting: when `destroy` appears first, subsequent lower-index
        // actions are only flagged if out of order relative to their immediate predecessor,
        // not relative to the global maximum seen so far.
        for pair in actions.windows(2) {
            let (prev_name, prev_idx, _) = &pair[0];
            let (name, idx, offset) = &pair[1];
            if idx < prev_idx {
                let (line, column) = source.offset_to_line_col(*offset);
                let name_str = String::from_utf8_lossy(name);
                let prev_str = String::from_utf8_lossy(prev_name);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Action `{name_str}` should appear before `{prev_str}` in the controller."
                    ),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ActionOrder, "cops/rails/action_order");
}
