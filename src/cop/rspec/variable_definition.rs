use crate::cop::node_type::{CALL_NODE, KEYWORD_HASH_NODE, STRING_NODE, SYMBOL_NODE};
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct VariableDefinition;

impl Cop for VariableDefinition {
    fn name(&self) -> &'static str {
        "RSpec/VariableDefinition"
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
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Config: EnforcedStyle — "symbols" (default) or "strings"
        let enforced_style = config.get_str("EnforcedStyle", "symbols");
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.receiver().is_some() {
            return;
        }

        let method_name = call.name().as_slice();
        if method_name != b"let"
            && method_name != b"let!"
            && method_name != b"subject"
            && method_name != b"subject!"
        {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        for arg in args.arguments().iter() {
            if arg.as_keyword_hash_node().is_some() {
                continue;
            }
            if enforced_style == "strings" {
                // "strings" style: flag symbol names, prefer strings
                if arg.as_symbol_node().is_some() {
                    let loc = arg.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Use strings for variable names.".to_string(),
                    ));
                }
            } else {
                // Default "symbols" style: flag string names, prefer symbols
                if arg.as_string_node().is_some() {
                    let loc = arg.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Use symbols for variable names.".to_string(),
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
    crate::cop_fixture_tests!(VariableDefinition, "cops/rspec/variable_definition");

    #[test]
    fn strings_style_flags_symbol_names() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("strings".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"let(:foo) { 'bar' }\n";
        let diags = crate::testutil::run_cop_full_with_config(&VariableDefinition, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("strings"));
    }

    #[test]
    fn strings_style_does_not_flag_string_names() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("strings".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"let('foo') { 'bar' }\n";
        let diags = crate::testutil::run_cop_full_with_config(&VariableDefinition, source, config);
        assert!(diags.is_empty());
    }
}
