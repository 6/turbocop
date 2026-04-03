use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Performance/Sum detects `inject`/`reduce` calls that can be replaced with `sum`.
///
/// Detected patterns:
/// 1. Symbol form: `inject(:+)`, `inject(0, :+)`, `inject(init, :+)`
/// 2. Block-pass form: `inject(&:+)`, `inject(0, &:+)`
/// 3. Block form: `inject(init) { |acc, elem| acc + elem }` (or `elem + acc`)
/// 4. Block form without init: `inject { |acc, elem| acc + elem }`
///
/// Root cause of FN=294: the cop previously rejected all calls with blocks,
/// missing the block-based addition pattern. Also missed non-zero initial
/// values with `:+` and `&:+` block-pass form.
///
/// FP fix (5 FP): `check_map_sum` flagged `map { it.foo }.sum` and
/// `map { _1[:bar] }.sum` — blocks using Ruby 3.4 `it` keyword or numbered
/// parameters (`_1`). In Parser gem these create `numblock`/`itblock` nodes
/// that RuboCop's `(block ...)` NodePattern doesn't match. In Prism they use
/// `NumberedParametersNode`/`ItParametersNode` inside a regular block. Fix:
/// skip blocks whose parameters are these node types.
///
/// FN fix (10 FN): Two bugs: (1) receiver-less `inject(:+)` / `reduce(0, :+)`
/// was rejected by an unnecessary `call.receiver().is_none()` guard — removed.
/// (2) `inject(0, :+) do |count| ... end` entered the block path because
/// `.block()` returned the `do..end` block, but `is_sum_block` failed.
/// Fix: check args for `:+` symbol before entering block logic.
pub struct Sum;

impl Cop for Sum {
    fn name(&self) -> &'static str {
        "Performance/Sum"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let only_sum_or_with_initial_value = config.get_bool("OnlySumOrWithInitialValue", false);
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Check for map/collect { ... }.sum pattern
        if method_name == b"sum" {
            self.check_map_sum(source, &call, diagnostics);
            return;
        }

        if method_name != b"inject" && method_name != b"reduce" {
            return;
        }

        let method_str = std::str::from_utf8(method_name).unwrap_or("inject");

        if let Some(block) = call.block() {
            // Check if args contain :+ symbol — if so, handle as symbol form
            // (the :+ takes precedence over the block, matching RuboCop's behavior).
            // This handles: inject(0, :+) do |count| ... end
            if let Some(args) = call.arguments() {
                let arg_nodes: Vec<_> = args.arguments().iter().collect();
                if arg_nodes.len() == 2 && is_plus_symbol(&arg_nodes[1]) {
                    let msg_loc = match call.message_loc() {
                        Some(loc) => loc,
                        None => return,
                    };
                    let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
                    let raw_init = get_raw_init_text(source, call.arguments()).unwrap_or_default();
                    let suggestion_init = get_suggestion_init(source, call.arguments());
                    let message =
                        format_symbol_message(method_str, &raw_init, &suggestion_init, ":+");
                    diagnostics.push(self.diagnostic(source, line, column, message));
                    return;
                }
                if arg_nodes.len() == 1 && is_plus_symbol(&arg_nodes[0]) {
                    if only_sum_or_with_initial_value {
                        return;
                    }
                    let msg_loc = match call.message_loc() {
                        Some(loc) => loc,
                        None => return,
                    };
                    let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Use `sum` instead of `{method_str}(:+)`, unless calling `{method_str}(:+)` on an empty array."),
                    ));
                    return;
                }
            }

            // Check for block-based pattern: inject(init) { |acc, elem| acc + elem }
            if let Some(block_node) = block.as_block_node() {
                if is_sum_block(&block_node) {
                    let has_init = call.arguments().is_some();
                    if only_sum_or_with_initial_value && !has_init {
                        return;
                    }
                    let msg_loc = match call.message_loc() {
                        Some(loc) => loc,
                        None => return,
                    };
                    let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
                    let raw_init = get_raw_init_text(source, call.arguments());
                    let suggestion_init = get_suggestion_init(source, call.arguments());
                    let message =
                        format_block_message(method_str, raw_init.as_deref(), &suggestion_init);
                    diagnostics.push(self.diagnostic(source, line, column, message));
                }
                return;
            }

            // Check for block-pass pattern: inject(&:+), inject(0, &:+)
            if let Some(block_arg) = block.as_block_argument_node() {
                let is_plus = block_arg
                    .expression()
                    .and_then(|e| e.as_symbol_node())
                    .is_some_and(|s| s.unescaped() == b"+");
                if !is_plus {
                    return;
                }

                let has_init = call.arguments().is_some();
                if only_sum_or_with_initial_value && !has_init {
                    return;
                }

                let msg_loc = match call.message_loc() {
                    Some(loc) => loc,
                    None => return,
                };
                let (line, column) = source.offset_to_line_col(msg_loc.start_offset());

                if has_init {
                    let raw_init = get_raw_init_text(source, call.arguments()).unwrap_or_default();
                    let suggestion_init = get_suggestion_init(source, call.arguments());
                    let message =
                        format_symbol_message(method_str, &raw_init, &suggestion_init, "&:+");
                    diagnostics.push(self.diagnostic(source, line, column, message));
                } else {
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Use `sum` instead of `{method_str}(&:+)`, unless calling `{method_str}(&:+)` on an empty array."),
                    ));
                }
                return;
            }

            return;
        }

        // No block — check symbol argument patterns: inject(:+), inject(init, :+)
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_nodes: Vec<_> = args.arguments().iter().collect();

        match arg_nodes.len() {
            1 => {
                if is_plus_symbol(&arg_nodes[0]) {
                    // inject(:+) — no initial value
                    if only_sum_or_with_initial_value {
                        return;
                    }
                    let msg_loc = match call.message_loc() {
                        Some(loc) => loc,
                        None => return,
                    };
                    let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Use `sum` instead of `{method_str}(:+)`."),
                    ));
                }
            }
            2 => {
                if is_plus_symbol(&arg_nodes[1]) {
                    let msg_loc = match call.message_loc() {
                        Some(loc) => loc,
                        None => return,
                    };
                    let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
                    let raw_init = get_raw_init_text(source, call.arguments()).unwrap_or_default();
                    let suggestion_init = get_suggestion_init(source, call.arguments());
                    let message =
                        format_symbol_message(method_str, &raw_init, &suggestion_init, ":+");
                    diagnostics.push(self.diagnostic(source, line, column, message));
                }
            }
            _ => {}
        }
    }
}

impl Sum {
    /// Check for `map/collect { ... }.sum` or `map/collect(&:method).sum` patterns
    fn check_map_sum(
        &self,
        source: &SourceFile,
        sum_call: &ruby_prism::CallNode<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // .sum must not have a block itself (except sum with no block is fine)
        // If .sum has a block or block_arg, skip: `map(&:count).sum { |x| x }` is not flagged
        if let Some(block) = sum_call.block() {
            if block.as_block_node().is_some() {
                return;
            }
            // block_arg on sum like `.sum(&:count)` — also skip
            if block.as_block_argument_node().is_some() {
                return;
            }
        }

        // Receiver must be a map/collect call
        let receiver = match sum_call.receiver() {
            Some(r) => r,
            None => return,
        };
        let map_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };
        let map_method = map_call.name().as_slice();
        if map_method != b"map" && map_method != b"collect" {
            return;
        }

        // map/collect must have a block or block_arg
        let map_block = match map_call.block() {
            Some(b) => b,
            None => return,
        };

        // Skip blocks using numbered parameters (_1, _2) or `it` keyword —
        // RuboCop's NodePattern `(block ...)` doesn't match numblock/itblock nodes
        // from Parser gem, so RuboCop doesn't flag these.
        if let Some(block_node) = map_block.as_block_node() {
            if let Some(params) = block_node.parameters() {
                if params.as_numbered_parameters_node().is_some()
                    || params.as_it_parameters_node().is_some()
                {
                    return;
                }
            }
        }

        let map_method_str = std::str::from_utf8(map_method).unwrap_or("map");

        // Get the message_loc of the map/collect call for the offense location
        let msg_loc = match map_call.message_loc() {
            Some(loc) => loc,
            None => return,
        };
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());

        // Check if .sum has an initial value argument
        let sum_init = get_raw_init_text(source, sum_call.arguments());

        let message = match &sum_init {
            Some(init) => format!(
                "Use `sum({init}) {{ ... }}` instead of `{map_method_str} {{ ... }}.sum({init})`."
            ),
            None => format!("Use `sum {{ ... }}` instead of `{map_method_str} {{ ... }}.sum`."),
        };

        diagnostics.push(self.diagnostic(source, line, column, message));
    }
}

/// Check if a block implements summation: `{ |acc, elem| acc + elem }` or `{ |acc, elem| elem + acc }`
fn is_sum_block(block: &ruby_prism::BlockNode<'_>) -> bool {
    // Must have exactly 2 block parameters
    let params = match block.parameters() {
        Some(p) => p,
        None => return false,
    };
    let block_params = match params.as_block_parameters_node() {
        Some(bp) => bp,
        None => return false,
    };
    let param_list = match block_params.parameters() {
        Some(pl) => pl,
        None => return false,
    };
    let requireds: Vec<_> = param_list.requireds().iter().collect();
    if requireds.len() != 2 {
        return false;
    }

    // Get parameter names
    let param1_name = match requireds[0].as_required_parameter_node() {
        Some(p) => p.name().as_slice().to_vec(),
        None => return false,
    };
    let param2_name = match requireds[1].as_required_parameter_node() {
        Some(p) => p.name().as_slice().to_vec(),
        None => return false,
    };

    // Block body must be a single expression that is `param1 + param2` or `param2 + param1`
    let body = match block.body() {
        Some(b) => b,
        None => return false,
    };

    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return false,
    };

    let body_stmts: Vec<_> = stmts.body().iter().collect();
    if body_stmts.len() != 1 {
        return false;
    }

    let call = match body_stmts[0].as_call_node() {
        Some(c) => c,
        None => return false,
    };

    // Must be a `+` call
    if call.name().as_slice() != b"+" {
        return false;
    }

    // Get the receiver (left operand) and the argument (right operand)
    let receiver = match call.receiver() {
        Some(r) => r,
        None => return false,
    };

    let args = match call.arguments() {
        Some(a) => a,
        None => return false,
    };

    let arg_nodes: Vec<_> = args.arguments().iter().collect();
    if arg_nodes.len() != 1 {
        return false;
    }

    let left_name = local_var_name(&receiver);
    let right_name = local_var_name(&arg_nodes[0]);

    let (left_name, right_name) = match (left_name, right_name) {
        (Some(l), Some(r)) => (l, r),
        _ => return false,
    };

    // acc + elem or elem + acc
    (left_name == param1_name && right_name == param2_name)
        || (left_name == param2_name && right_name == param1_name)
}

fn local_var_name(node: &ruby_prism::Node<'_>) -> Option<Vec<u8>> {
    node.as_local_variable_read_node()
        .map(|n| n.name().as_slice().to_vec())
}

fn is_plus_symbol(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(sym) = node.as_symbol_node() {
        return sym.unescaped() == b"+";
    }
    false
}

fn is_zero_literal(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(int) = node.as_integer_node() {
        let value = int.value();
        let (negative, digits) = value.to_u32_digits();
        return !negative && digits == [0];
    }
    false
}

fn is_zero_int(node: &ruby_prism::Node<'_>) -> bool {
    is_zero_literal(node)
}

/// Get the raw source text of the first argument (init value).
/// Returns None if no arguments.
fn get_raw_init_text(
    source: &SourceFile,
    arguments: Option<ruby_prism::ArgumentsNode<'_>>,
) -> Option<String> {
    let args = arguments?;
    let arg_nodes: Vec<_> = args.arguments().iter().collect();
    if arg_nodes.is_empty() {
        return None;
    }
    let init_node = &arg_nodes[0];
    let start = init_node.location().start_offset();
    let end = init_node.location().end_offset();
    let init_text = &source.as_bytes()[start..end];
    Some(String::from_utf8_lossy(init_text).to_string())
}

/// Get the suggestion init text (empty for integer 0, since `sum` == `sum(0)`).
fn get_suggestion_init(
    source: &SourceFile,
    arguments: Option<ruby_prism::ArgumentsNode<'_>>,
) -> String {
    let args = match arguments {
        Some(a) => a,
        None => return String::new(),
    };
    let arg_nodes: Vec<_> = args.arguments().iter().collect();
    if arg_nodes.is_empty() {
        return String::new();
    }
    let init_node = &arg_nodes[0];
    if is_zero_int(init_node) {
        return String::new();
    }
    let start = init_node.location().start_offset();
    let end = init_node.location().end_offset();
    let init_text = &source.as_bytes()[start..end];
    String::from_utf8_lossy(init_text).to_string()
}

fn format_sum_suggestion(init: &str) -> String {
    if init.is_empty() {
        "sum".to_string()
    } else {
        format!("sum({init})")
    }
}

fn format_block_message(method_str: &str, raw_init: Option<&str>, suggestion_init: &str) -> String {
    let suggestion = format_sum_suggestion(suggestion_init);
    match raw_init {
        Some(init) => format!(
            "Use `{suggestion}` instead of `{method_str}({init}) {{ |acc, elem| acc + elem }}`."
        ),
        None => {
            format!("Use `{suggestion}` instead of `{method_str} {{ |acc, elem| acc + elem }}`.")
        }
    }
}

fn format_symbol_message(
    method_str: &str,
    raw_init: &str,
    suggestion_init: &str,
    sym_str: &str,
) -> String {
    let suggestion = format_sum_suggestion(suggestion_init);
    format!("Use `{suggestion}` instead of `{method_str}({raw_init}, {sym_str})`.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full_with_config;

    crate::cop_fixture_tests!(Sum, "cops/performance/sum");

    #[test]
    fn only_sum_or_with_initial_value_skips_single_arg() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "OnlySumOrWithInitialValue".into(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };
        // inject(:+) without initial value — should NOT be flagged
        let src = b"[1, 2, 3].inject(:+)\n";
        let diags = run_cop_full_with_config(&Sum, src, config.clone());
        assert!(
            diags.is_empty(),
            "OnlySumOrWithInitialValue should skip inject(:+)"
        );

        // inject(0, :+) with initial value — SHOULD be flagged
        let src2 = b"[1, 2, 3].inject(0, :+)\n";
        let diags2 = run_cop_full_with_config(&Sum, src2, config);
        assert_eq!(
            diags2.len(),
            1,
            "OnlySumOrWithInitialValue should still flag inject(0, :+)"
        );
    }

    #[test]
    fn instance_var_inject_block() {
        let src = b"@stack.inject(0) { |n, sum| sum + n }\n";
        let diags = run_cop_full_with_config(&Sum, src, CopConfig::default());
        assert_eq!(diags.len(), 1, "should flag @stack.inject(0) block pattern");
    }
}
