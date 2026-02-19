use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct ShortI18n;

impl Cop for ShortI18n {
    fn name(&self) -> &'static str {
        "Rails/ShortI18n"
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
        let style = config.get_str("EnforcedStyle", "conservative");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        let message = if method_name == b"translate" {
            "Use `I18n.t` instead of `I18n.translate`."
        } else if method_name == b"localize" {
            "Use `I18n.l` instead of `I18n.localize`."
        } else {
            return Vec::new();
        };

        match call.receiver() {
            Some(recv) => {
                // Receiver must be I18n
                // Handle both ConstantReadNode (I18n) and ConstantPathNode (::I18n)
                if util::constant_name(&recv) != Some(b"I18n") {
                    return Vec::new();
                }
            }
            None => {
                // Bare translate/localize without receiver:
                // only flag in aggressive mode
                if style != "aggressive" {
                    return Vec::new();
                }
            }
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, message.to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ShortI18n, "cops/rails/short_i18n");

    #[test]
    fn conservative_style_skips_bare_translate() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;

        let config = CopConfig::default();
        let source = b"translate :key\nlocalize Time.now\n";
        assert_cop_no_offenses_full_with_config(&ShortI18n, source, config);
    }

    #[test]
    fn aggressive_style_flags_bare_translate() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("aggressive".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"translate :key\n";
        let diags = run_cop_full_with_config(&ShortI18n, source, config);
        assert!(!diags.is_empty(), "aggressive style should flag bare translate");
    }

    #[test]
    fn aggressive_style_flags_bare_localize() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("aggressive".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"localize Time.now\n";
        let diags = run_cop_full_with_config(&ShortI18n, source, config);
        assert!(!diags.is_empty(), "aggressive style should flag bare localize");
    }
}
