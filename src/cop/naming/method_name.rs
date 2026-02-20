use crate::cop::util::is_snake_case;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::DEF_NODE;

pub struct MethodName;

/// Returns true if the name consists entirely of non-alphabetic characters (operator methods).
fn is_operator_method(name: &[u8]) -> bool {
    !name.iter().any(|b| b.is_ascii_alphabetic())
}

impl Cop for MethodName {
    fn name(&self) -> &'static str {
        "Naming/MethodName"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
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
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        let enforced_style = config.get_str("EnforcedStyle", "snake_case");
        let allowed_patterns = config.get_string_array("AllowedPatterns");
        let forbidden_identifiers = config.get_string_array("ForbiddenIdentifiers");
        let forbidden_patterns = config.get_string_array("ForbiddenPatterns");

        let method_name = def_node.name().as_slice();
        let method_name_str = std::str::from_utf8(method_name).unwrap_or("");

        // Skip operator methods (e.g., +, -, [], <=>, ==)
        if is_operator_method(method_name) {
            return;
        }

        let loc = def_node.name_loc();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        // ForbiddenIdentifiers: flag if method name is in the forbidden list
        if let Some(forbidden) = &forbidden_identifiers {
            if forbidden.iter().any(|f| f == method_name_str) {
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("`{method_name_str}` is forbidden, use another method name instead."),
                ));
            }
        }

        // ForbiddenPatterns: flag if method name matches any forbidden regex
        if let Some(patterns) = &forbidden_patterns {
            for pattern in patterns {
                if let Ok(re) = regex::Regex::new(pattern) {
                    if re.is_match(method_name_str) {
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            format!("`{method_name_str}` is forbidden, use another method name instead."),
                        ));
                    }
                }
            }
        }

        // AllowedPatterns: skip if method name matches any pattern
        if let Some(patterns) = &allowed_patterns {
            if patterns.iter().any(|p| method_name_str.contains(p.as_str())) {
                return;
            }
        }

        // Check naming style
        let style_ok = match enforced_style {
            "camelCase" => is_lower_camel_case(method_name),
            _ => is_snake_case(method_name), // snake_case is default
        };

        if style_ok {
            return;
        }

        let style_msg = match enforced_style {
            "camelCase" => "camelCase",
            _ => "snake_case",
        };

        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use {style_msg} for method names."),
        ));
    }
}

/// Returns true if the name is lowerCamelCase (starts lowercase, no underscores, has uppercase).
fn is_lower_camel_case(name: &[u8]) -> bool {
    if name.is_empty() {
        return true;
    }
    // Must start with lowercase or underscore
    if name[0].is_ascii_uppercase() {
        return false;
    }
    // No underscores allowed (except leading)
    let name_without_leading = name.iter().skip_while(|&&b| b == b'_').copied().collect::<Vec<_>>();
    for &b in &name_without_leading {
        if b == b'_' {
            return false;
        }
        if !(b.is_ascii_alphanumeric() || b == b'?' || b == b'!' || b == b'=') {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MethodName, "cops/naming/method_name");

    #[test]
    fn config_enforced_style_camel_case() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("camelCase".into())),
            ]),
            ..CopConfig::default()
        };
        // camelCase method should pass
        let source = b"def myMethod\nend\n";
        let diags = run_cop_full_with_config(&MethodName, source, config);
        assert!(diags.is_empty(), "camelCase method should not be flagged in camelCase mode");
    }

    #[test]
    fn config_enforced_style_camel_case_flags_snake() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("camelCase".into())),
            ]),
            ..CopConfig::default()
        };
        let source = b"def my_method\nend\n";
        let diags = run_cop_full_with_config(&MethodName, source, config);
        assert!(!diags.is_empty(), "snake_case method should be flagged in camelCase mode");
    }

    #[test]
    fn config_forbidden_identifiers() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("ForbiddenIdentifiers".into(), serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("destroy".into()),
                ])),
            ]),
            ..CopConfig::default()
        };
        let source = b"def destroy\nend\n";
        let diags = run_cop_full_with_config(&MethodName, source, config);
        assert!(!diags.is_empty(), "Forbidden identifier should be flagged");
        assert!(diags[0].message.contains("forbidden"));
    }

    #[test]
    fn config_forbidden_patterns() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("ForbiddenPatterns".into(), serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("_v1\\z".into()),
                ])),
            ]),
            ..CopConfig::default()
        };
        let source = b"def release_v1\nend\n";
        let diags = run_cop_full_with_config(&MethodName, source, config);
        assert!(!diags.is_empty(), "Forbidden pattern should be flagged");
        assert!(diags[0].message.contains("forbidden"));
    }
}
