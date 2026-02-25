use std::collections::HashMap;

use crate::cop::node_type::{
    CALL_NODE, CLASS_NODE, CONSTANT_PATH_WRITE_NODE, CONSTANT_WRITE_NODE, DEF_NODE,
    STATEMENTS_NODE, SYMBOL_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

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

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CLASS_NODE,
            CONSTANT_PATH_WRITE_NODE,
            CONSTANT_WRITE_NODE,
            DEF_NODE,
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
        let class_node = match node.as_class_node() {
            Some(c) => c,
            None => return,
        };

        let body = match class_node.body() {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
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

        let all_stmts: Vec<_> = stmts.body().iter().collect();

        for (idx, stmt) in all_stmts.iter().enumerate() {
            // Track visibility changes (bare private/protected/public without args)
            if let Some(call) = stmt.as_call_node() {
                if call.receiver().is_none() && call.arguments().is_none() {
                    let name = call.name().as_slice();
                    match name {
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
) -> Option<String> {
    // Send nodes (method calls): look up in categories
    if let Some(call) = stmt.as_call_node() {
        if call.receiver().is_none() {
            let name_bytes = call.name().as_slice();
            let name = std::str::from_utf8(name_bytes).unwrap_or("");

            // Handle def modifiers: `private def foo` / `public def bar`
            if matches!(name, "private" | "protected" | "public") && call.arguments().is_some() {
                return Some(format!("{name}_methods"));
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
        return Some(format!("{current_visibility}_methods"));
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
            if call.receiver().is_none() && call.name().as_slice() == b"private_constant" {
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
