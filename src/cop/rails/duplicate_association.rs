use std::collections::HashMap;

use crate::cop::node_type::{CLASS_NODE, SYMBOL_NODE};
use crate::cop::util::{is_dsl_call, parent_class_name};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/DuplicateAssociation
///
/// Detects two kinds of duplicate associations in ActiveRecord models:
/// 1. Same association name used multiple times (any association type)
/// 2. Same `class_name:` option used in multiple `has_many`/`has_one`/`has_and_belongs_to_many`
///    associations that have no other options (excludes `belongs_to`)
///
/// Supports all four association methods: `has_many`, `has_one`, `belongs_to`,
/// `has_and_belongs_to_many`. Accepts both symbol and string first arguments.
///
/// ## Implementation notes
///
/// RuboCop's `register_offense` flags ALL members of a duplicate group, including the first
/// occurrence. The implementation groups calls by name and then flags all members of groups
/// with >1 member (both passes: name duplicates and class_name duplicates).
///
/// Message format for name duplicates: "Association `x` is defined multiple times. Don't
/// repeat associations." (matching RuboCop exactly).
///
/// ## FP fixes (2026-03-26)
///
/// Verified against the corpus bundle's `rubocop-rails` 2.34.3:
///
/// 1. `ClassSendNodeHelper#class_send_nodes` only descends into `if`/`unless`
///    bodies when the conditional is the class body's sole statement. When the
///    class body is a multi-statement `begin`, conditional associations are
///    ignored. Our unconditional descent caused FP=72 across
///    `voormedia/rails-erd`, `lorint/brick`, `rails_admin`, `cocoon`, and
///    `front_end_builds`.
/// 2. `ActiveRecordHelper#active_record?` matches only bare `ApplicationRecord`
///    and `ActiveRecord::Base`, not namespaced parents like
///    `Ci::ApplicationRecord`. Our `ends_with("Record")` fallback caused FP=4
///    in `gisia`.
/// 3. In multi-statement class bodies, block associations are `block` nodes in
///    Parser AST, so `class_send_nodes` does not include them. Prism models
///    them as `CallNode` with `block()`, so we must skip block-bearing calls in
///    the multi-statement case. This caused the remaining FP=1 in `lowdown`.
///
/// ## Reverted fix attempt (2026-03-23, commit 3002d481)
///
/// Attempted to fix FP on block associations and FN on if-branch patterns.
/// Introduced FP=2 on standard corpus; reverted in 1bf1bea3.
///
/// **FP=2 (elsif recursion over-collects):** The `collect_calls_from_if_branches`
/// function recursively collected calls from both `if` and `elsif` branches.
/// In Parser AST, `if ... elsif ... end` is `(if cond (send ...) (if cond2
/// (send ...) nil))` — `each_child_node(:send)` on the outer `if` only finds
/// the `if` branch's send, NOT the `elsif` branch's send (it's inside a nested
/// `if` node). The fix over-collected by recursing into elsif. Fix: only
/// collect from the `if` body and the `else` body, not nested `if` nodes from
/// elsif chains.
pub struct DuplicateAssociation;

/// Association method names we track.
const ASSOCIATION_METHODS: &[&[u8]] = &[
    b"has_many",
    b"has_one",
    b"belongs_to",
    b"has_and_belongs_to_many",
];

/// Check if the parent class looks like an ActiveRecord base class.
fn is_active_record_parent(parent: &[u8]) -> bool {
    let parent = if let Some(stripped) = parent.strip_prefix(b"::") {
        stripped
    } else {
        parent
    };

    parent == b"ApplicationRecord" || parent == b"ActiveRecord::Base"
}

impl Cop for DuplicateAssociation {
    fn name(&self) -> &'static str {
        "Rails/DuplicateAssociation"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_NODE, SYMBOL_NODE]
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
        let class = match node.as_class_node() {
            Some(c) => c,
            None => return,
        };

        // Only check classes that inherit from ActiveRecord
        let parent = parent_class_name(source, &class);
        if let Some(parent_name) = parent {
            if !is_active_record_parent(parent_name) {
                return;
            }
        } else {
            // No parent class at all — skip
            return;
        }

        let calls = collect_association_calls(&class);

        // --- Pass 1: Duplicate association names ---
        // Group calls by name, then flag ALL occurrences in groups with >1 member.
        // RuboCop's `register_offense` flags every member of a duplicate group,
        // including the first occurrence — not just subsequent ones.
        let mut name_groups: HashMap<Vec<u8>, Vec<usize>> = HashMap::new();

        for (idx, call) in calls.iter().enumerate() {
            if !is_association_call(call) {
                continue;
            }

            let name = match extract_first_name_arg(call) {
                Some(n) => n,
                None => continue,
            };

            name_groups.entry(name).or_default().push(idx);
        }

        for (name, indices) in &name_groups {
            if indices.len() <= 1 {
                continue;
            }
            let name_str = String::from_utf8_lossy(name);
            for &idx in indices {
                let call = &calls[idx];
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Association `{name_str}` is defined multiple times. Don't repeat associations."
                    ),
                ));
            }
        }

        // --- Pass 2: Duplicate class_name (has_* only, not belongs_to) ---
        // Only flag when the hash argument has exactly one pair: `class_name: 'X'`
        // RuboCop flags ALL members of a duplicate group, not just subsequent ones.
        let mut class_name_groups: HashMap<Vec<u8>, Vec<usize>> = HashMap::new();

        for (idx, call) in calls.iter().enumerate() {
            // Skip belongs_to — RuboCop excludes it from class_name duplicate check
            if !is_association_call(call) || is_dsl_call(call, b"belongs_to") {
                continue;
            }

            if let Some(cn_source) = extract_sole_class_name(source, call) {
                class_name_groups.entry(cn_source).or_default().push(idx);
            }
        }

        for (cn_source, indices) in &class_name_groups {
            if indices.len() <= 1 {
                continue;
            }
            let cn_str = String::from_utf8_lossy(cn_source);
            for &idx in indices {
                let call = &calls[idx];
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Association `class_name: {cn_str}` is defined multiple times. Don't repeat associations."
                    ),
                ));
            }
        }
    }
}

/// Collect association-relevant calls, emulating RuboCop's Parser AST traversal.
///
/// Prism always wraps class bodies in `StatementsNode`, but Parser only wraps
/// multi-statement bodies in `begin`. RuboCop's `class_send_nodes` therefore has
/// two distinct behaviors we must preserve:
///
/// 1. Single-statement class body:
///    - bare send / send+block => include that call
///    - `if` / `unless` => descend into branch sends
/// 2. Multi-statement class body (`begin` in Parser):
///    - include only direct send statements
///    - do not descend into nested conditionals
///    - do not include block-wrapped send statements
fn collect_association_calls<'a>(
    class_node: &ruby_prism::ClassNode<'a>,
) -> Vec<ruby_prism::CallNode<'a>> {
    let body = match class_node.body() {
        Some(b) => b,
        None => return Vec::new(),
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return Vec::new(),
    };

    let stmt_nodes: Vec<_> = stmts.body().iter().collect();
    if stmt_nodes.len() == 1 {
        let node = &stmt_nodes[0];

        if let Some(call) = node.as_call_node() {
            return vec![call];
        }

        if let Some(if_node) = node.as_if_node() {
            let mut calls = Vec::new();
            collect_calls_from_if_node(&if_node, &mut calls);
            return calls;
        }

        if let Some(unless_node) = node.as_unless_node() {
            let mut calls = Vec::new();
            collect_calls_from_unless_node(&unless_node, &mut calls);
            return calls;
        }

        return Vec::new();
    }

    stmt_nodes
        .into_iter()
        .filter_map(|node| {
            let call = node.as_call_node()?;
            if call.block().is_some() {
                None
            } else {
                Some(call)
            }
        })
        .collect()
}

/// Collect calls from an IfNode's body and else clause.
///
/// IMPORTANT: Do NOT recurse into elsif (subsequent IfNode). In Parser AST,
/// `if ... elsif ... end` has the elsif as a nested `if` node. RuboCop's
/// `each_child_node(:send)` only finds sends that are direct children of the
/// outer `if`, which means only the `if` body and `else` body sends, not
/// sends inside elsif branches. We replicate this exactly.
///
/// Prism's `subsequent()` returns:
/// - `Some(ElseNode)` for an `else` clause
/// - `Some(IfNode)` for an `elsif` clause
/// - `None` if there is no else/elsif
fn collect_calls_from_if_node<'a>(
    if_node: &ruby_prism::IfNode<'a>,
    calls: &mut Vec<ruby_prism::CallNode<'a>>,
) {
    // Collect from the if body (StatementsNode)
    if let Some(body) = if_node.statements() {
        for stmt in body.body().iter() {
            if let Some(call) = stmt.as_call_node() {
                calls.push(call);
            }
        }
    }

    // Collect from the else clause only (ElseNode -> StatementsNode).
    // If subsequent() is an IfNode, that's an elsif — do NOT collect from it.
    if let Some(subsequent) = if_node.subsequent() {
        if let Some(else_node) = subsequent.as_else_node() {
            if let Some(else_stmts) = else_node.statements() {
                for stmt in else_stmts.body().iter() {
                    if let Some(call) = stmt.as_call_node() {
                        calls.push(call);
                    }
                }
            }
        }
    }
}

/// Collect calls from an UnlessNode's body and else clause.
///
/// Mirrors the `if` handling above: only the direct sends from the unless body
/// and the else body are visible through RuboCop's `each_child_node(:send)`.
fn collect_calls_from_unless_node<'a>(
    unless_node: &ruby_prism::UnlessNode<'a>,
    calls: &mut Vec<ruby_prism::CallNode<'a>>,
) {
    if let Some(body) = unless_node.statements() {
        for stmt in body.body().iter() {
            if let Some(call) = stmt.as_call_node() {
                calls.push(call);
            }
        }
    }

    if let Some(else_clause) = unless_node.else_clause() {
        if let Some(else_stmts) = else_clause.statements() {
            for stmt in else_stmts.body().iter() {
                if let Some(call) = stmt.as_call_node() {
                    calls.push(call);
                }
            }
        }
    }
}

/// Check if the call is one of the four association methods.
fn is_association_call(call: &ruby_prism::CallNode<'_>) -> bool {
    ASSOCIATION_METHODS.iter().any(|m| is_dsl_call(call, m))
}

/// Extract the first argument (association name) as either a symbol or string.
fn extract_first_name_arg(call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let args = call.arguments()?;
    let first_arg = args.arguments().iter().next()?;
    if let Some(sym) = first_arg.as_symbol_node() {
        return Some(sym.unescaped().to_vec());
    }
    if let Some(s) = first_arg.as_string_node() {
        return Some(s.unescaped().to_vec());
    }
    None
}

/// If the call has exactly one extra argument beyond the name, and that argument
/// is a keyword hash with exactly one pair `class_name: <value>`, return the
/// source text of the value (e.g., `'Foo'`).
///
/// This matches RuboCop's `class_name` node pattern: `(hash (pair (sym :class_name) $_))`
/// combined with the `arguments.one?` guard.
fn extract_sole_class_name(
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
) -> Option<Vec<u8>> {
    if call.block().is_some() {
        return None;
    }

    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();

    // Must have exactly 2 arguments: name + hash (arguments.one? in RuboCop
    // refers to the rest-args after the name capture, so 1 extra arg)
    if arg_list.len() != 2 {
        return None;
    }

    // The second arg should be a keyword hash with exactly one pair
    let hash_node = arg_list[1].as_keyword_hash_node()?;
    let elements: Vec<_> = hash_node.elements().iter().collect();
    if elements.len() != 1 {
        return None;
    }

    let assoc = elements[0].as_assoc_node()?;
    let key_sym = assoc.key().as_symbol_node()?;
    if key_sym.unescaped() != b"class_name" {
        return None;
    }

    // Return the source text of the value node (e.g., 'Foo' or "Foo")
    let value = assoc.value();
    let start = value.location().start_offset();
    let end = value.location().end_offset();
    Some(source.as_bytes()[start..end].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateAssociation, "cops/rails/duplicate_association");
}
