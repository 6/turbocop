use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use std::collections::HashMap;
use crate::cop::node_type::{CALL_NODE, PARENTHESES_NODE, STATEMENTS_NODE};

pub struct InverseMethods;

impl InverseMethods {
    /// Build the inverse methods map from config or defaults.
    fn build_inverse_map(config: &CopConfig) -> HashMap<Vec<u8>, String> {
        let mut map = HashMap::new();

        if let Some(configured) = config.get_string_hash("InverseMethods") {
            for (key, val) in &configured {
                let k = key.trim_start_matches(':');
                let v = val.trim_start_matches(':');
                map.insert(k.as_bytes().to_vec(), v.to_string());
            }
        } else {
            // RuboCop defaults — note: relationship only defined one direction
            // but we need both directions for lookup
            let defaults: &[(&[u8], &str)] = &[
                (b"any?", "none?"),
                (b"none?", "any?"),
                (b"even?", "odd?"),
                (b"odd?", "even?"),
                (b"==", "!="),
                (b"!=", "=="),
                (b"=~", "!~"),
                (b"!~", "=~"),
                (b"<", ">="),
                (b">=", "<"),
                (b">", "<="),
                (b"<=", ">"),
            ];
            for &(k, v) in defaults {
                map.insert(k.to_vec(), v.to_string());
            }
        }
        map
    }

    fn build_inverse_blocks(config: &CopConfig) -> HashMap<Vec<u8>, String> {
        let mut map = HashMap::new();

        if let Some(configured) = config.get_string_hash("InverseBlocks") {
            for (key, val) in &configured {
                let k = key.trim_start_matches(':');
                let v = val.trim_start_matches(':');
                map.insert(k.as_bytes().to_vec(), v.to_string());
            }
        } else {
            // RuboCop defaults
            let defaults: &[(&[u8], &str)] = &[
                (b"select", "reject"),
                (b"reject", "select"),
            ];
            for &(k, v) in defaults {
                map.insert(k.to_vec(), v.to_string());
            }
        }
        map
    }
}

impl Cop for InverseMethods {
    fn name(&self) -> &'static str {
        "Style/InverseMethods"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, PARENTHESES_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_bytes = call.name().as_slice();

        // Pattern: !receiver.method - the call is `!` with the inner being a method call
        if method_bytes != b"!" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Try to get the inner call - either directly from receiver or by unwrapping parens
        let inner_call = if let Some(c) = receiver.as_call_node() {
            c
        } else if let Some(parens) = receiver.as_parentheses_node() {
            let body = match parens.body() {
                Some(b) => b,
                None => return,
            };
            let stmts = match body.as_statements_node() {
                Some(s) => s,
                None => return,
            };
            let stmts_list: Vec<_> = stmts.body().iter().collect();
            if stmts_list.len() != 1 {
                return;
            }
            match stmts_list[0].as_call_node() {
                Some(c) => c,
                None => return,
            }
        } else {
            return;
        };

        let inner_method = inner_call.name().as_slice();

        // Check InverseMethods (predicate methods: !foo.any? -> foo.none?)
        let inverse_methods = Self::build_inverse_map(config);
        if let Some(inv) = inverse_methods.get(inner_method) {
            // Skip comparison operators when either operand is a constant (CamelCase).
            // Module#< can return nil for unrelated classes, so !(A < B) != (A >= B).
            // This matches RuboCop's `possible_class_hierarchy_check?`.
            if is_comparison_operator(inner_method) && has_constant_operand(&inner_call) {
                return;
            }

            let inner_name = std::str::from_utf8(inner_method).unwrap_or("method");
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Use `{}` instead of inverting `{}`.", inv, inner_name),
            ));
        }

        // Check InverseBlocks (block methods: !foo.select { } -> foo.reject { })
        let inverse_blocks = Self::build_inverse_blocks(config);
        if inner_call.block().is_some() {
            if let Some(inv) = inverse_blocks.get(inner_method) {
                let inner_name = std::str::from_utf8(inner_method).unwrap_or("method");
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `{}` instead of inverting `{}`.", inv, inner_name),
                ));
            }
        }

    }
}

/// Returns true if the method name is a comparison operator.
fn is_comparison_operator(method: &[u8]) -> bool {
    matches!(method, b"<" | b">" | b"<=" | b">=")
}

/// Returns true if either operand (receiver or first argument) of a call is a constant node,
/// suggesting a possible class hierarchy check (e.g., `Module < OtherModule`).
fn has_constant_operand(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(receiver) = call.receiver() {
        if receiver.as_constant_read_node().is_some() || receiver.as_constant_path_node().is_some()
        {
            return true;
        }
    }
    if let Some(args) = call.arguments() {
        for arg in args.arguments().iter() {
            if arg.as_constant_read_node().is_some() || arg.as_constant_path_node().is_some() {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InverseMethods, "cops/style/inverse_methods");
}
