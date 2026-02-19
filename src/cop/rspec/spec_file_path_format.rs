use crate::cop::util::{is_rspec_example_group, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ASSOC_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, KEYWORD_HASH_NODE, PROGRAM_NODE, STRING_NODE, SYMBOL_NODE};

pub struct SpecFilePathFormat;

impl Cop for SpecFilePathFormat {
    fn name(&self) -> &'static str {
        "RSpec/SpecFilePathFormat"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, KEYWORD_HASH_NODE, PROGRAM_NODE, STRING_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Config: CustomTransform — hash of class name to file path overrides (complex; pass-through)
        let custom_transform = config.get_string_hash("CustomTransform").unwrap_or_default();
        // Config: IgnoreMethods — when true, skip method description part in path matching
        let ignore_methods = config.get_bool("IgnoreMethods", false);
        // Config: IgnoreMetadata — metadata keys whose values should be ignored in path matching
        let ignore_metadata = config.get_string_hash("IgnoreMetadata").unwrap_or_default();
        // Config: InflectorPath — path to Zeitwerk inflector (Ruby-specific, no-op in Rust)
        let _inflector_path = config.get_str("InflectorPath", "");
        // Config: EnforcedInflector — which inflector to use (only "default" supported in Rust)
        let enforced_inflector = config.get_str("EnforcedInflector", "default");

        // Only check ProgramNode (root) so we examine top-level describes
        let program = match node.as_program_node() {
            Some(p) => p,
            None => return,
        };

        let stmts = program.statements();
        let body = stmts.body();

        // Collect top-level describe calls
        let mut describes: Vec<ruby_prism::CallNode<'_>> = Vec::new();
        for stmt in body.iter() {
            if let Some(call) = stmt.as_call_node() {
                let name = call.name().as_slice();
                if !is_rspec_example_group(name) {
                    continue;
                }
                // Skip shared examples
                if name == b"shared_examples" || name == b"shared_examples_for" || name == b"shared_context" {
                    continue;
                }
                describes.push(call);
            }
        }

        // If multiple top-level describes, skip (ambiguous)
        if describes.len() != 1 {
            return;
        }

        let call = &describes[0];
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // First arg must be a constant (class name)
        let first_arg = &arg_list[0];
        let class_name = if let Some(cr) = first_arg.as_constant_read_node() {
            std::str::from_utf8(cr.name().as_slice()).unwrap_or("").to_string()
        } else if let Some(cp) = first_arg.as_constant_path_node() {
            let loc = cp.location();
            let text = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
            let s = std::str::from_utf8(text).unwrap_or("");
            // Strip leading ::
            s.trim_start_matches("::").to_string()
        } else {
            return;
        };

        // CustomTransform: override class name → path segment mappings
        let expected_path = if let Some(custom_path) = custom_transform.get(&class_name) {
            custom_path.clone()
        } else {
            // Apply CustomTransform to individual parts of namespaced classes
            let parts: Vec<&str> = class_name.split("::").collect();
            let snake_parts: Vec<String> = parts.iter().map(|p| {
                if let Some(custom) = custom_transform.get(*p) {
                    custom.clone()
                } else {
                    camel_to_snake(p)
                }
            }).collect();
            snake_parts.join("/")
        };

        // IgnoreMetadata: if the describe call has metadata matching key:value pairs in
        // ignore_metadata, skip this check. E.g., `type: routing` means skip when metadata
        // has `type: :routing`.
        if !ignore_metadata.is_empty() && arg_list.len() >= 2 {
            for arg in &arg_list[1..] {
                if let Some(hash) = arg.as_keyword_hash_node() {
                    for elem in hash.elements().iter() {
                        if let Some(assoc) = elem.as_assoc_node() {
                            if let Some(sym) = assoc.key().as_symbol_node() {
                                let key_str = std::str::from_utf8(sym.unescaped()).unwrap_or("");
                                if let Some(expected_value) = ignore_metadata.get(key_str) {
                                    // Extract the actual metadata value
                                    let actual_value = if let Some(val_sym) = assoc.value().as_symbol_node() {
                                        std::str::from_utf8(val_sym.unescaped()).unwrap_or("").to_string()
                                    } else if let Some(val_str) = assoc.value().as_string_node() {
                                        std::str::from_utf8(val_str.unescaped()).unwrap_or("").to_string()
                                    } else {
                                        String::new()
                                    };
                                    if actual_value == *expected_value {
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // EnforcedInflector: only "default" is supported; other inflectors require
        // Ruby-specific Zeitwerk integration (InflectorPath) which is not available in Rust.
        let _ = enforced_inflector;

        // Get optional second string argument for method description
        // When IgnoreMethods is true, skip the method part entirely
        let method_part = if ignore_methods {
            None
        } else if arg_list.len() >= 2 {
            if let Some(s) = arg_list[1].as_string_node() {
                let val = std::str::from_utf8(s.unescaped()).unwrap_or("");
                // Convert to path-friendly form
                let cleaned: String = val.chars()
                    .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
                    .collect();
                let cleaned = cleaned.trim_matches('_').to_string();
                if cleaned.is_empty() { None } else { Some(cleaned) }
            } else {
                None
            }
        } else {
            None
        };

        let expected_suffix = match &method_part {
            Some(m) => format!("{expected_path}*{m}*_spec.rb"),
            None => format!("{expected_path}*_spec.rb"),
        };

        // Check if the file path matches
        let file_path = source.path_str();
        let normalized = file_path.replace('\\', "/");

        if !path_matches(&normalized, &expected_path, method_part.as_deref()) {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Spec path should end with `{expected_suffix}`."),
            ));
        }

    }
}

fn camel_to_snake(s: &str) -> String {
    // Matches Ruby's ActiveSupport `underscore` method:
    // 1. Insert underscore between acronym run and next word: "URLValidator" → "URL_Validator"
    // 2. Insert underscore between lowercase/digit and uppercase: "fooBar" → "foo_Bar"
    // 3. Lowercase everything
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() && i > 0 {
            let prev = chars[i - 1];
            if prev.is_lowercase() || prev.is_ascii_digit() {
                // Pattern: lowercase/digit followed by uppercase → insert underscore
                result.push('_');
            } else if prev.is_uppercase() {
                // Check if next char is lowercase (end of acronym)
                // "URL" + "Validator" → at 'V', prev='L' is upper, next='a' is lower
                // But at 'L' in "URL", prev='R' is upper, next='V' is upper → no underscore
                if i + 1 < chars.len() && chars[i + 1].is_lowercase() {
                    // This uppercase char starts a new word after an acronym
                    result.push('_');
                }
            }
        }
        result.push(c.to_ascii_lowercase());
    }
    result
}

fn path_matches(path: &str, expected_class: &str, method: Option<&str>) -> bool {
    // Check that the path ends with the expected class path and _spec.rb
    let path_lower = path.to_lowercase();
    let class_lower = expected_class.to_lowercase();

    // Must contain the class path
    if !path_lower.contains(&class_lower) {
        return false;
    }

    // Must end with _spec.rb
    if !path_lower.ends_with("_spec.rb") {
        return false;
    }

    // If there's a method part, it should appear in the path too
    if let Some(m) = method {
        let m_lower = m.to_lowercase();
        if !path_lower.contains(&m_lower) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_scenario_fixture_tests!(
        SpecFilePathFormat, "cops/rspec/spec_file_path_format",
        scenario_wrong_class = "wrong_class.rb",
        scenario_wrong_method = "wrong_method.rb",
        scenario_wrong_path = "wrong_path.rb",
    );

    #[test]
    fn custom_transform_overrides_class_path() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let mut transform = serde_yml::Mapping::new();
        transform.insert(
            serde_yml::Value::String("MyClass".into()),
            serde_yml::Value::String("custom_dir".into()),
        );
        let config = CopConfig {
            options: HashMap::from([(
                "CustomTransform".into(),
                serde_yml::Value::Mapping(transform),
            )]),
            ..CopConfig::default()
        };
        // Without CustomTransform, MyClass maps to my_class — with it, maps to custom_dir
        // The test.rb filename won't match either way, but the expected path in the message differs
        let source = b"describe MyClass do\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&SpecFilePathFormat, source, config.clone());
        assert!(!diags.is_empty(), "Should still flag with wrong filename");
        assert!(diags[0].message.contains("custom_dir"), "Expected path should use custom_dir from CustomTransform, got: {}", diags[0].message);
    }

    #[test]
    fn ignore_metadata_skips_check_when_value_matches() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let mut ignore_meta = serde_yml::Mapping::new();
        ignore_meta.insert(
            serde_yml::Value::String("type".into()),
            serde_yml::Value::String("routing".into()),
        );
        let config = CopConfig {
            options: HashMap::from([(
                "IgnoreMetadata".into(),
                serde_yml::Value::Mapping(ignore_meta),
            )]),
            ..CopConfig::default()
        };
        // describe with metadata value matching the ignored value
        let source = b"describe MyClass, type: :routing do\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&SpecFilePathFormat, source, config);
        assert!(diags.is_empty(), "IgnoreMetadata should skip path check when metadata value matches");
    }

    #[test]
    fn ignore_metadata_does_not_skip_when_value_differs() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let mut ignore_meta = serde_yml::Mapping::new();
        ignore_meta.insert(
            serde_yml::Value::String("type".into()),
            serde_yml::Value::String("routing".into()),
        );
        let config = CopConfig {
            options: HashMap::from([(
                "IgnoreMetadata".into(),
                serde_yml::Value::Mapping(ignore_meta),
            )]),
            ..CopConfig::default()
        };
        // describe with metadata value NOT matching the ignored value
        let source = b"describe MyClass, type: :controller do\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&SpecFilePathFormat, source, config);
        assert!(!diags.is_empty(), "IgnoreMetadata should NOT skip when metadata value differs");
    }

    #[test]
    fn camel_to_snake_handles_acronyms() {
        assert_eq!(camel_to_snake("URLValidator"), "url_validator");
        assert_eq!(camel_to_snake("MyClass"), "my_class");
        assert_eq!(camel_to_snake("HTTPSConnection"), "https_connection");
        assert_eq!(camel_to_snake("FooBar"), "foo_bar");
        assert_eq!(camel_to_snake("Foo"), "foo");
        assert_eq!(camel_to_snake("API"), "api");
        assert_eq!(camel_to_snake("HTMLParser"), "html_parser");
    }

    #[test]
    fn ignore_methods_skips_method_check() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "IgnoreMethods".into(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };
        // Source describes MyClass with method "#create", but file doesn't have method in path
        // With IgnoreMethods=true, only the class part is checked
        let source = b"describe MyClass, '#create' do\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&SpecFilePathFormat, source, config);
        // Even with wrong filename (test.rb), the class part won't match, so there will be an offense
        // But the key thing is that the method part is ignored
        assert!(
            diags.iter().all(|d| !d.message.contains("create")),
            "IgnoreMethods should not check method part"
        );
    }
}
