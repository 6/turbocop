use crate::cop::shared::node_type::{CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-15)
///
/// Corpus oracle reported FP=0, FN=2.
///
/// FN fixes:
/// - Prism represents `::Random::DEFAULT` as a `ConstantPathNode` whose source
///   text includes the leading `::`, but config keys are stored without that
///   prefix. Normalize cbase-qualified constant paths before config lookup.
/// - Config-driven diagnostics need to preserve `DeprecatedVersion` in the
///   rendered message so the output matches RuboCop.
/// - RuboCop's built-in default list also includes version-gated deprecations
///   like `Random::DEFAULT`, `Struct::Group`, and `Struct::Passwd`. Mirror the
///   full default table and honor `TargetRubyVersion` before flagging them.
///
/// Remaining FP/FN were not reported for this cop in the current corpus run.
pub struct DeprecatedConstants;

/// Built-in deprecated constants when no config is provided.
const BUILTIN_DEPRECATED: &[(&str, &str, &str)] = &[
    ("NIL", "nil", "2.4"),
    ("TRUE", "true", "2.4"),
    ("FALSE", "false", "2.4"),
    (
        "Net::HTTPServerException",
        "Net::HTTPClientException",
        "2.6",
    ),
    ("Random::DEFAULT", "Random.new", "3.0"),
    ("Struct::Group", "Etc::Group", "3.0"),
    ("Struct::Passwd", "Etc::Passwd", "3.0"),
];

impl Cop for DeprecatedConstants {
    fn name(&self) -> &'static str {
        "Lint/DeprecatedConstants"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CONSTANT_PATH_NODE, CONSTANT_READ_NODE]
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
        // Handle ConstantReadNode (bare constants like NIL, TRUE, FALSE)
        if let Some(const_read) = node.as_constant_read_node() {
            let name = const_read.name().as_slice();
            let name_str = match std::str::from_utf8(name) {
                Ok(s) => s,
                Err(_) => return,
            };

            if let Some(msg) = deprecated_message(name_str, config) {
                let loc = const_read.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(source, line, column, msg));
            }
        }

        // Handle ConstantPathNode (qualified constants like Net::HTTPServerException)
        if let Some(const_path) = node.as_constant_path_node() {
            let loc = const_path.location();
            let full_name = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
            let name_str = match std::str::from_utf8(full_name) {
                Ok(s) => s,
                Err(_) => return,
            };

            if let Some(msg) = deprecated_message(name_str, config) {
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(source, line, column, msg));
            }
        }
    }
}

#[derive(Default)]
struct DeprecatedConstantInfo {
    alternative: Option<String>,
    deprecated_version: Option<String>,
}

fn normalize_constant_name(constant_name: &str) -> &str {
    constant_name.strip_prefix("::").unwrap_or(constant_name)
}

fn target_ruby_version(config: &CopConfig) -> f64 {
    config
        .options
        .get("TargetRubyVersion")
        .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
        .unwrap_or(2.7)
}

fn deprecated_info(constant_name: &str, config: &CopConfig) -> Option<DeprecatedConstantInfo> {
    let normalized_name = normalize_constant_name(constant_name);

    if let Some(val) = config.options.get("DeprecatedConstants") {
        if let Some(mapping) = val.as_mapping() {
            for (k, v) in mapping.iter() {
                if let Some(key_str) = k.as_str() {
                    if normalize_constant_name(key_str) == normalized_name {
                        let mut info = DeprecatedConstantInfo::default();

                        if let Some(value_mapping) = v.as_mapping() {
                            for (field, field_value) in value_mapping.iter() {
                                match field.as_str() {
                                    Some("Alternative") => {
                                        info.alternative = field_value.as_str().map(str::to_string)
                                    }
                                    Some("DeprecatedVersion") => {
                                        info.deprecated_version =
                                            field_value.as_str().map(str::to_string)
                                    }
                                    _ => {}
                                }
                            }
                        }

                        return Some(info);
                    }
                }
            }
        }
    }

    for &(name, alt, version) in BUILTIN_DEPRECATED {
        if name == normalized_name {
            return Some(DeprecatedConstantInfo {
                alternative: Some(alt.to_string()),
                deprecated_version: Some(version.to_string()),
            });
        }
    }

    None
}

fn deprecated_message(constant_name: &str, config: &CopConfig) -> Option<String> {
    let info = deprecated_info(constant_name, config)?;

    if let Some(version) = &info.deprecated_version {
        let version_number = version.parse::<f64>().ok()?;
        if target_ruby_version(config) < version_number {
            return None;
        }
    }

    match (info.alternative, info.deprecated_version) {
        (Some(alternative), Some(version)) => Some(format!(
            "Use `{alternative}` instead of `{constant_name}`, deprecated since Ruby {version}."
        )),
        (Some(alternative), None) => {
            Some(format!("Use `{alternative}` instead of `{constant_name}`."))
        }
        (None, Some(version)) => Some(format!(
            "Do not use `{constant_name}`, deprecated since Ruby {version}."
        )),
        (None, None) => Some(format!("Do not use `{constant_name}`.")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DeprecatedConstants, "cops/lint/deprecated_constants");

    #[test]
    fn flags_cbase_configured_constant_paths() {
        let mut config = CopConfig::default();
        config.options.insert(
            "TargetRubyVersion".to_string(),
            serde_yml::from_str("3.0").unwrap(),
        );
        config.options.insert(
            "DeprecatedConstants".to_string(),
            serde_yml::from_str(
                r#"
Random::DEFAULT:
  Alternative: Random.new
  DeprecatedVersion: "3.0"
"#,
            )
            .unwrap(),
        );

        crate::testutil::assert_cop_offenses_full_with_config(
            &DeprecatedConstants,
            b"::Random::DEFAULT\n^^^^^^^^^^^^^^^^^ Lint/DeprecatedConstants: Use `Random.new` instead of `::Random::DEFAULT`, deprecated since Ruby 3.0.\n",
            config,
        );
    }

    #[test]
    fn flags_builtin_random_default_when_used_as_receiver() {
        let mut config = CopConfig::default();
        config.options.insert(
            "TargetRubyVersion".to_string(),
            serde_yml::from_str("3.0").unwrap(),
        );

        crate::testutil::assert_cop_offenses_full_with_config(
            &DeprecatedConstants,
            b"::Random::DEFAULT.rand(max)\n^^^^^^^^^^^^^^^^^ Lint/DeprecatedConstants: Use `Random.new` instead of `::Random::DEFAULT`, deprecated since Ruby 3.0.\n",
            config,
        );
    }
}
