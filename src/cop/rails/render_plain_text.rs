use crate::cop::util::keyword_arg_value;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, STRING_NODE};

pub struct RenderPlainText;

impl Cop for RenderPlainText {
    fn name(&self) -> &'static str {
        "Rails/RenderPlainText"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE]
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
        let content_type_compat = config.get_bool("ContentTypeCompatibility", true);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if call.name().as_slice() != b"render" {
            return;
        }
        if keyword_arg_value(&call, b"text").is_none() {
            return;
        }

        // ContentTypeCompatibility: when true, only flag if content_type: 'text/plain' is present
        // (because without content_type:, Rails 4 defaults text: to 'text/html',
        //  so changing to plain: would change the content type)
        // When false, always flag render text:
        if content_type_compat {
            let ct_value = keyword_arg_value(&call, b"content_type");
            match ct_value {
                Some(v) => {
                    // Only flag if content_type is 'text/plain'
                    if let Some(s) = v.as_string_node() {
                        if s.unescaped() != b"text/plain" {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                None => {
                    // No content_type specified; with compat mode, don't flag
                    return;
                }
            }
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `render plain:` instead of `render text:`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RenderPlainText, "cops/rails/render_plain_text");

    #[test]
    fn content_type_compat_true_skips_without_content_type() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;

        // Default config (ContentTypeCompatibility: true)
        let config = CopConfig::default();
        let source = b"render text: 'hello'\n";
        assert_cop_no_offenses_full_with_config(&RenderPlainText, source, config);
    }

    #[test]
    fn content_type_compat_false_flags_without_content_type() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "ContentTypeCompatibility".to_string(),
                serde_yml::Value::Bool(false),
            )]),
            ..CopConfig::default()
        };
        let source = b"render text: 'hello'\n";
        let diags = run_cop_full_with_config(&RenderPlainText, source, config);
        assert!(!diags.is_empty(), "ContentTypeCompatibility:false should flag render text: without content_type");
    }
}
