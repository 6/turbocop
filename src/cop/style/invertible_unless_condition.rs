use crate::cop::node_type::{
    AND_NODE, BEGIN_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, OR_NODE,
    PARENTHESES_NODE, STATEMENTS_NODE, UNLESS_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use std::collections::HashMap;

/// Corpus investigation (2026-03-23):
///
/// FN=624 root cause: compound conditions (AND/OR at top level) never produced
/// diagnostics. The `is_fully_invertible` check correctly recurses into AND/OR
/// nodes, but the message-building code only handled top-level CallNode predicates,
/// falling through to `return` for any other node type. This dropped all compound
/// conditions like `unless x.present? && y.present?`.
///
/// Additional FN: methods called without an explicit receiver (implicit self),
/// like `unless odd?`, were handled by `is_fully_invertible` but the message
/// builder didn't account for them in compound conditions.
///
/// FP=51 root cause: for `!` negation conditions, two diagnostics were emitted —
/// one in the `!` branch (line 183) and another after falling through (line 203).
/// The second diagnostic tried to look up `!` in the inverse map and produced a
/// malformed message. Additionally, the inheritance check for `<` operator did not
/// match RuboCop's behavior of allowing `x < FOO` (all-uppercase constants are
/// NOT inheritance checks).
///
/// Fix: rewrote message generation to use a recursive `preferred_condition()` method
/// matching RuboCop's approach, and changed message format to match RuboCop's
/// `Prefer 'if <preferred>' over 'unless <current>'.` format.
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
    fn is_fully_invertible(
        node: &ruby_prism::Node<'_>,
        inverse_map: &HashMap<Vec<u8>, String>,
    ) -> bool {
        // Negation: `!expr` — always invertible (just remove the `!`)
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"!" {
                return true;
            }

            // Safe-navigation calls (`&.method`) are not invertible — RuboCop only
            // handles `:send` nodes, not `:csend` (safe-navigation) nodes.
            if call
                .call_operator_loc()
                .is_some_and(|op: ruby_prism::Location<'_>| op.as_slice() == b"&.")
            {
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
                if call.name().as_slice() == b"<" && Self::is_inheritance_check(&call) {
                    return false;
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

    /// Check if a `<` call is a class inheritance check.
    /// RuboCop: `node.method?(:<) && argument.const_type? &&
    ///   argument.short_name.to_s.upcase != argument.short_name.to_s`
    /// This means `x < Foo` is inheritance (Foo is not all-uppercase),
    /// but `x < FOO` is NOT inheritance (FOO is all-uppercase).
    fn is_inheritance_check(call: &ruby_prism::CallNode<'_>) -> bool {
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() == 1 {
                // Get the constant name to check if it's all-uppercase
                let const_name = if let Some(cr) = arg_list[0].as_constant_read_node() {
                    Some(cr.name())
                } else if let Some(cp) = arg_list[0].as_constant_path_node() {
                    // For Foo::Bar, use the last segment (Bar)
                    cp.name()
                } else {
                    None
                };

                if let Some(name) = const_name {
                    let name_str = std::str::from_utf8(name.as_slice()).unwrap_or("");
                    // All-uppercase names (like FOO, VERSION) are NOT inheritance checks
                    // Only mixed-case names (like Foo, FooBar) are inheritance checks
                    return name_str.to_uppercase() != name_str;
                }
            }
        }
        false
    }

    /// Build the preferred (inverted) condition string for the message.
    /// Matches RuboCop's `preferred_condition` method.
    fn preferred_condition(
        node: &ruby_prism::Node<'_>,
        inverse_map: &HashMap<Vec<u8>, String>,
    ) -> String {
        // Parentheses: wrap the inner condition
        if let Some(paren) = node.as_parentheses_node() {
            if let Some(body) = paren.body() {
                if let Some(stmts) = body.as_statements_node() {
                    let body_list: Vec<_> = stmts.body().iter().collect();
                    if body_list.len() == 1 {
                        let inner = Self::preferred_condition(&body_list[0], inverse_map);
                        return format!("({})", inner);
                    }
                }
            }
            return String::from_utf8_lossy(node.location().as_slice()).to_string();
        }

        // AND/OR: invert both sides and swap the operator
        if let Some(and_node) = node.as_and_node() {
            let left = Self::preferred_condition(&and_node.left(), inverse_map);
            let right = Self::preferred_condition(&and_node.right(), inverse_map);
            return format!("{} || {}", left, right);
        }
        if let Some(or_node) = node.as_or_node() {
            let left = Self::preferred_condition(&or_node.left(), inverse_map);
            let right = Self::preferred_condition(&or_node.right(), inverse_map);
            return format!("{} && {}", left, right);
        }

        // Call node
        if let Some(call) = node.as_call_node() {
            let name_bytes = call.name().as_slice();

            // `!expr` → just the receiver
            if name_bytes == b"!" {
                return call
                    .receiver()
                    .map(|r| String::from_utf8_lossy(r.location().as_slice()).to_string())
                    .unwrap_or_default();
            }

            let receiver_source = call
                .receiver()
                .map(|r| String::from_utf8_lossy(r.location().as_slice()).to_string());

            if let Some(inv) = inverse_map.get(name_bytes) {
                // Check if the method has arguments
                let has_args = call.arguments().is_some_and(|a| !a.arguments().is_empty());

                if has_args {
                    let args = call.arguments().unwrap();
                    let arg_source: Vec<String> = args
                        .arguments()
                        .iter()
                        .map(|a| String::from_utf8_lossy(a.location().as_slice()).to_string())
                        .collect();
                    let arg_list = arg_source.join(", ");

                    // Check if it's an operator method (has receiver and no message_loc dot)
                    let is_operator = Self::is_operator_method(name_bytes);
                    if is_operator {
                        let recv = receiver_source.unwrap_or_default();
                        return format!("{} {} {}", recv, inv, arg_list);
                    }

                    // Check if parenthesized
                    let is_parenthesized = call.opening_loc().is_some();
                    let dotted_receiver = if let Some(ref r) = receiver_source {
                        format!("{}.", r)
                    } else {
                        String::new()
                    };

                    if is_parenthesized {
                        return format!("{}{}({})", dotted_receiver, inv, arg_list);
                    }
                    return format!("{}{} {}", dotted_receiver, inv, arg_list);
                }

                // No arguments — simple method call
                let dotted_receiver = if let Some(ref r) = receiver_source {
                    format!("{}.", r)
                } else {
                    String::new()
                };
                return format!("{}{}", dotted_receiver, inv);
            }
        }

        // Fallback: return source as-is
        String::from_utf8_lossy(node.location().as_slice()).to_string()
    }

    /// Check if a method name is an operator (like !=, >, >=, etc.)
    fn is_operator_method(name: &[u8]) -> bool {
        matches!(
            name,
            b"!=" | b"==" | b">" | b">=" | b"<" | b"<=" | b"!~" | b"=~"
        )
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

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            AND_NODE,
            BEGIN_NODE,
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            OR_NODE,
            PARENTHESES_NODE,
            STATEMENTS_NODE,
            UNLESS_NODE,
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
        let unless_node = match node.as_unless_node() {
            Some(u) => u,
            None => return,
        };

        let inverse_map = Self::build_inverse_map(config);

        let predicate = unless_node.predicate();

        // The entire condition must be invertible for us to report
        if !Self::is_fully_invertible(&predicate, &inverse_map) {
            return;
        }

        // Check for begin-wrapped conditions — don't flag those
        // RuboCop: `when :begin` recurses but `begin x end` (explicit begin) is skipped
        if predicate.as_begin_node().is_some() {
            return;
        }

        // Build the preferred condition and the current condition source
        let preferred = Self::preferred_condition(&predicate, &inverse_map);
        let current_src = String::from_utf8_lossy(predicate.location().as_slice()).to_string();

        // Determine the inverse keyword: "unless" → "if"
        let inverse_keyword = "if";
        let keyword = "unless";

        let message = format!(
            "Prefer `{} {}` over `{} {}`.",
            inverse_keyword, preferred, keyword, current_src
        );

        let loc = unless_node.keyword_loc();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, message));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        InvertibleUnlessCondition,
        "cops/style/invertible_unless_condition"
    );
}
