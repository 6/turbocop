use crate::cop::rspec_rails::RSPEC_RAILS_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct NegationBeValid;

impl Cop for NegationBeValid {
    fn name(&self) -> &'static str {
        "RSpecRails/NegationBeValid"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_RAILS_DEFAULT_INCLUDE
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "not_to");

        // Look for runner calls: to/not_to/to_not
        let runner_call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let runner_name = runner_call.name().as_slice();
        let is_to = runner_name == b"to";
        let is_not_to = runner_name == b"not_to" || runner_name == b"to_not";

        if !is_to && !is_not_to {
            return Vec::new();
        }

        // Verify receiver is expect(...)
        let recv = match runner_call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };
        let expect_call = match recv.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if expect_call.name().as_slice() != b"expect" || expect_call.receiver().is_some() {
            return Vec::new();
        }

        // Get the matcher argument
        let args = match runner_call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return Vec::new();
        }

        let matcher = match arg_list[0].as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let matcher_name = matcher.name().as_slice();

        // Must be be_valid or be_invalid, with no receiver (bare matcher call)
        if matcher_name != b"be_valid" && matcher_name != b"be_invalid" {
            return Vec::new();
        }

        if matcher.receiver().is_some() {
            return Vec::new();
        }

        match enforced_style {
            "not_to" => {
                // Flag: expect(x).to be_invalid -> suggest expect(x).not_to be_valid
                if is_to && matcher_name == b"be_invalid" {
                    let runner_loc =
                        runner_call.message_loc().unwrap_or(runner_call.location());
                    let (line, column) = source.offset_to_line_col(runner_loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `expect(...).not_to be_valid`.".to_string(),
                    )];
                }
            }
            "be_invalid" => {
                // Flag: expect(x).not_to be_valid -> suggest expect(x).to be_invalid
                if is_not_to && matcher_name == b"be_valid" {
                    let runner_loc =
                        runner_call.message_loc().unwrap_or(runner_call.location());
                    let (line, column) = source.offset_to_line_col(runner_loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `expect(...).to be_invalid`.".to_string(),
                    )];
                }
            }
            _ => {}
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NegationBeValid, "cops/rspecrails/negation_be_valid");

    #[test]
    fn be_invalid_style_flags_not_to_be_valid() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("be_invalid".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"expect(foo).not_to be_valid\n";
        let diags = run_cop_full_with_config(&NegationBeValid, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("be_invalid"));
    }

    #[test]
    fn be_invalid_style_allows_to_be_invalid() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("be_invalid".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"expect(foo).to be_invalid\n";
        assert_cop_no_offenses_full_with_config(&NegationBeValid, source, config);
    }
}
