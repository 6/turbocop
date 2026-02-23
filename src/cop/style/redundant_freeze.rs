use crate::cop::node_type::{
    ARRAY_NODE, CALL_NODE, FALSE_NODE, FLOAT_NODE, IMAGINARY_NODE, INTEGER_NODE,
    INTERPOLATED_STRING_NODE, NIL_NODE, PARENTHESES_NODE, RANGE_NODE, RATIONAL_NODE,
    REGULAR_EXPRESSION_NODE, STATEMENTS_NODE, STRING_NODE, SYMBOL_NODE, TRUE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantFreeze;

impl RedundantFreeze {
    fn is_immutable_literal(node: &ruby_prism::Node<'_>) -> bool {
        // Integers, floats, symbols, ranges, true, false, nil are immutable
        node.as_integer_node().is_some()
            || node.as_float_node().is_some()
            || node.as_rational_node().is_some()
            || node.as_imaginary_node().is_some()
            || node.as_symbol_node().is_some()
            || node.as_true_node().is_some()
            || node.as_false_node().is_some()
            || node.as_nil_node().is_some()
    }

    fn is_numeric(node: &ruby_prism::Node<'_>) -> bool {
        node.as_integer_node().is_some() || node.as_float_node().is_some()
    }

    fn is_string_or_array(node: &ruby_prism::Node<'_>) -> bool {
        node.as_string_node().is_some()
            || node.as_interpolated_string_node().is_some()
            || node.as_array_node().is_some()
    }

    fn is_operation_producing_immutable(node: &ruby_prism::Node<'_>) -> bool {
        // Method calls that always return immutable values (integers).
        // count/length/size always return Integer regardless of receiver.
        if let Some(call) = node.as_call_node() {
            let method = call.name();
            let name = method.as_slice();
            if matches!(name, b"count" | b"length" | b"size") {
                return true;
            }
        }
        // Parenthesized expressions containing operations.
        // Must match the vendor's patterns precisely:
        //   (begin (send {float int} {:+ :- :* :** :/ :% :<<} _))
        //   (begin (send !{(str _) array} {:+ :- :* :** :/ :%} {float int}))
        //   (begin (send _ {:== :=== :!= :<= :>= :< :>} _))
        if let Some(parens) = node.as_parentheses_node() {
            if let Some(body) = parens.body() {
                if let Some(stmts) = body.as_statements_node() {
                    let body_nodes: Vec<_> = stmts.body().into_iter().collect();
                    if body_nodes.len() == 1 {
                        let inner = &body_nodes[0];
                        if let Some(call) = inner.as_call_node() {
                            let method_name = call.name();
                            let name_bytes = method_name.as_slice();

                            // Comparison operators always produce booleans (immutable)
                            if matches!(
                                name_bytes,
                                b"<" | b">" | b"<=" | b">=" | b"==" | b"===" | b"!="
                            ) {
                                return true;
                            }

                            // Arithmetic: only when operand types guarantee numeric result
                            let is_arithmetic = matches!(
                                name_bytes,
                                b"+" | b"-" | b"*" | b"/" | b"%" | b"**" | b"<<"
                            );
                            if is_arithmetic {
                                if let Some(receiver) = call.receiver() {
                                    // Pattern 1: numeric_left op anything
                                    if Self::is_numeric(&receiver) {
                                        return true;
                                    }
                                    // Pattern 2: non_string_non_array op numeric_right
                                    if !Self::is_string_or_array(&receiver) && name_bytes != b"<<" {
                                        if let Some(args) = call.arguments() {
                                            let arg_list: Vec<_> =
                                                args.arguments().iter().collect();
                                            if arg_list.len() == 1 && Self::is_numeric(&arg_list[0])
                                            {
                                                return true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Check if a node is a regex or range literal (frozen since Ruby 3.0).
    fn is_frozen_since_ruby3(node: &ruby_prism::Node<'_>) -> bool {
        node.as_regular_expression_node().is_some()
            || node.as_interpolated_regular_expression_node().is_some()
            || node.as_range_node().is_some()
    }

    /// Check if a node is a plain (non-interpolated) string literal.
    fn is_plain_string(node: &ruby_prism::Node<'_>) -> bool {
        node.as_string_node().is_some()
    }

    /// Check if the source file has `# frozen_string_literal: true` in the
    /// first few lines (before any code).
    fn has_frozen_string_literal_true(source: &SourceFile) -> bool {
        let lines = source.lines();
        for (i, line) in lines.enumerate() {
            if i >= 3 {
                break;
            }
            let s = match std::str::from_utf8(line) {
                Ok(s) => s.trim(),
                Err(_) => continue,
            };
            if s.is_empty() {
                continue;
            }
            if let Some(rest) = s.strip_prefix('#') {
                let rest = rest.trim_start();
                if let Some(value) = rest.strip_prefix("frozen_string_literal:") {
                    return value.trim() == "true";
                }
            }
        }
        false
    }

    /// Check a predicate on the node, stripping one layer of parentheses first
    /// (matching vendor's strip_parenthesis behavior).
    fn check_stripped<F>(node: &ruby_prism::Node<'_>, predicate: F) -> bool
    where
        F: Fn(&ruby_prism::Node<'_>) -> bool,
    {
        if let Some(parens) = node.as_parentheses_node() {
            if let Some(body) = parens.body() {
                if let Some(stmts) = body.as_statements_node() {
                    let body_nodes: Vec<_> = stmts.body().into_iter().collect();
                    if body_nodes.len() == 1 {
                        return predicate(&body_nodes[0]);
                    }
                }
            }
            return false;
        }
        predicate(node)
    }
}

impl Cop for RedundantFreeze {
    fn name(&self) -> &'static str {
        "Style/RedundantFreeze"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ARRAY_NODE,
            CALL_NODE,
            FALSE_NODE,
            FLOAT_NODE,
            IMAGINARY_NODE,
            INTEGER_NODE,
            INTERPOLATED_STRING_NODE,
            NIL_NODE,
            PARENTHESES_NODE,
            RANGE_NODE,
            RATIONAL_NODE,
            REGULAR_EXPRESSION_NODE,
            STATEMENTS_NODE,
            STRING_NODE,
            SYMBOL_NODE,
            TRUE_NODE,
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
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be a call to `.freeze` with no arguments
        if call_node.name().as_slice() != b"freeze" {
            return;
        }
        if call_node.arguments().is_some() {
            return;
        }

        // Must have a receiver
        let receiver = match call_node.receiver() {
            Some(r) => r,
            None => return,
        };

        let target_ruby_version = config
            .options
            .get("TargetRubyVersion")
            .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
            .unwrap_or(2.7);

        let frozen_strings = Self::has_frozen_string_literal_true(source);

        // Check if the receiver is an immutable literal (strip parens like vendor)
        let is_immutable = Self::check_stripped(&receiver, Self::is_immutable_literal)
            || (target_ruby_version >= 3.0
                && Self::check_stripped(&receiver, Self::is_frozen_since_ruby3))
            || (frozen_strings && Self::check_stripped(&receiver, Self::is_plain_string))
            || Self::is_operation_producing_immutable(&receiver);

        if !is_immutable {
            return;
        }

        let loc = receiver.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Do not freeze immutable objects, as freezing them has no effect.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    crate::cop_fixture_tests!(RedundantFreeze, "cops/style/redundant_freeze");

    fn config_ruby30() -> CopConfig {
        let mut options = HashMap::new();
        options.insert(
            "TargetRubyVersion".to_string(),
            serde_yml::Value::Number(serde_yml::Number::from(3.0)),
        );
        CopConfig {
            options,
            ..CopConfig::default()
        }
    }

    #[test]
    fn regex_literal_frozen_ruby30() {
        let source = b"PATTERN = /foo/.freeze\n";
        let diags =
            crate::testutil::run_cop_full_with_config(&RedundantFreeze, source, config_ruby30());
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense, got {}: {:?}",
            diags.len(),
            diags
        );
    }

    #[test]
    fn regex_literal_with_flags_frozen_ruby30() {
        let source = b"PATTERN = /bar|baz/i.freeze\n";
        let diags =
            crate::testutil::run_cop_full_with_config(&RedundantFreeze, source, config_ruby30());
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense, got {}: {:?}",
            diags.len(),
            diags
        );
    }

    #[test]
    fn range_literal_frozen_ruby30() {
        let source = b"RANGE = (1..10).freeze\n";
        let diags =
            crate::testutil::run_cop_full_with_config(&RedundantFreeze, source, config_ruby30());
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense, got {}: {:?}",
            diags.len(),
            diags
        );
    }

    #[test]
    fn exclusive_range_literal_frozen_ruby30() {
        let source = b"RANGE = (1...10).freeze\n";
        let diags =
            crate::testutil::run_cop_full_with_config(&RedundantFreeze, source, config_ruby30());
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense, got {}: {:?}",
            diags.len(),
            diags
        );
    }

    #[test]
    fn regex_not_flagged_ruby27() {
        // Regex is not frozen before Ruby 3.0
        let source = b"PATTERN = /foo/.freeze\n";
        let diags = crate::testutil::run_cop_full(&RedundantFreeze, source);
        assert_eq!(
            diags.len(),
            0,
            "Expected 0 offenses, got {}: {:?}",
            diags.len(),
            diags
        );
    }

    #[test]
    fn range_not_flagged_ruby27() {
        // Range is not frozen before Ruby 3.0
        let source = b"RANGE = (1..10).freeze\n";
        let diags = crate::testutil::run_cop_full(&RedundantFreeze, source);
        assert_eq!(
            diags.len(),
            0,
            "Expected 0 offenses, got {}: {:?}",
            diags.len(),
            diags
        );
    }

    #[test]
    fn interpolated_string_not_flagged_with_frozen_literal() {
        // Interpolated strings remain mutable even with frozen_string_literal: true (Ruby >= 3.0)
        let source = b"# frozen_string_literal: true\nINTERP = \"top#{1 + 2}\".freeze\n";
        let diags =
            crate::testutil::run_cop_full_with_config(&RedundantFreeze, source, config_ruby30());
        assert_eq!(
            diags.len(),
            0,
            "Expected 0 offenses for interpolated string, got {}: {:?}",
            diags.len(),
            diags
        );
    }

    #[test]
    fn plain_string_not_flagged_without_magic_comment() {
        // Without frozen_string_literal: true, plain strings are mutable
        let source = b"CONST = 'hello'.freeze\n";
        let diags = crate::testutil::run_cop_full(&RedundantFreeze, source);
        assert_eq!(
            diags.len(),
            0,
            "Expected 0 offenses without magic comment, got {}: {:?}",
            diags.len(),
            diags
        );
    }

    #[test]
    fn plain_string_flagged_with_frozen_string_literal() {
        let source = b"# frozen_string_literal: true\nCONST = 'hello'.freeze\n";
        let diags = crate::testutil::run_cop_full(&RedundantFreeze, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense with magic comment, got {}: {:?}",
            diags.len(),
            diags
        );
    }
}
