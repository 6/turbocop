use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct I18nLazyLookup;

impl Cop for I18nLazyLookup {
    fn name(&self) -> &'static str {
        "Rails/I18nLazyLookup"
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
        let style = config.get_str("EnforcedStyle", "lazy");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method = call.name().as_slice();

        // Match I18n.t("key") or bare t("key")
        // Handle both ConstantReadNode (I18n) and ConstantPathNode (::I18n)
        let is_i18n_t = if method == b"t" {
            if let Some(recv) = call.receiver() {
                util::constant_name(&recv) == Some(b"I18n")
            } else {
                // bare t()
                true
            }
        } else {
            false
        };

        if !is_i18n_t {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        if let Some(s) = arg_list[0].as_string_node() {
            let key = s.unescaped();

            match style {
                "explicit" => {
                    // Flag lazy lookups (keys starting with '.')
                    if !key.starts_with(b".") {
                        return Vec::new();
                    }
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Use explicit lookup for i18n keys.".to_string(),
                    )];
                }
                _ => {
                    // "lazy" (default): flag explicit lookups with 3+ dot-separated segments
                    let dot_count = key.iter().filter(|&&b| b == b'.').count();
                    if dot_count < 2 {
                        return Vec::new();
                    }
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Use lazy lookup for i18n keys.".to_string(),
                    )];
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(I18nLazyLookup, "cops/rails/i18n_lazy_lookup");

    #[test]
    fn explicit_style_flags_lazy_key() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("explicit".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"t('.success')\n";
        let diags = run_cop_full_with_config(&I18nLazyLookup, source, config);
        assert!(!diags.is_empty(), "explicit style should flag lazy lookups");
    }

    #[test]
    fn explicit_style_allows_explicit_key() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("explicit".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"t('books.create.success')\n";
        assert_cop_no_offenses_full_with_config(&I18nLazyLookup, source, config);
    }
}
