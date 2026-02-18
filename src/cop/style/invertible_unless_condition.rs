use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use std::collections::HashMap;

pub struct InvertibleUnlessCondition;

impl InvertibleUnlessCondition {
    /// Build the inverse methods map from config or defaults.
    fn build_inverse_map(config: &CopConfig) -> HashMap<Vec<u8>, String> {
        let mut map = HashMap::new();

        if let Some(configured) = config.get_string_hash("InverseMethods") {
            for (key, val) in &configured {
                // Config keys are like ":!=" => ":==" — strip leading colons
                let k = key.trim_start_matches(':');
                let v = val.trim_start_matches(':');
                map.insert(k.as_bytes().to_vec(), v.to_string());
            }
        } else {
            // RuboCop defaults from vendor/rubocop/config/default.yml
            let defaults: &[(&[u8], &str)] = &[
                (b"!=", "=="),
                (b">", "<="),
                (b"<=", ">"),
                (b"<", ">="),
                (b">=", "<"),
                (b"!~", "=~"),
                (b"zero?", "nonzero?"),
                (b"nonzero?", "zero?"),
                (b"any?", "none?"),
                (b"none?", "any?"),
                (b"even?", "odd?"),
                (b"odd?", "even?"),
            ];
            for &(k, v) in defaults {
                map.insert(k.to_vec(), v.to_string());
            }
        }
        map
    }

    /// Check if every method call in a condition tree is invertible.
    /// Returns true only if the entire condition can be inverted.
    fn is_fully_invertible(node: &ruby_prism::Node<'_>, inverse_map: &HashMap<Vec<u8>, String>) -> bool {
        // Negation: `!expr` — always invertible (just remove the `!`)
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"!" {
                return true;
            }

            // Safe-navigation calls (`&.method`) are not invertible — RuboCop only
            // handles `:send` nodes, not `:csend` (safe-navigation) nodes.
            if call.call_operator_loc().is_some_and(|op: ruby_prism::Location<'_>| op.as_slice() == b"&.") {
                return false;
            }

            // Calls with blocks (e.g., `any? { |x| ... }`) are not invertible —
            // in RuboCop's AST, block calls are `:block` nodes, not `:send` nodes.
            if call.block().is_some() {
                return false;
            }

            // Check if the method has an inverse in our map
            if inverse_map.contains_key(call.name().as_slice()) {
                // For `<` operator: check if the receiver is a constant (class inheritance check)
                // `x < Foo` is class inheritance, not a numeric comparison
                if call.name().as_slice() == b"<" {
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if arg_list.len() == 1 {
                            if arg_list[0].as_constant_read_node().is_some()
                                || arg_list[0].as_constant_path_node().is_some()
                            {
                                return false; // Class inheritance check, not invertible
                            }
                        }
                    }
                }
                return true;
            }
            return false;
        }

        // Parentheses: just check inner expression
        if let Some(paren) = node.as_parentheses_node() {
            if let Some(body) = paren.body() {
                if let Some(stmts) = body.as_statements_node() {
                    let body_list: Vec<_> = stmts.body().iter().collect();
                    if body_list.len() == 1 {
                        return Self::is_fully_invertible(&body_list[0], inverse_map);
                    }
                }
            }
            return false;
        }

        // `&&` / `||` — both sides must be invertible
        if let Some(and_node) = node.as_and_node() {
            return Self::is_fully_invertible(&and_node.left(), inverse_map)
                && Self::is_fully_invertible(&and_node.right(), inverse_map);
        }
        if let Some(or_node) = node.as_or_node() {
            return Self::is_fully_invertible(&or_node.left(), inverse_map)
                && Self::is_fully_invertible(&or_node.right(), inverse_map);
        }

        false
    }
}

impl Cop for InvertibleUnlessCondition {
    fn name(&self) -> &'static str {
        "Style/InvertibleUnlessCondition"
    }

    /// This cop is disabled by default in RuboCop (Enabled: false in vendor config/default.yml).
    fn default_enabled(&self) -> bool {
        false
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let unless_node = match node.as_unless_node() {
            Some(u) => u,
            None => return Vec::new(),
        };

        let inverse_map = Self::build_inverse_map(config);

        let predicate = unless_node.predicate();

        // The entire condition must be invertible for us to report
        if !Self::is_fully_invertible(&predicate, &inverse_map) {
            return Vec::new();
        }

        // Check for begin-wrapped conditions — don't flag those
        if predicate.as_begin_node().is_some() {
            return Vec::new();
        }

        // Build a descriptive message
        let method_str = if let Some(call) = predicate.as_call_node() {
            if call.name().as_slice() == b"!" {
                let receiver_src = call.receiver()
                    .map(|r| String::from_utf8_lossy(r.location().as_slice()).to_string())
                    .unwrap_or_default();
                let cond_src = String::from_utf8_lossy(predicate.location().as_slice());
                let loc = unless_node.keyword_loc();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Prefer `if {}` over `unless {}`.", receiver_src, cond_src),
                )];
            }
            let name_bytes = call.name().as_slice();
            let name = std::str::from_utf8(name_bytes).unwrap_or("method");
            let inv = inverse_map.get(name_bytes).map(|s| s.as_str()).unwrap_or("?");
            (name.to_string(), inv.to_string())
        } else {
            return Vec::new();
        };

        let loc = unless_node.keyword_loc();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Use `if` with `{}` instead of `unless` with `{}`.", method_str.1, method_str.0),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InvertibleUnlessCondition, "cops/style/invertible_unless_condition");
}
