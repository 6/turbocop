use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DeprecatedConstants;

/// Built-in deprecated constants when no config is provided.
const BUILTIN_DEPRECATED: &[(&str, &str)] = &[
    ("NIL", "nil"),
    ("TRUE", "true"),
    ("FALSE", "false"),
];

impl Cop for DeprecatedConstants {
    fn name(&self) -> &'static str {
        "Lint/DeprecatedConstants"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Handle ConstantReadNode (bare constants like NIL, TRUE, FALSE)
        if let Some(const_read) = node.as_constant_read_node() {
            let name = const_read.name().as_slice();
            let name_str = match std::str::from_utf8(name) {
                Ok(s) => s,
                Err(_) => return Vec::new(),
            };

            if let Some(msg) = deprecated_message(name_str, config) {
                let loc = const_read.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(source, line, column, msg)];
            }
        }

        // Handle ConstantPathNode (qualified constants like Net::HTTPServerException)
        if let Some(const_path) = node.as_constant_path_node() {
            let loc = const_path.location();
            let full_name = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
            let name_str = match std::str::from_utf8(full_name) {
                Ok(s) => s,
                Err(_) => return Vec::new(),
            };

            if let Some(msg) = deprecated_message(name_str, config) {
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(source, line, column, msg)];
            }
        }

        Vec::new()
    }
}

fn deprecated_message(constant_name: &str, config: &CopConfig) -> Option<String> {
    // Check config-defined DeprecatedConstants first
    let _deprecated_constants = config.get_string_hash("DeprecatedConstants");

    // Try to find in config hash (nested structure)
    if let Some(val) = config.options.get("DeprecatedConstants") {
        if let Some(mapping) = val.as_mapping() {
            for (k, v) in mapping.iter() {
                if let Some(key_str) = k.as_str() {
                    if key_str == constant_name {
                        let alternative = v
                            .as_mapping()
                            .and_then(|m| {
                                m.iter().find_map(|(mk, mv)| {
                                    if mk.as_str() == Some("Alternative") {
                                        mv.as_str().map(|s| s.to_string())
                                    } else {
                                        None
                                    }
                                })
                            });

                        return if let Some(alt) = alternative {
                            Some(format!("Use `{alt}` instead of `{constant_name}`."))
                        } else {
                            Some(format!("Do not use `{constant_name}`."))
                        };
                    }
                }
            }
        }
    }

    // Fall back to built-in defaults
    for &(name, alt) in BUILTIN_DEPRECATED {
        if name == constant_name {
            return Some(format!("Use `{alt}` instead of `{constant_name}`."));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DeprecatedConstants, "cops/lint/deprecated_constants");
}
