use crate::cop::util::is_snake_case;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::LOCAL_VARIABLE_WRITE_NODE;

pub struct VariableName;

impl Cop for VariableName {
    fn name(&self) -> &'static str {
        "Naming/VariableName"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[LOCAL_VARIABLE_WRITE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let write_node = match node.as_local_variable_write_node() {
            Some(n) => n,
            None => return,
        };

        let enforced_style = config.get_str("EnforcedStyle", "snake_case");
        let allowed_identifiers = config.get_string_array("AllowedIdentifiers");
        let allowed_patterns = config.get_string_array("AllowedPatterns");
        let forbidden_identifiers = config.get_string_array("ForbiddenIdentifiers");
        let forbidden_patterns = config.get_string_array("ForbiddenPatterns");

        let var_name = write_node.name().as_slice();
        let var_name_str = std::str::from_utf8(var_name).unwrap_or("");

        // Skip names starting with _ (convention for unused vars)
        if var_name.starts_with(b"_") {
            return;
        }

        let loc = write_node.name_loc();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        // ForbiddenIdentifiers: flag if var name is in the forbidden list
        if let Some(forbidden) = &forbidden_identifiers {
            if forbidden.iter().any(|f| f == var_name_str) {
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("`{var_name_str}` is forbidden, use another variable name instead."),
                ));
            }
        }

        // ForbiddenPatterns: flag if var name matches any forbidden regex
        if let Some(patterns) = &forbidden_patterns {
            for pattern in patterns {
                if let Ok(re) = regex::Regex::new(pattern) {
                    if re.is_match(var_name_str) {
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            format!("`{var_name_str}` is forbidden, use another variable name instead."),
                        ));
                    }
                }
            }
        }

        // AllowedIdentifiers: skip if var name is explicitly allowed
        if let Some(allowed) = &allowed_identifiers {
            if allowed.iter().any(|a| a == var_name_str) {
                return;
            }
        }

        // AllowedPatterns: skip if var name matches any pattern
        if let Some(patterns) = &allowed_patterns {
            if patterns.iter().any(|p| var_name_str.contains(p.as_str())) {
                return;
            }
        }

        // Check naming style
        let style_ok = match enforced_style {
            "camelCase" => is_lower_camel_case(var_name),
            _ => is_snake_case(var_name), // snake_case is default
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
            format!("Use {style_msg} for variable names."),
        ));
    }
}

/// Returns true if the name is lowerCamelCase (starts lowercase, no underscores).
fn is_lower_camel_case(name: &[u8]) -> bool {
    if name.is_empty() {
        return true;
    }
    if name[0].is_ascii_uppercase() {
        return false;
    }
    for &b in name {
        if b == b'_' {
            return false;
        }
        if !(b.is_ascii_alphanumeric()) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(VariableName, "cops/naming/variable_name");

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
        let source = b"myVar = 1\n";
        let diags = run_cop_full_with_config(&VariableName, source, config);
        assert!(diags.is_empty(), "camelCase variable should not be flagged in camelCase mode");
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
        let source = b"my_var = 1\n";
        let diags = run_cop_full_with_config(&VariableName, source, config);
        assert!(!diags.is_empty(), "snake_case variable should be flagged in camelCase mode");
    }

    #[test]
    fn config_forbidden_identifiers() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("ForbiddenIdentifiers".into(), serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("data".into()),
                ])),
            ]),
            ..CopConfig::default()
        };
        let source = b"data = 1\n";
        let diags = run_cop_full_with_config(&VariableName, source, config);
        assert!(!diags.is_empty(), "Forbidden variable name should be flagged");
        assert!(diags[0].message.contains("forbidden"));
    }

    #[test]
    fn config_forbidden_patterns() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("ForbiddenPatterns".into(), serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("_tmp\\z".into()),
                ])),
            ]),
            ..CopConfig::default()
        };
        let source = b"data_tmp = 1\n";
        let diags = run_cop_full_with_config(&VariableName, source, config);
        assert!(!diags.is_empty(), "Variable matching forbidden pattern should be flagged");
    }
}
