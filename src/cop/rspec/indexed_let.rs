use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

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
        // Config: Max — maximum allowed index (default 1, meaning any index triggers)
        let max = config.get_usize("Max", 1);
        // Config: AllowedIdentifiers — identifiers to ignore
        let allowed_ids = config.get_string_array("AllowedIdentifiers");
        // Config: AllowedPatterns — regex patterns to ignore
        let allowed_patterns = config.get_string_array("AllowedPatterns");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.receiver().is_some() {
            return Vec::new();
        }

        let method_name = call.name().as_slice();
        if method_name != b"let" && method_name != b"let!" {
            return Vec::new();
        }

        // Get the first argument (the name)
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        for arg in args.arguments().iter() {
            if arg.as_keyword_hash_node().is_some() {
                continue;
            }
            let name: Vec<u8> = if let Some(sym) = arg.as_symbol_node() {
                sym.unescaped().to_vec()
            } else if let Some(s) = arg.as_string_node() {
                s.unescaped().to_vec()
            } else {
                break;
            };

            let name_str = std::str::from_utf8(&name).unwrap_or("");

            // Check AllowedIdentifiers
            if let Some(ref ids) = allowed_ids {
                if ids.iter().any(|id| id == name_str) {
                    break;
                }
            }

            // Check AllowedPatterns
            if let Some(ref patterns) = allowed_patterns {
                let mut skip = false;
                for pat in patterns {
                    if let Ok(re) = regex::Regex::new(pat) {
                        if re.is_match(name_str) {
                            skip = true;
                            break;
                        }
                    }
                }
                if skip {
                    break;
                }
            }

            // Check if the name ends with a numeric suffix (e.g., item_1, item1)
            if let Some(index) = find_trailing_index(&name) {
                if index >= max {
                    let loc = call.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        format!(
                            "This `let` statement uses `{}` in its name. Please give it a meaningful name.",
                            index
                        ),
                    )];
                }
            }
            break;
        }

        Vec::new()
    }
}

/// Find trailing numeric suffix in a name like `item_1` or `item1`.
/// Returns the numeric value of the trailing digits.
fn find_trailing_index(name: &[u8]) -> Option<usize> {
    // Walk backwards to find trailing digits
    let mut i = name.len();
    while i > 0 && name[i - 1].is_ascii_digit() {
        i -= 1;
    }
    if i < name.len() && i > 0 {
        // There's at least one trailing digit and something before it
        let digits = &name[i..];
        std::str::from_utf8(digits).ok()?.parse().ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IndexedLet, "cops/rspec/indexed_let");

    #[test]
    fn max_config_allows_higher_indices() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "Max".into(),
                serde_yml::Value::Number(serde_yml::Number::from(5)),
            )]),
            ..CopConfig::default()
        };
        let source = b"let(:item_3) { 'x' }\n";
        let diags = crate::testutil::run_cop_full_with_config(&IndexedLet, source, config);
        assert!(diags.is_empty(), "Max=5 should allow index 3");
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
                ]),
            )]),
            ..CopConfig::default()
        };
        let source = b"let(:item_1) { 'x' }\n";
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
                    serde_yml::Value::String("^item_".into()),
                ]),
            )]),
            ..CopConfig::default()
        };
        let source = b"let(:item_2) { 'x' }\n";
        let diags = crate::testutil::run_cop_full_with_config(&IndexedLet, source, config);
        assert!(diags.is_empty(), "AllowedPatterns should skip matching regex");
    }
}
