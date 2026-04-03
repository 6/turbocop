use std::collections::HashMap;

use crate::cop::shared::access_modifier_predicates;
use crate::cop::shared::method_dispatch_predicates;
use crate::cop::shared::node_type::{
    CALL_NODE, CLASS_NODE, CONSTANT_PATH_WRITE_NODE, CONSTANT_WRITE_NODE, DEF_NODE,
    SINGLETON_CLASS_NODE, STATEMENTS_NODE, SYMBOL_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Layout/ClassStructure: checks class body element ordering.
///
/// ## Investigation findings (2026-03-11)
///
/// **Root cause of 352 FPs:** `private :method_name` (and `protected :sym`) was
/// misclassified as `private_methods`/`protected_methods`. In RuboCop, only
/// `private def foo` (where the argument is a def node — `def_modifier?`) gets
/// classified as `{vis}_methods`. Bare `private :foo` is a visibility declaration
/// that resolves to the plain method name `"private"`, which is not in
/// `ExpectedOrder` and gets ignored. The fix checks `as_def_node()` on the first
/// argument before classifying as a def modifier.
///
/// **FN source (128 FNs):** nitrocop did not handle `SingletonClassNode`
/// (`class << self`). RuboCop uses `alias on_sclass on_class` to process both.
/// Added `SINGLETON_CLASS_NODE` to interested types and handling in `check_node`.
///
/// **Remaining gaps:** None known for default config. Custom `Categories` /
/// `ExpectedOrder` configs may reveal edge cases in the category lookup.
///
/// ## Investigation findings (2026-03-23)
///
/// **Root cause of 312 FNs:** `private :method_name` and `protected :method_name`
/// (inline visibility declarations with symbol args) were not affecting the
/// classification of the corresponding `def` node. In RuboCop,
/// `VisibilityHelp#node_visibility_from_visibility_inline_on_method_name` looks
/// at right siblings of a `def` node for matching `private/protected :name`
/// declarations, making the def classified as `private_methods` or
/// `protected_methods`. Added `find_inline_visibility()` to replicate this.
///
/// **Root cause of 1 FP:** Send nodes with a receiver (e.g. `singleton_class.prepend`)
/// were not being classified. RuboCop classifies ALL send nodes by `method_name`
/// regardless of receiver. Removed the `receiver().is_none()` gate from the
/// category-lookup path in `classify_statement`, so e.g. `singleton_class.prepend`
/// is classified as `module_inclusion` (matching RuboCop). This prevents the
/// ordering tracker from missing intermediate classifications.
///
/// ## Investigation findings (2026-03-24)
///
/// **Root cause of 27 FPs:** `find_inline_visibility` was matching multi-argument
/// visibility calls like `public(:method1, :method2)`. RuboCop's
/// `visibility_inline_on_method_name?` node pattern `(send nil? VISIBILITY_SCOPES
/// (sym %method_name))` only matches single-argument calls. With multiple args,
/// the methods keep their section visibility (e.g., `private`), so no ordering
/// violation occurs. Fixed by requiring exactly one argument in
/// `find_inline_visibility`.
///
/// ## Investigation findings (2026-03-30)
///
/// **Root cause of 1 FN:** when a class body consisted of a single block-backed
/// call like `initializer "x" do ... end`, nitrocop only iterated the outer
/// `CallNode`. RuboCop's `class_elements` effectively compares both the
/// `initializer` send and the single block body node, which means
/// `ActionView::Base.include ...` inside that block can be ordered against the
/// initializer itself. Fixed by expanding only single-statement class bodies
/// with a real `BlockNode`, leaving multi-statement class bodies unchanged.
pub struct ClassStructure;

/// Default expected order (matches vendor/rubocop/config/default.yml).
const DEFAULT_EXPECTED_ORDER: &[&str] = &[
    "module_inclusion",
    "constants",
    "public_class_methods",
    "initializer",
    "public_methods",
    "protected_methods",
    "private_methods",
];

/// Default categories (matches vendor/rubocop/config/default.yml).
/// Maps method names to category names.
fn default_categories() -> HashMap<String, String> {
    let mut m = HashMap::new();
    for name in &["include", "prepend", "extend"] {
        m.insert(name.to_string(), "module_inclusion".to_string());
    }
    m
}

impl Cop for ClassStructure {
    fn name(&self) -> &'static str {
        "Layout/ClassStructure"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CLASS_NODE,
            CONSTANT_PATH_WRITE_NODE,
            CONSTANT_WRITE_NODE,
            DEF_NODE,
            SINGLETON_CLASS_NODE,
            STATEMENTS_NODE,
            SYMBOL_NODE,
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
        // Handle both ClassNode and SingletonClassNode (class << self),
        // matching RuboCop's `alias on_sclass on_class`.
        let body = if let Some(class_node) = node.as_class_node() {
            class_node.body()
        } else if let Some(sclass_node) = node.as_singleton_class_node() {
            sclass_node.body()
        } else {
            return;
        };

        let body = match body {
            Some(b) => b,
            None => return,
        };

        // Build expected order from config (or defaults).
        let expected_order: Vec<String> =
            config.get_string_array("ExpectedOrder").unwrap_or_else(|| {
                DEFAULT_EXPECTED_ORDER
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            });

        // Build categories from config: category_name -> [method_names].
        // Then invert to method_name -> category_name for fast lookup.
        let method_to_category = build_method_to_category(config);

        let mut current_visibility = "public";
        let mut previous_index: Option<usize> = None;

        let all_stmts = class_elements(body);

        for (idx, stmt) in all_stmts.iter().enumerate() {
            // Track visibility changes (bare private/protected/public without args)
            if let Some(call) = stmt.as_call_node() {
                if access_modifier_predicates::is_bare_access_modifier(&call) {
                    match call.name().as_slice() {
                        b"protected" => {
                            current_visibility = "protected";
                            continue;
                        }
                        b"private" => {
                            current_visibility = "private";
                            continue;
                        }
                        b"public" => {
                            current_visibility = "public";
                            continue;
                        }
                        _ => {}
                    }
                }
            }

            let classification = classify_statement(
                stmt,
                current_visibility,
                &method_to_category,
                &expected_order,
                &all_stmts,
                idx,
            );

            // Determine whether to ignore this node (matching RuboCop's ignore? method)
            let classification = match classification {
                Some(c) if c.ends_with('=') => continue,
                Some(c) => c,
                None => continue,
            };

            // Skip if classification is not in expected order
            let order_index = match expected_order.iter().position(|e| e == &classification) {
                Some(i) => i,
                None => continue,
            };

            // Skip private constants
            if classification == "constants" && is_private_constant(stmt, &all_stmts, idx) {
                continue;
            }

            if let Some(prev) = previous_index {
                if order_index < prev {
                    let (line, col) = source.offset_to_line_col(stmt.location().start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        col,
                        format!(
                            "`{}` is supposed to appear before `{}`.",
                            classification, expected_order[prev]
                        ),
                    ));
                }
            }
            // Always update previous_index (matching RuboCop behavior)
            previous_index = Some(order_index);
        }
    }
}

/// Mirror RuboCop's `class_elements` for Prism:
/// - normal multi-statement class bodies use their top-level statements
/// - a single block-backed call exposes both the outer call and the block body
fn class_elements<'a>(body: ruby_prism::Node<'a>) -> Vec<ruby_prism::Node<'a>> {
    if let Some(stmts) = body.as_statements_node() {
        let stmts: Vec<_> = stmts.body().iter().collect();
        if stmts.len() == 1 {
            return expand_block_body(stmts.into_iter().next().unwrap());
        }
        return stmts;
    }

    expand_block_body(body)
}

fn expand_block_body<'a>(stmt: ruby_prism::Node<'a>) -> Vec<ruby_prism::Node<'a>> {
    let block_body = stmt
        .as_call_node()
        .and_then(|call| call.block())
        .and_then(|block| block.as_block_node())
        .and_then(|block| block.body())
        .map(single_statement_or_self);

    let mut elements = vec![stmt];
    if let Some(body) = block_body {
        elements.push(body);
    }

    elements
}

fn single_statement_or_self<'a>(node: ruby_prism::Node<'a>) -> ruby_prism::Node<'a> {
    if let Some(stmts) = node.as_statements_node() {
        if stmts.body().len() == 1 {
            return stmts.body().iter().next().unwrap();
        }
    }

    node
}

/// Build a map from method_name -> category_name from the Categories config.
/// Categories config is a YAML mapping: { category_name: [method_name, ...] }.
fn build_method_to_category(config: &CopConfig) -> HashMap<String, String> {
    if let Some(val) = config.options.get("Categories") {
        if let Some(mapping) = val.as_mapping() {
            let mut result = HashMap::new();
            for (k, v) in mapping.iter() {
                if let Some(category_name) = k.as_str() {
                    if let Some(methods) = v.as_sequence() {
                        for method in methods {
                            if let Some(name) = method.as_str() {
                                result.insert(name.to_string(), category_name.to_string());
                            }
                        }
                    }
                }
            }
            return result;
        }
    }
    default_categories()
}

/// Classify a statement node into a category string.
/// Returns None for nodes that should be ignored.
fn classify_statement(
    stmt: &ruby_prism::Node<'_>,
    current_visibility: &str,
    method_to_category: &HashMap<String, String>,
    expected_order: &[String],
    all_stmts: &[ruby_prism::Node<'_>],
    idx: usize,
) -> Option<String> {
    // Send nodes (method calls): look up in categories.
    // RuboCop classifies ALL send nodes by method_name (even with a receiver),
    // so we handle both receiver and no-receiver cases.
    if let Some(call) = stmt.as_call_node() {
        let name_bytes = call.name().as_slice();
        let name = std::str::from_utf8(name_bytes).unwrap_or("");

        // Handle def modifiers and visibility declarations (receiver-less only)
        if call.receiver().is_none() {
            // Handle def modifiers: `private def foo` / `public def bar`
            // Only classify as {vis}_methods when the argument is an actual def node.
            // `private :foo` (symbol arg) is a visibility declaration, NOT a def modifier —
            // it should be ignored (not in ExpectedOrder), matching RuboCop's def_modifier? check.
            if matches!(name, "private" | "protected" | "public") {
                if let Some(args) = call.arguments() {
                    let first_arg = args.arguments().iter().next();
                    if first_arg.is_some_and(|a| a.as_def_node().is_some()) {
                        return Some(format!("{name}_methods"));
                    }
                    // Fall through: `private :foo` is not a def modifier,
                    // classify as plain method name (will be skipped if not in expected_order)
                }
            }
        }

        // Check if this method name is in any category
        let category = method_to_category.get(name);
        let key = category.map_or(name, |c| c.as_str());

        // Build visibility-prefixed key (e.g., "public_module_inclusion")
        let vis_key = format!("{current_visibility}_{key}");
        // If the visibility-prefixed form is in expected_order, use it;
        // otherwise use the plain key (matching RuboCop's find_send_node_category)
        if expected_order.iter().any(|e| e == &vis_key) {
            return Some(vis_key);
        }
        return Some(key.to_string());
    }

    // Constants
    if stmt.as_constant_write_node().is_some() || stmt.as_constant_path_write_node().is_some() {
        // Check if "constants" is mapped to a custom category
        let key = "constants";
        if let Some(category) = method_to_category.get(key) {
            return Some(category.clone());
        }
        return Some(key.to_string());
    }

    // Method definitions
    if let Some(def) = stmt.as_def_node() {
        if def.receiver().is_some() {
            return Some("public_class_methods".to_string());
        }
        if def.name().as_slice() == b"initialize" {
            return Some("initializer".to_string());
        }
        // Check for inline visibility declarations among right siblings:
        // `private :method_name` or `protected :method_name` that match this def's name.
        // This mirrors RuboCop's VisibilityHelp#node_visibility_from_visibility_inline_on_method_name.
        let def_name = def.name().as_slice();
        let inline_vis = find_inline_visibility(def_name, all_stmts, idx);
        let vis = inline_vis.unwrap_or(current_visibility);
        return Some(format!("{vis}_methods"));
    }

    None
}

/// Check right siblings for an inline visibility declaration matching the given method name.
/// e.g., `private :foo` or `protected :bar` after `def foo` / `def bar`.
/// Returns the visibility string ("private", "protected", or "public") if found, None otherwise.
///
/// Only matches single-argument visibility calls: `private :foo`, NOT `private :foo, :bar`.
/// This mirrors RuboCop's `visibility_inline_on_method_name?` node pattern:
///   `(send nil? VISIBILITY_SCOPES (sym %method_name))`
/// which requires exactly one symbol argument. Multi-argument calls like
/// `public(:method1, :method2)` are NOT recognized as inline visibility overrides
/// by RuboCop and should not be matched here.
fn find_inline_visibility<'a>(
    def_name: &[u8],
    all_stmts: &[ruby_prism::Node<'a>],
    idx: usize,
) -> Option<&'static str> {
    // Search right siblings (nodes after this def in the class body)
    for sibling in &all_stmts[idx + 1..] {
        if let Some(call) = sibling.as_call_node() {
            if access_modifier_predicates::is_access_modifier_declaration(&call) {
                let vis = match call.name().as_slice() {
                    b"private" => "private",
                    b"protected" => "protected",
                    b"public" => "public",
                    _ => continue,
                };
                // Only match single-argument calls (matching RuboCop's pattern)
                if let Some(args) = call.arguments() {
                    let mut args_iter = args.arguments().iter();
                    let first = args_iter.next();
                    let second = args_iter.next();
                    // Must have exactly one argument that is a symbol matching the def name
                    if second.is_none() {
                        if let Some(arg) = first {
                            if let Some(sym) = arg.as_symbol_node() {
                                if sym.unescaped() == def_name {
                                    return Some(vis);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Check if a constant assignment has a `private_constant :NAME` call among its siblings.
fn is_private_constant(
    stmt: &ruby_prism::Node<'_>,
    all_stmts: &[ruby_prism::Node<'_>],
    idx: usize,
) -> bool {
    // Get the constant name
    let const_name = if let Some(cw) = stmt.as_constant_write_node() {
        cw.name().as_slice().to_vec()
    } else if let Some(cpw) = stmt.as_constant_path_write_node() {
        // For constant path writes, get the last component
        let target = cpw.target();
        let bytes = target.location().as_slice();
        // Extract just the last name after ::
        if let Some(pos) = bytes.windows(2).rposition(|w| w == b"::") {
            bytes[pos + 2..].to_vec()
        } else {
            bytes.to_vec()
        }
    } else {
        return false;
    };

    // Check subsequent siblings for `private_constant :NAME`
    for sibling in &all_stmts[idx + 1..] {
        if let Some(call) = sibling.as_call_node() {
            if method_dispatch_predicates::is_command(&call, b"private_constant") {
                if let Some(args) = call.arguments() {
                    for arg in args.arguments().iter() {
                        if let Some(sym) = arg.as_symbol_node() {
                            if sym.unescaped() == const_name.as_slice() {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(ClassStructure, "cops/layout/class_structure");
}
