use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RequestReferer;

impl Cop for RequestReferer {
    fn name(&self) -> &'static str {
        "Rails/RequestReferer"
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
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "referer");

        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        if chain.inner_method != b"request" {
            return;
        }

        // Determine the "wrong" method based on style
        let (wrong_method, preferred) = match style {
            "referrer" => (b"referer" as &[u8], "referrer"),
            _ => (b"referrer" as &[u8], "referer"),
        };

        if chain.outer_method != wrong_method {
            return;
        }

        let wrong_str = std::str::from_utf8(wrong_method).unwrap_or("?");
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `request.{preferred}` instead of `request.{wrong_str}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RequestReferer, "cops/rails/request_referer");

    #[test]
    fn referrer_style_flags_referer() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("referrer".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"request.referer\n";
        let diags = run_cop_full_with_config(&RequestReferer, source, config);
        assert!(
            !diags.is_empty(),
            "referrer style should flag request.referer"
        );
        assert!(diags[0].message.contains("referrer"));
    }

    #[test]
    fn referrer_style_allows_referrer() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("referrer".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"request.referrer\n";
        assert_cop_no_offenses_full_with_config(&RequestReferer, source, config);
    }
}
