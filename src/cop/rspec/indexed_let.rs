use crate::cop::shared::node_type::{
    BLOCK_NODE, CALL_NODE, STATEMENTS_NODE, STRING_NODE, SYMBOL_NODE,
};
use crate::cop::shared::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_example_group};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use std::collections::HashMap;

/// RSpec/IndexedLet — flags `let` statements using numeric indexes (e.g., `let(:item_1)`)
/// when the group (by base name with all digits stripped) exceeds `Max`.
///
/// **Conformance fixes applied:**
/// - Grouping uses `gsub(/\d+/, '')` (strips ALL digit sequences) to match RuboCop's
///   `let_name_stripped_index`, not just trailing digits. This matters for names like
///   `user_1_item_1` / `user_1_item_2` / `user_2_item_1` which all group to `user__item_`.
/// - Message uses the first `/\d+/` match (first digit sequence in the name), matching
///   RuboCop's `let_name(let_node)[INDEX_REGEX]`.
/// - Names without trailing `/_?\d+$/` are still excluded (matching `SUFFIX_INDEX_REGEX`).
/// - Fixed: `RSpec.shared_examples` and `RSpec.shared_context` (with explicit `RSpec.`
///   receiver) were not recognized as spec groups. The receiver path only matched
///   `RSpec.describe` but RuboCop's `spec_group?` matches all ExampleGroups + SharedGroups.
///   Now uses `is_rspec_example_group()` for the receiver path too (50 FN fix).
pub struct IndexedLet;

impl Cop for IndexedLet {
    fn name(&self) -> &'static str {
        "RSpec/IndexedLet"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            CALL_NODE,
            STATEMENTS_NODE,
            STRING_NODE,
            SYMBOL_NODE,
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
        // Config: Max — maximum allowed group size (default 1)
        let max = config.get_usize("Max", 1);
        // Config: AllowedIdentifiers — identifiers to ignore
        let allowed_ids = config.get_string_array("AllowedIdentifiers");
        // Config: AllowedPatterns — regex patterns to ignore
        let allowed_patterns = config.get_string_array("AllowedPatterns");

        // This cop checks at example group level: group indexed lets by
        // base name and flag groups with more than Max entries.
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        let is_group = if let Some(recv) = call.receiver() {
            crate::cop::shared::util::constant_name(&recv).is_some_and(|n| n == b"RSpec")
                && is_rspec_example_group(method_name)
        } else {
            is_rspec_example_group(method_name)
        };
        if !is_group {
            return;
        }

        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return,
            },
            None => return,
        };

        let body = match block.body() {
            Some(b) => b,
            None => return,
        };
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        // Collect indexed lets at this level (direct children only)
        struct LetInfo {
            /// Grouping key: name with all digits stripped (matches RuboCop's gsub(/\d+/, ''))
            group_key: String,
            /// First digit sequence in the name (for the message)
            first_index: String,
            line: usize,
            column: usize,
        }

        let mut indexed_lets: Vec<LetInfo> = Vec::new();

        for stmt in stmts.body().iter() {
            let c = match stmt.as_call_node() {
                Some(c) => c,
                None => continue,
            };
            if c.receiver().is_some() {
                continue;
            }
            let mn = c.name().as_slice();
            if mn != b"let" && mn != b"let!" {
                continue;
            }
            let args = match c.arguments() {
                Some(a) => a,
                None => continue,
            };
            let first_arg = match args.arguments().iter().next() {
                Some(a) => a,
                None => continue,
            };
            let name_bytes = if let Some(sym) = first_arg.as_symbol_node() {
                sym.unescaped().to_vec()
            } else if let Some(s) = first_arg.as_string_node() {
                s.unescaped().to_vec()
            } else {
                continue;
            };
            let name_str = match std::str::from_utf8(&name_bytes) {
                Ok(s) => s.to_string(),
                Err(_) => continue,
            };

            // Check AllowedIdentifiers
            if let Some(ref ids) = allowed_ids {
                if ids.iter().any(|id| id == &name_str) {
                    continue;
                }
            }
            // Check AllowedPatterns
            if let Some(ref patterns) = allowed_patterns {
                let mut skip = false;
                for pat in patterns {
                    if let Ok(re) = regex::Regex::new(pat) {
                        if re.is_match(&name_str) {
                            skip = true;
                            break;
                        }
                    }
                }
                if skip {
                    continue;
                }
            }

            // Check if name has a trailing numeric suffix (SUFFIX_INDEX_REGEX = /_?\d+$/)
            if has_trailing_index(&name_str) {
                let loc = c.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let group_key = strip_all_digits(&name_str);
                let first_index = first_digit_sequence(&name_str).unwrap_or_default();
                indexed_lets.push(LetInfo {
                    group_key,
                    first_index,
                    line,
                    column,
                });
            }
        }

        // Group by key (all digits stripped) and flag groups with more than Max entries
        let mut groups: HashMap<&str, Vec<&LetInfo>> = HashMap::new();
        for info in &indexed_lets {
            groups.entry(&info.group_key).or_default().push(info);
        }

        for lets in groups.values() {
            if lets.len() > max {
                for let_info in lets {
                    diagnostics.push(self.diagnostic(
                        source,
                        let_info.line,
                        let_info.column,
                        format!(
                            "This `let` statement uses `{}` in its name. Please give it a meaningful name.",
                            let_info.first_index
                        ),
                    ));
                }
            }
        }
    }
}

/// Check if name has a trailing numeric suffix matching `/_?\d+$/`.
/// Returns `true` for `item_1`, `item1`, `user_1_item_2`, etc.
fn has_trailing_index(name: &str) -> bool {
    let bytes = name.as_bytes();
    let mut i = bytes.len();
    while i > 0 && bytes[i - 1].is_ascii_digit() {
        i -= 1;
    }
    // Must have trailing digits, and not be all digits
    i < bytes.len() && i > 0
}

/// Strip ALL digit sequences from name (equivalent to Ruby's `gsub(/\d+/, '')`).
/// Used for grouping: `user_1_item_2` → `user__item_`.
fn strip_all_digits(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    for ch in name.chars() {
        if !ch.is_ascii_digit() {
            result.push(ch);
        }
    }
    result
}

/// Extract the first digit sequence from name (equivalent to Ruby's `name[/\d+/]`).
/// Used for the message: `user_1_item_2` → `1`.
fn first_digit_sequence(name: &str) -> Option<String> {
    let bytes = name.as_bytes();
    let mut start = None;
    for (i, &b) in bytes.iter().enumerate() {
        if b.is_ascii_digit() {
            if start.is_none() {
                start = Some(i);
            }
        } else if start.is_some() {
            return Some(name[start.unwrap()..i].to_string());
        }
    }
    start.map(|s| name[s..].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IndexedLet, "cops/rspec/indexed_let");

    #[test]
    fn max_config_allows_larger_groups() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "Max".into(),
                serde_yml::Value::Number(serde_yml::Number::from(3)),
            )]),
            ..CopConfig::default()
        };
        // 2 indexed lets with same base — group size 2 <= Max(3) → OK
        let source = b"describe 'test' do\n  let(:item_1) { 'x' }\n  let(:item_2) { 'x' }\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&IndexedLet, source, config);
        assert!(diags.is_empty(), "Max=3 should allow groups up to size 3");
    }

    #[test]
    fn allowed_identifiers_skips_matching() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowedIdentifiers".into(),
                serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("item_1".into()),
                    serde_yml::Value::String("item_2".into()),
                ]),
            )]),
            ..CopConfig::default()
        };
        let source = b"describe 'test' do\n  let(:item_1) { 'x' }\n  let(:item_2) { 'x' }\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&IndexedLet, source, config);
        assert!(
            diags.is_empty(),
            "AllowedIdentifiers should skip matching names"
        );
    }

    #[test]
    fn shared_examples_detected() {
        let source =
            b"shared_examples 'test' do\n  let(:item_1) { 'x' }\n  let(:item_2) { 'x' }\nend\n";
        let diags = crate::testutil::run_cop_full(&IndexedLet, source);
        assert_eq!(
            diags.len(),
            2,
            "shared_examples should detect indexed lets, got: {:?}",
            diags
        );
    }

    #[test]
    fn shared_context_detected() {
        let source =
            b"shared_context 'test' do\n  let(:item_1) { 'x' }\n  let(:item_2) { 'x' }\nend\n";
        let diags = crate::testutil::run_cop_full(&IndexedLet, source);
        assert_eq!(
            diags.len(),
            2,
            "shared_context should detect indexed lets, got: {:?}",
            diags
        );
    }

    #[test]
    fn rspec_shared_examples_detected() {
        let source = b"RSpec.shared_examples 'test' do\n  let(:item_1) { 'x' }\n  let(:item_2) { 'x' }\nend\n";
        let diags = crate::testutil::run_cop_full(&IndexedLet, source);
        assert_eq!(
            diags.len(),
            2,
            "RSpec.shared_examples should detect indexed lets, got: {:?}",
            diags
        );
    }

    #[test]
    fn rspec_shared_context_detected() {
        let source = b"RSpec.shared_context 'test' do\n  let(:item_1) { 'x' }\n  let(:item_2) { 'x' }\nend\n";
        let diags = crate::testutil::run_cop_full(&IndexedLet, source);
        assert_eq!(
            diags.len(),
            2,
            "RSpec.shared_context should detect indexed lets, got: {:?}",
            diags
        );
    }

    #[test]
    fn allowed_patterns_skips_matching_regex() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowedPatterns".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("^item".into())]),
            )]),
            ..CopConfig::default()
        };
        let source = b"describe 'test' do\n  let(:item_1) { 'x' }\n  let(:item_2) { 'x' }\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&IndexedLet, source, config);
        assert!(
            diags.is_empty(),
            "AllowedPatterns should skip matching regex"
        );
    }
}
