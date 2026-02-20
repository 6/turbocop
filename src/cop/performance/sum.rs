use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, INTEGER_NODE, SYMBOL_NODE};

pub struct Sum;

impl Cop for Sum {
    fn name(&self) -> &'static str {
        "Performance/Sum"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, INTEGER_NODE, SYMBOL_NODE]
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
        let only_sum_or_with_initial_value =
            config.get_bool("OnlySumOrWithInitialValue", false);
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        if method_name != b"inject" && method_name != b"reduce" {
            return;
        }

        // Must have a receiver
        if call.receiver().is_none() {
            return;
        }

        // Must not have a block
        if call.block().is_some() {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_nodes: Vec<_> = args.arguments().iter().collect();

        let is_sum_pattern = match arg_nodes.len() {
            1 => {
                // inject(:+) or reduce(:+)
                // OnlySumOrWithInitialValue: when true, skip this pattern (no initial value)
                if only_sum_or_with_initial_value {
                    false
                } else {
                    is_plus_symbol(&arg_nodes[0])
                }
            }
            2 => {
                // inject(0, :+) or reduce(0, :+)
                is_zero_literal(&arg_nodes[0]) && is_plus_symbol(&arg_nodes[1])
            }
            _ => false,
        };

        if !is_sum_pattern {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        let method_str = std::str::from_utf8(method_name).unwrap_or("inject");
        let args_str = if arg_nodes.len() == 2 {
            format!("{method_str}(0, :+)")
        } else {
            format!("{method_str}(:+)")
        };

        diagnostics.push(self.diagnostic(source, line, column, format!("Use `sum` instead of `{args_str}`.")));
    }
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
        assert!(diags.is_empty(), "OnlySumOrWithInitialValue should skip inject(:+)");

        // inject(0, :+) with initial value — SHOULD be flagged
        let src2 = b"[1, 2, 3].inject(0, :+)\n";
        let diags2 = run_cop_full_with_config(&Sum, src2, config);
        assert_eq!(diags2.len(), 1, "OnlySumOrWithInitialValue should still flag inject(0, :+)");
    }
}
