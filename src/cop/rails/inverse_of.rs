use crate::cop::util::{class_body_calls, has_keyword_arg, is_dsl_call};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct InverseOf;

impl Cop for InverseOf {
    fn name(&self) -> &'static str {
        "Rails/InverseOf"
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
        let ignore_scopes = config.get_bool("IgnoreScopes", false);

        let class = match node.as_class_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let mut diagnostics = Vec::new();
        let calls = class_body_calls(&class);

        for call in &calls {
            let is_assoc = is_dsl_call(call, b"has_many")
                || is_dsl_call(call, b"has_one")
                || is_dsl_call(call, b"belongs_to");

            if !is_assoc {
                continue;
            }

            // Check if the call has a scope (lambda argument)
            let has_scope = call.arguments().is_some_and(|args| {
                args.arguments().iter().any(|a| a.as_lambda_node().is_some())
            });

            // Only flag when :foreign_key or :as is specified without :inverse_of,
            // OR when a scope is present (and IgnoreScopes is false)
            let has_foreign_key = has_keyword_arg(call, b"foreign_key");
            let has_as = has_keyword_arg(call, b"as");
            let needs_inverse = has_foreign_key || has_as || (has_scope && !ignore_scopes);

            if needs_inverse && !has_keyword_arg(call, b"inverse_of") {
                let loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Specify an `:inverse_of` option.".to_string(),
                ));
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InverseOf, "cops/rails/inverse_of");

    #[test]
    fn ignore_scopes_true_allows_scope_without_inverse_of() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "IgnoreScopes".to_string(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };
        let source = b"class Blog < ApplicationRecord\n  has_many :posts, -> { order(:name) }\nend\n";
        assert_cop_no_offenses_full_with_config(&InverseOf, source, config);
    }

    #[test]
    fn ignore_scopes_false_flags_scope_without_inverse_of() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig::default();
        let source = b"class Blog < ApplicationRecord\n  has_many :posts, -> { order(:name) }\nend\n";
        let diags = run_cop_full_with_config(&InverseOf, source, config);
        assert!(!diags.is_empty(), "IgnoreScopes:false should flag scope without inverse_of");
    }
}
