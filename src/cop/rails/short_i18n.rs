use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-07)
///
/// FP=13, FN=0. All FPs from `Pagy::I18n.translate(...)` — a qualified constant path.
/// `util::constant_name` returns the last segment, so `Pagy::I18n` matched `I18n`.
/// RuboCop's pattern is `(const {nil? cbase} :I18n)` — only unqualified `I18n` or `::I18n`.
/// Fixed by checking ConstantReadNode/ConstantPathNode directly instead of constant_name.
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
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "conservative");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        let message = if method_name == b"translate" {
            "Use `I18n.t` instead of `I18n.translate`."
        } else if method_name == b"localize" {
            "Use `I18n.l` instead of `I18n.localize`."
        } else {
            return;
        };

        match call.receiver() {
            Some(recv) => {
                // Receiver must be unqualified I18n or root-qualified ::I18n.
                // RuboCop: (const {nil? cbase} :I18n) — NOT Pagy::I18n etc.
                if let Some(cr) = recv.as_constant_read_node() {
                    if cr.name().as_slice() != b"I18n" {
                        return;
                    }
                } else if let Some(cp) = recv.as_constant_path_node() {
                    // ::I18n — parent must be None (cbase) and name must be I18n
                    if cp.parent().is_some() {
                        return;
                    }
                    if cp.name().map(|n| n.as_slice()) != Some(b"I18n") {
                        return;
                    }
                } else {
                    return;
                }
            }
            None => {
                // Bare translate/localize without receiver:
                // only flag in aggressive mode
                if style != "aggressive" {
                    return;
                }
            }
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, message.to_string()));
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
        assert!(
            !diags.is_empty(),
            "aggressive style should flag bare translate"
        );
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
        assert!(
            !diags.is_empty(),
            "aggressive style should flag bare localize"
        );
    }
}
