use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct ImplicitExpect;

impl Cop for ImplicitExpect {
    fn name(&self) -> &'static str {
        "RSpec/ImplicitExpect"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
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
        // Config: EnforcedStyle — "is_expected" (default) or "should"
        let enforced_style = config.get_str("EnforcedStyle", "is_expected");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.receiver().is_some() {
            return;
        }

        let method_name = call.name().as_slice();

        if enforced_style == "should" {
            // "should" style: flag `is_expected`
            if method_name == b"is_expected" {
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Prefer `should` over `is_expected.to`.".to_string(),
                ));
            }
        } else {
            // Default "is_expected" style: flag `should` and `should_not`
            if method_name == b"should" {
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Prefer `is_expected.to` over `should`.".to_string(),
                ));
            }

            if method_name == b"should_not" {
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Prefer `is_expected.to_not` over `should_not`.".to_string(),
                ));
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ImplicitExpect, "cops/rspec/implicit_expect");

    #[test]
    fn should_style_flags_is_expected() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("should".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"is_expected.to eq(1)\n";
        let diags = crate::testutil::run_cop_full_with_config(&ImplicitExpect, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("should"));
    }

    #[test]
    fn should_style_does_not_flag_should() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("should".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"should eq(1)\n";
        let diags = crate::testutil::run_cop_full_with_config(&ImplicitExpect, source, config);
        assert!(diags.is_empty());
    }
}
