use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

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
    ) -> Vec<Diagnostic> {
        let enabled_for_flatten_without_params =
            config.get_bool("EnabledForFlattenWithoutParams", true);
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return Vec::new(),
        };

        if chain.outer_method != b"flatten" {
            return Vec::new();
        }

        // EnabledForFlattenWithoutParams: when false, only flag flatten with args (e.g., flatten(1))
        if !enabled_for_flatten_without_params {
            let outer_call = match node.as_call_node() {
                Some(c) => c,
                None => return Vec::new(),
            };
            if outer_call.arguments().is_none() {
                return Vec::new();
            }
        }

        let inner = chain.inner_method;
        let inner_name = if inner == b"map" {
            "map"
        } else if inner == b"collect" {
            "collect"
        } else {
            return Vec::new();
        };

        // The inner call should have a block
        if chain.inner_call.block().is_none() {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, format!("Use `flat_map` instead of `{inner_name}...flatten`."))]
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
        assert!(diags.is_empty(), "Should skip flatten without params when EnabledForFlattenWithoutParams is false");
    }
}
