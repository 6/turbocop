use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantMerge;

impl Cop for RedundantMerge {
    fn name(&self) -> &'static str {
        "Performance/RedundantMerge"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let max_kv_pairs = config.get_usize("MaxKeyValuePairs", 2);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"merge!" {
            return Vec::new();
        }

        // Must have a receiver (hash.merge!)
        if call.receiver().is_none() {
            return Vec::new();
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args = arguments.arguments();

        // Count key-value pairs in the merge! argument
        let kv_count = if args.len() == 1 {
            let first = args.iter().next().unwrap();
            if let Some(kw_hash) = first.as_keyword_hash_node() {
                kw_hash.elements().len()
            } else if let Some(hash) = first.as_hash_node() {
                hash.elements().len()
            } else {
                0
            }
        } else {
            0
        };

        if kv_count == 0 || kv_count > max_kv_pairs {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let msg = if kv_count == 1 {
            "Use `[]=` instead of `merge!` with a single key-value pair.".to_string()
        } else {
            format!("Use `[]=` instead of `merge!` with {kv_count} key-value pairs.")
        };
        vec![self.diagnostic(source, line, column, msg)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantMerge, "cops/performance/redundant_merge");

    #[test]
    fn config_max_kv_pairs_flags_two() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        // Default MaxKeyValuePairs:2 should flag merge! with 2 KV pairs
        let config = CopConfig {
            options: HashMap::from([
                ("MaxKeyValuePairs".into(), serde_yml::Value::Number(2.into())),
            ]),
            ..CopConfig::default()
        };
        let source = b"hash.merge!(a: 1, b: 2)\n";
        let diags = run_cop_full_with_config(&RedundantMerge, source, config);
        assert!(!diags.is_empty(), "Should flag merge! with 2 pairs when MaxKeyValuePairs:2");
    }

    #[test]
    fn config_max_kv_pairs_allows_three() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        // MaxKeyValuePairs:2 should NOT flag merge! with 3 KV pairs
        let config = CopConfig {
            options: HashMap::from([
                ("MaxKeyValuePairs".into(), serde_yml::Value::Number(2.into())),
            ]),
            ..CopConfig::default()
        };
        let source = b"hash.merge!(a: 1, b: 2, c: 3)\n";
        let diags = run_cop_full_with_config(&RedundantMerge, source, config);
        assert!(diags.is_empty(), "Should not flag merge! with 3 pairs when MaxKeyValuePairs:2");
    }

    #[test]
    fn config_max_kv_pairs_higher() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        // MaxKeyValuePairs:5 should flag merge! with up to 5 KV pairs
        let config = CopConfig {
            options: HashMap::from([
                ("MaxKeyValuePairs".into(), serde_yml::Value::Number(5.into())),
            ]),
            ..CopConfig::default()
        };
        let source = b"hash.merge!(a: 1, b: 2, c: 3)\n";
        let diags = run_cop_full_with_config(&RedundantMerge, source, config);
        assert!(!diags.is_empty(), "Should flag merge! with 3 pairs when MaxKeyValuePairs:5");
    }
}
