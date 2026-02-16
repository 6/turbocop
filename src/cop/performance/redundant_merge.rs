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
            // Don't flag if argument contains a splat (**hash)
            if first.as_keyword_hash_node().is_some() {
                let kw = first.as_keyword_hash_node().unwrap();
                if kw.elements().iter().any(|e| e.as_assoc_splat_node().is_some()) {
                    return Vec::new();
                }
                kw.elements().len()
            } else if first.as_hash_node().is_some() {
                let hash = first.as_hash_node().unwrap();
                if hash.elements().iter().any(|e| e.as_assoc_splat_node().is_some()) {
                    return Vec::new();
                }
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

        // RuboCop: when pairs > 1, only flag if receiver is "pure" (a simple
        // local variable). Method calls, indexing etc. could have side effects.
        let receiver = call.receiver().unwrap();
        if kv_count > 1 {
            let is_pure = receiver.as_local_variable_read_node().is_some();
            if !is_pure {
                return Vec::new();
            }
        }

        // Don't flag if the return value of merge! appears to be used.
        // merge! returns the hash, while []= returns the assigned value —
        // they're not interchangeable when the result is used.
        let call_end = call.location().end_offset();
        let bytes = source.as_bytes();
        let mut pos = call_end;
        while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
            pos += 1;
        }
        if pos < bytes.len() {
            let next = bytes[pos];
            // Result is chained, used as sub-expression, or otherwise consumed
            if next == b'.' || next == b')' || next == b']' || next == b'&' {
                return Vec::new();
            }
        }
        // Check if merge! is the last expression in a block (its return value
        // becomes the block's return value). Look for `end`/`}` on the next
        // non-blank/non-comment line after the merge! END line. This also handles
        // modifier conditionals like `h.merge!(k: v) if cond`.
        // Use the call's END offset to handle multi-line merge! calls.
        let end_off = call.location().end_offset().saturating_sub(1).max(call.location().start_offset());
        let (call_line, _) = source.offset_to_line_col(end_off);
        let all_lines: Vec<&[u8]> = source.lines().collect();
        for next_line_idx in call_line..all_lines.len() {
            if let Some(nl) = all_lines.get(next_line_idx) {
                let nt = nl.iter()
                    .position(|&b| b != b' ' && b != b'\t')
                    .map(|start| &nl[start..])
                    .unwrap_or(&[]);
                if nt.is_empty() || nt.starts_with(b"#") {
                    continue;
                }
                if nt.starts_with(b"end") || nt.starts_with(b"}") {
                    return Vec::new();
                }
                break;
            }
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

        // Default MaxKeyValuePairs:2 should flag merge! with 2 KV pairs on a local var
        let config = CopConfig {
            options: HashMap::from([
                ("MaxKeyValuePairs".into(), serde_yml::Value::Number(2.into())),
            ]),
            ..CopConfig::default()
        };
        let source = b"h = {}\nh.merge!(a: 1, b: 2)\n";
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
        let source = b"h = {}\nh.merge!(a: 1, b: 2, c: 3)\n";
        let diags = run_cop_full_with_config(&RedundantMerge, source, config);
        assert!(diags.is_empty(), "Should not flag merge! with 3 pairs when MaxKeyValuePairs:2");
    }

    #[test]
    fn config_max_kv_pairs_higher() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        // MaxKeyValuePairs:5 should flag merge! with up to 5 KV pairs on a local var
        let config = CopConfig {
            options: HashMap::from([
                ("MaxKeyValuePairs".into(), serde_yml::Value::Number(5.into())),
            ]),
            ..CopConfig::default()
        };
        let source = b"h = {}\nh.merge!(a: 1, b: 2, c: 3)\n";
        let diags = run_cop_full_with_config(&RedundantMerge, source, config);
        assert!(!diags.is_empty(), "Should flag merge! with 3 pairs when MaxKeyValuePairs:5");
    }

    #[test]
    fn non_pure_receiver_multi_pair_not_flagged() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("MaxKeyValuePairs".into(), serde_yml::Value::Number(2.into())),
            ]),
            ..CopConfig::default()
        };
        // Method call receiver — not a local variable, should not be flagged with 2 pairs
        let source = b"obj.options.merge!(a: 1, b: 2)\n";
        let diags = run_cop_full_with_config(&RedundantMerge, source, config);
        assert!(diags.is_empty(), "Should not flag non-pure receiver with multiple pairs");
    }
}
