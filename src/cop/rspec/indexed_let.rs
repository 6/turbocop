use crate::cop::util::{is_rspec_example_group, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use std::collections::HashMap;

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

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
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
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        let is_group = if let Some(recv) = call.receiver() {
            crate::cop::util::constant_name(&recv).map_or(false, |n| n == b"RSpec")
                && method_name == b"describe"
        } else {
            is_rspec_example_group(method_name)
        };
        if !is_group {
            return Vec::new();
        }

        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        let body = match block.body() {
            Some(b) => b,
            None => return Vec::new(),
        };
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Collect indexed lets at this level (direct children only)
        struct LetInfo {
            #[allow(dead_code)]
            name: String,
            base_name: String,
            index_str: String,
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

            // Check if name has a trailing numeric suffix
            if let Some((base, index_str)) = split_trailing_index(&name_str) {
                let loc = c.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                indexed_lets.push(LetInfo {
                    name: name_str,
                    base_name: base,
                    index_str,
                    line,
                    column,
                });
            }
        }

        // Group by base name and flag groups with more than Max entries
        let mut groups: HashMap<&str, Vec<&LetInfo>> = HashMap::new();
        for info in &indexed_lets {
            groups.entry(&info.base_name).or_default().push(info);
        }

        let mut diagnostics = Vec::new();
        for (_base, lets) in &groups {
            if lets.len() > max {
                for let_info in lets {
                    diagnostics.push(self.diagnostic(
                        source,
                        let_info.line,
                        let_info.column,
                        format!(
                            "This `let` statement uses `{}` in its name. Please give it a meaningful name.",
                            let_info.index_str
                        ),
                    ));
                }
            }
        }

        diagnostics
    }
}

/// Split a name into base name (with digits stripped) and index string.
/// Matches Ruby's `/_?\d+$/` pattern.
/// e.g., "item_1" → Some(("item", "1")), "item1" → Some(("item", "1"))
fn split_trailing_index(name: &str) -> Option<(String, String)> {
    // Find trailing digits
    let name_bytes = name.as_bytes();
    let mut i = name_bytes.len();
    while i > 0 && name_bytes[i - 1].is_ascii_digit() {
        i -= 1;
    }
    if i >= name_bytes.len() || i == 0 {
        return None; // No trailing digits or only digits
    }
    let index_str = &name[i..];
    // Base name is everything before digits, also strip trailing underscore
    let mut base = &name[..i];
    if base.ends_with('_') {
        base = &base[..base.len() - 1];
    }
    Some((base.to_string(), index_str.to_string()))
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
        assert!(diags.is_empty(), "AllowedIdentifiers should skip matching names");
    }

    #[test]
    fn allowed_patterns_skips_matching_regex() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowedPatterns".into(),
                serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("^item".into()),
                ]),
            )]),
            ..CopConfig::default()
        };
        let source = b"describe 'test' do\n  let(:item_1) { 'x' }\n  let(:item_2) { 'x' }\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&IndexedLet, source, config);
        assert!(diags.is_empty(), "AllowedPatterns should skip matching regex");
    }
}
