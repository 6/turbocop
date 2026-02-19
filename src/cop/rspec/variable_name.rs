use crate::cop::util::{is_camel_case, is_snake_case, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, KEYWORD_HASH_NODE, STRING_NODE, SYMBOL_NODE};

pub struct VariableName;

impl Cop for VariableName {
    fn name(&self) -> &'static str {
        "RSpec/VariableName"
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
    ) -> Vec<Diagnostic> {
        // Config: EnforcedStyle — "snake_case" (default) or "camelCase"
        let enforced_style = config.get_str("EnforcedStyle", "snake_case");
        // Config: AllowedPatterns — regex patterns to exclude
        let allowed_patterns = config.get_string_array("AllowedPatterns");
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.receiver().is_some() {
            return Vec::new();
        }

        let method_name = call.name().as_slice();
        if method_name != b"let"
            && method_name != b"let!"
            && method_name != b"subject"
            && method_name != b"subject!"
        {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let check_style = |name_bytes: &[u8]| -> bool {
            if enforced_style == "camelCase" {
                is_camel_case(name_bytes)
            } else {
                is_snake_case(name_bytes)
            }
        };

        let style_name = if enforced_style == "camelCase" { "camelCase" } else { "snake_case" };

        for arg in args.arguments().iter() {
            if arg.as_keyword_hash_node().is_some() {
                continue;
            }
            let name_owned: Option<Vec<u8>> = if let Some(sym) = arg.as_symbol_node() {
                Some(sym.unescaped().to_vec())
            } else if let Some(s) = arg.as_string_node() {
                Some(s.unescaped().to_vec())
            } else {
                None
            };

            if let Some(ref name) = name_owned {
                let name_str = std::str::from_utf8(name).unwrap_or("");

                // Check AllowedPatterns
                if let Some(ref patterns) = allowed_patterns {
                    let mut skip = false;
                    for pat in patterns {
                        if let Ok(re) = regex::Regex::new(pat) {
                            if re.is_match(name_str) {
                                skip = true;
                                break;
                            }
                        }
                    }
                    if skip {
                        break;
                    }
                }

                if !check_style(name) {
                    let loc = arg.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Use {style_name} for variable names."),
                    )];
                }
            }
            break;
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(VariableName, "cops/rspec/variable_name");

    #[test]
    fn camel_case_style_flags_snake_case() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("camelCase".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"let(:my_var) { 'x' }\n";
        let diags = crate::testutil::run_cop_full_with_config(&VariableName, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("camelCase"));
    }

    #[test]
    fn allowed_patterns_skips_matching() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowedPatterns".into(),
                serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("^myVar".into()),
                ]),
            )]),
            ..CopConfig::default()
        };
        let source = b"let(:myVar) { 'x' }\n";
        let diags = crate::testutil::run_cop_full_with_config(&VariableName, source, config);
        assert!(diags.is_empty(), "AllowedPatterns should skip matching variable names");
    }
}
