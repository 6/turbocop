use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct RefuteMethods;

impl Cop for RefuteMethods {
    fn name(&self) -> &'static str {
        "Rails/RefuteMethods"
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
        let style = config.get_str("EnforcedStyle", "assert_not");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.receiver().is_some() {
            return Vec::new();
        }

        let name = call.name().as_slice();
        let name_str = std::str::from_utf8(name).unwrap_or("");

        let (is_bad, message) = match style {
            "refute" => {
                // Flag assert_not_* methods, suggest refute_*
                if name_str.starts_with("assert_not_") {
                    let replacement = name_str.replacen("assert_not_", "refute_", 1);
                    (true, format!("Prefer `{replacement}` over `{name_str}`."))
                } else if name_str == "assert_not" {
                    (true, "Prefer `refute` over `assert_not`.".to_string())
                } else {
                    (false, String::new())
                }
            }
            _ => {
                // "assert_not" (default): flag refute_* methods
                if name_str.starts_with("refute_") {
                    let replacement = name_str.replacen("refute_", "assert_not_", 1);
                    (true, format!("Prefer `{replacement}` over `{name_str}`."))
                } else if name_str == "refute" {
                    (true, "Prefer `assert_not` over `refute`.".to_string())
                } else {
                    (false, String::new())
                }
            }
        };

        if !is_bad {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, message)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RefuteMethods, "cops/rails/refute_methods");

    #[test]
    fn refute_style_flags_assert_not() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("refute".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"assert_not false\n";
        let diags = run_cop_full_with_config(&RefuteMethods, source, config);
        assert!(!diags.is_empty(), "refute style should flag assert_not");
    }

    #[test]
    fn refute_style_allows_refute() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("refute".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"refute false\nrefute_empty []\nrefute_equal 1, 2\n";
        assert_cop_no_offenses_full_with_config(&RefuteMethods, source, config);
    }
}
