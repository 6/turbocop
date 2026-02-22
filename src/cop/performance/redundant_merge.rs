use crate::cop::node_type::{
    ASSOC_SPLAT_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE, LOCAL_VARIABLE_READ_NODE,
};
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

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ASSOC_SPLAT_NODE,
            CALL_NODE,
            HASH_NODE,
            KEYWORD_HASH_NODE,
            LOCAL_VARIABLE_READ_NODE,
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
        let max_kv_pairs = config.get_usize("MaxKeyValuePairs", 2);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"merge!" {
            return;
        }

        // Must have a receiver (hash.merge!)
        if call.receiver().is_none() {
            return;
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args = arguments.arguments();

        // Count key-value pairs in the merge! argument
        let kv_count = if args.len() == 1 {
            let first = args.iter().next().unwrap();
            // Don't flag if argument contains a splat (**hash)
            if first.as_keyword_hash_node().is_some() {
                let kw = first.as_keyword_hash_node().unwrap();
                if kw
                    .elements()
                    .iter()
                    .any(|e| e.as_assoc_splat_node().is_some())
                {
                    return;
                }
                kw.elements().len()
            } else if first.as_hash_node().is_some() {
                let hash = first.as_hash_node().unwrap();
                if hash
                    .elements()
                    .iter()
                    .any(|e| e.as_assoc_splat_node().is_some())
                {
                    return;
                }
                hash.elements().len()
            } else {
                0
            }
        } else {
            0
        };

        if kv_count == 0 || kv_count > max_kv_pairs {
            return;
        }

        // RuboCop: when pairs > 1, only flag if receiver is "pure" (a simple
        // local variable). Method calls, indexing etc. could have side effects.
        let receiver = call.receiver().unwrap();
        if kv_count > 1 {
            let is_pure = receiver.as_local_variable_read_node().is_some();
            if !is_pure {
                return;
            }
        }

        // Don't flag if the return value of merge! appears to be used.
        // merge! returns the hash, while []= returns the assigned value —
        // they're not interchangeable when the result is used.
        //
        // Check if the merge! call is the RHS of an assignment.
        // The linter visits each node; we check if the call's source starts
        // after an `=` on the same line (indicating it's an assignment RHS).
        let call_start = call.location().start_offset();
        let call_line_start = {
            let mut pos = call_start;
            while pos > 0 && source.as_bytes()[pos - 1] != b'\n' {
                pos -= 1;
            }
            pos
        };
        let before_call = &source.as_bytes()[call_line_start..call_start];
        // Check for assignment operator before the merge! call.
        // Match `x = merge!` but not `==`, `!=`, `>=`, `<=`.
        for i in 0..before_call.len() {
            if before_call[i] == b'=' {
                // Make sure it's not ==, !=, >=, <=
                let prev = if i > 0 { before_call[i - 1] } else { 0 };
                let next = if i + 1 < before_call.len() {
                    before_call[i + 1]
                } else {
                    0
                };
                if prev != b'=' && prev != b'!' && prev != b'>' && prev != b'<' && next != b'=' {
                    return;
                }
            }
        }

        let call_end = call.location().end_offset();
        let bytes = source.as_bytes();
        let mut pos = call_end;
        while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
            pos += 1;
        }
        if pos < bytes.len() {
            let next = bytes[pos];
            // Result is chained, used as sub-expression, or otherwise consumed.
            // Also skip `}` — merge! is the last expression in a single-line block,
            // its return value becomes the block's return value.
            if next == b'.' || next == b')' || next == b']' || next == b'&' || next == b'}' {
                return;
            }
        }
        // Check if merge! is the last expression in a block (its return value
        // becomes the block's return value). Look for `end`/`}` on the next
        // non-blank/non-comment line after the merge! END line. This also handles
        // modifier conditionals like `h.merge!(k: v) if cond`.
        // Use the call's END offset to handle multi-line merge! calls.
        let end_off = call
            .location()
            .end_offset()
            .saturating_sub(1)
            .max(call.location().start_offset());
        let (call_line, _) = source.offset_to_line_col(end_off);
        let all_lines: Vec<&[u8]> = source.lines().collect();
        for next_line_idx in call_line..all_lines.len() {
            if let Some(nl) = all_lines.get(next_line_idx) {
                let nt = nl
                    .iter()
                    .position(|&b| b != b' ' && b != b'\t')
                    .map(|start| &nl[start..])
                    .unwrap_or(&[]);
                if nt.is_empty() || nt.starts_with(b"#") {
                    continue;
                }
                if nt.starts_with(b"end")
                    || nt.starts_with(b"}")
                    || nt.starts_with(b"rescue")
                    || nt.starts_with(b"ensure")
                    || nt.starts_with(b"else")
                    || nt.starts_with(b"elsif")
                    || nt.starts_with(b"when")
                {
                    return;
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
        diagnostics.push(self.diagnostic(source, line, column, msg));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantMerge, "cops/performance/redundant_merge");

    #[test]
    fn config_max_kv_pairs_flags_two() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // Default MaxKeyValuePairs:2 should flag merge! with 2 KV pairs on a local var
        let config = CopConfig {
            options: HashMap::from([(
                "MaxKeyValuePairs".into(),
                serde_yml::Value::Number(2.into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"h = {}\nh.merge!(a: 1, b: 2)\n";
        let diags = run_cop_full_with_config(&RedundantMerge, source, config);
        assert!(
            !diags.is_empty(),
            "Should flag merge! with 2 pairs when MaxKeyValuePairs:2"
        );
    }

    #[test]
    fn config_max_kv_pairs_allows_three() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // MaxKeyValuePairs:2 should NOT flag merge! with 3 KV pairs
        let config = CopConfig {
            options: HashMap::from([(
                "MaxKeyValuePairs".into(),
                serde_yml::Value::Number(2.into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"h = {}\nh.merge!(a: 1, b: 2, c: 3)\n";
        let diags = run_cop_full_with_config(&RedundantMerge, source, config);
        assert!(
            diags.is_empty(),
            "Should not flag merge! with 3 pairs when MaxKeyValuePairs:2"
        );
    }

    #[test]
    fn config_max_kv_pairs_higher() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // MaxKeyValuePairs:5 should flag merge! with up to 5 KV pairs on a local var
        let config = CopConfig {
            options: HashMap::from([(
                "MaxKeyValuePairs".into(),
                serde_yml::Value::Number(5.into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"h = {}\nh.merge!(a: 1, b: 2, c: 3)\n";
        let diags = run_cop_full_with_config(&RedundantMerge, source, config);
        assert!(
            !diags.is_empty(),
            "Should flag merge! with 3 pairs when MaxKeyValuePairs:5"
        );
    }

    #[test]
    fn non_pure_receiver_multi_pair_not_flagged() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "MaxKeyValuePairs".into(),
                serde_yml::Value::Number(2.into()),
            )]),
            ..CopConfig::default()
        };
        // Method call receiver — not a local variable, should not be flagged with 2 pairs
        let source = b"obj.options.merge!(a: 1, b: 2)\n";
        let diags = run_cop_full_with_config(&RedundantMerge, source, config);
        assert!(
            diags.is_empty(),
            "Should not flag non-pure receiver with multiple pairs"
        );
    }
}
