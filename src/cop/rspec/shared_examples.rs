use crate::cop::util::{self, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, KEYWORD_HASH_NODE, STRING_NODE, SYMBOL_NODE};

pub struct SharedExamples;

/// Methods that accept shared example titles.
const SHARED_EXAMPLE_METHODS: &[&[u8]] = &[
    b"it_behaves_like",
    b"it_should_behave_like",
    b"shared_examples",
    b"shared_examples_for",
    b"include_examples",
];

impl Cop for SharedExamples {
    fn name(&self) -> &'static str {
        "RSpec/SharedExamples"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, KEYWORD_HASH_NODE, STRING_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Config: EnforcedStyle — "string" (default) or "symbol"
        let enforced_style = config.get_str("EnforcedStyle", "string");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Check for RSpec.shared_examples / ::RSpec.shared_examples as well
        let is_shared = if let Some(recv) = call.receiver() {
            util::constant_name(&recv).map_or(false, |n| n == b"RSpec")
                && (method_name == b"shared_examples"
                    || method_name == b"shared_examples_for")
        } else {
            SHARED_EXAMPLE_METHODS
                .iter()
                .any(|m| method_name == *m)
        };

        if !is_shared {
            return;
        }

        // Get the first argument — should be a string (default enforced style)
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        for arg in args.arguments().iter() {
            if arg.as_keyword_hash_node().is_some() {
                continue;
            }
            if enforced_style == "symbol" {
                // "symbol" style: flag string arguments, prefer symbols
                if let Some(s) = arg.as_string_node() {
                    let str_val = std::str::from_utf8(s.unescaped()).unwrap_or("");
                    let sym_name = str_val.replace(' ', "_");
                    let loc = s.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Prefer `:{sym_name}` over '{str_val}' to symbolize shared examples."),
                    ));
                }
            } else {
                // Default "string" style: flag symbol arguments, prefer strings
                if let Some(sym) = arg.as_symbol_node() {
                    let sym_name = std::str::from_utf8(sym.unescaped()).unwrap_or("");
                    let title = sym_name.replace('_', " ");
                    let loc = sym.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!(
                            "Prefer '{}' over `:{sym_name}` to titleize shared examples.",
                            title
                        ),
                    ));
                }
            }
            break;
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SharedExamples, "cops/rspec/shared_examples");

    #[test]
    fn symbol_style_flags_string_args() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("symbol".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"it_behaves_like 'some example'\n";
        let diags = crate::testutil::run_cop_full_with_config(&SharedExamples, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains(":some_example"));
    }

    #[test]
    fn symbol_style_does_not_flag_symbol_args() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("symbol".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"it_behaves_like :some_example\n";
        let diags = crate::testutil::run_cop_full_with_config(&SharedExamples, source, config);
        assert!(diags.is_empty());
    }
}
