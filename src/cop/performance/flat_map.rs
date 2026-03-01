use crate::cop::node_type::CALL_NODE;
use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct FlatMap;

impl Cop for FlatMap {
    fn name(&self) -> &'static str {
        "Performance/FlatMap"
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
        let enabled_for_flatten_without_params =
            config.get_bool("EnabledForFlattenWithoutParams", true);
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        if chain.outer_method != b"flatten" && chain.outer_method != b"flatten!" {
            return;
        }

        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Check flatten argument: only flag no-arg flatten (when config allows) or flatten(1).
        // flatten(2), flatten(3), etc. should NOT be flagged.
        let has_args = outer_call.arguments().is_some();
        if has_args {
            // Only flag if the argument is exactly the integer literal 1
            let args = outer_call.arguments().unwrap();
            let arg_list = args.arguments();
            if arg_list.len() != 1 {
                return;
            }
            let arg = arg_list.iter().next().unwrap();
            let is_one = arg.as_integer_node().is_some_and(|n| {
                let src =
                    &source.as_bytes()[n.location().start_offset()..n.location().end_offset()];
                src == b"1"
            });
            if !is_one {
                return;
            }
        } else if !enabled_for_flatten_without_params {
            // No args and config says don't flag bare flatten
            return;
        }

        let inner = chain.inner_method;
        let inner_name = if inner == b"map" {
            "map"
        } else if inner == b"collect" {
            "collect"
        } else {
            return;
        };

        // The inner call should have a block (literal block or block-pass argument)
        if chain.inner_call.block().is_none() {
            return;
        }

        // Skip if the inner call has regular positional arguments (e.g., Parallel.map(items, opts, &block)).
        // RuboCop's pattern only matches Enumerable#map which takes just a block, not methods
        // like Parallel.map that accept additional positional arguments.
        if chain
            .inner_call
            .arguments()
            .is_some_and(|args| !args.arguments().is_empty())
        {
            return;
        }

        let flatten_name = std::str::from_utf8(chain.outer_method).unwrap_or("flatten");

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `flat_map` instead of `{inner_name}...{flatten_name}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FlatMap, "cops/performance/flat_map");

    #[test]
    fn disabled_for_flatten_without_params_skips_bare_flatten() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnabledForFlattenWithoutParams".into(),
                serde_yml::Value::Bool(false),
            )]),
            ..CopConfig::default()
        };
        // map { }.flatten without args — should NOT be flagged
        let src = b"[1, 2].map { |x| [x, x] }.flatten\n";
        let diags = run_cop_full_with_config(&FlatMap, src, config);
        assert!(
            diags.is_empty(),
            "Should skip flatten without params when EnabledForFlattenWithoutParams is false"
        );
    }
}
