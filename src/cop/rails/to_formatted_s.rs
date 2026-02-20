use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct ToFormattedS;

impl Cop for ToFormattedS {
    fn name(&self) -> &'static str {
        "Rails/ToFormattedS"
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
        let style = config.get_str("EnforcedStyle", "to_fs");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.receiver().is_none() {
            return;
        }

        let method_name = call.name().as_slice();

        // Flag the "wrong" method based on style
        let (wrong_method, preferred) = match style {
            "to_formatted_s" => (b"to_fs" as &[u8], "to_formatted_s"),
            _ => (b"to_formatted_s" as &[u8], "to_fs"), // "to_fs" (default)
        };

        if method_name != wrong_method {
            return;
        }

        let wrong_str = std::str::from_utf8(wrong_method).unwrap_or("?");
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{preferred}` instead of `{wrong_str}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ToFormattedS, "cops/rails/to_formatted_s");

    #[test]
    fn to_formatted_s_style_flags_to_fs() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("to_formatted_s".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"time.to_fs(:db)\n";
        let diags = run_cop_full_with_config(&ToFormattedS, source, config);
        assert!(!diags.is_empty(), "to_formatted_s style should flag to_fs");
    }

    #[test]
    fn to_formatted_s_style_allows_to_formatted_s() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("to_formatted_s".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"time.to_formatted_s(:db)\n";
        assert_cop_no_offenses_full_with_config(&ToFormattedS, source, config);
    }
}
