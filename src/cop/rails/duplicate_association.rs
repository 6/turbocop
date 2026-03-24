use std::collections::HashMap;

use crate::cop::node_type::{CLASS_NODE, SYMBOL_NODE};
use crate::cop::util::{class_body_calls, is_dsl_call, parent_class_name};
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
    parent == b"ApplicationRecord" || parent == b"ActiveRecord::Base" || parent.ends_with(b"Record")
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

        let calls = class_body_calls(&class);

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
