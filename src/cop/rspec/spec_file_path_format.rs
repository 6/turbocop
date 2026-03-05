use crate::cop::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_example_group};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/SpecFilePathFormat — checks that spec file paths match the described class.
///
/// ## Root cause of prior FPs/FNs
///
/// Two issues caused ~27 FPs and ~1,075 FNs:
///
/// 1. **Missing `top_level_nodes()` recursion (~1,050 FNs):** The old implementation only
///    checked direct children of ProgramNode. RuboCop's `TopLevelGroup` mixin recursively
///    unwraps `module`, `class`, and `begin` wrappers to find example groups nested inside
///    namespace modules (e.g., `module Foo; describe Bar do; end; end`). Without this
///    recursion, most real-world specs were missed entirely.
///
/// 2. **Missing namespace extraction (~900 FNs, overlapping with #1):** RuboCop's `Namespace`
///    mixin traverses ancestor `module`/`class` nodes and prepends their names to the expected
///    path. For example, `module Foo; describe Bar; end` expects `foo/bar*_spec.rb`. The old
///    implementation had no namespace awareness, so even when it found a describe inside a
///    module, it generated the wrong expected path.
///
/// ## Fix
///
/// - Switched from `check_node(PROGRAM_NODE)` to `check_source` with manual AST traversal.
/// - Implemented `top_level_nodes()` that recursively unwraps module/class/begin wrappers,
///   mirroring RuboCop's `TopLevelGroup#top_level_nodes`.
/// - Implemented namespace extraction that collects enclosing module/class names when
///   traversing into wrappers, mirroring RuboCop's `Namespace#namespace`.
/// - CustomTransform is checked per-component (namespace + class parts individually).
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

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let custom_transform = config
            .get_string_hash("CustomTransform")
            .unwrap_or_default();
        let ignore_methods = config.get_bool("IgnoreMethods", false);
        let ignore_metadata = config.get_string_hash("IgnoreMetadata").unwrap_or_default();
        let _inflector_path = config.get_str("InflectorPath", "");
        let _enforced_inflector = config.get_str("EnforcedInflector", "default");

        let program = match parse_result.node().as_program_node() {
            Some(p) => p,
            None => return,
        };

        let stmts: Vec<ruby_prism::Node<'_>> = program.statements().body().iter().collect();

        // Collect top-level example group calls, unwrapping module/class/begin wrappers.
        // This mirrors RuboCop's TopLevelGroup#top_level_nodes.
        let mut found: Vec<(ruby_prism::CallNode<'_>, Vec<String>)> = Vec::new();
        let namespace: Vec<String> = Vec::new();
        collect_top_level_describes(&stmts, source, &namespace, &mut found);

        // If not exactly one top-level describe, skip (ambiguous or none)
        if found.len() != 1 {
            return;
        }

        let (call, namespace) = &found[0];
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
            std::str::from_utf8(cr.name().as_slice())
                .unwrap_or("")
                .to_string()
        } else if let Some(cp) = first_arg.as_constant_path_node() {
            let loc = cp.location();
            let text = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
            let s = std::str::from_utf8(text).unwrap_or("");
            s.trim_start_matches("::").to_string()
        } else {
            return;
        };

        // IgnoreMetadata: skip check if metadata matches ignored key:value pairs
        if !ignore_metadata.is_empty() && arg_list.len() >= 2 {
            for arg in &arg_list[1..] {
                if let Some(hash) = arg.as_keyword_hash_node() {
                    for elem in hash.elements().iter() {
                        if let Some(assoc) = elem.as_assoc_node() {
                            if let Some(sym) = assoc.key().as_symbol_node() {
                                let key_str = std::str::from_utf8(sym.unescaped()).unwrap_or("");
                                if let Some(expected_value) = ignore_metadata.get(key_str) {
                                    let actual_value = if let Some(val_sym) =
                                        assoc.value().as_symbol_node()
                                    {
                                        std::str::from_utf8(val_sym.unescaped())
                                            .unwrap_or("")
                                            .to_string()
                                    } else if let Some(val_str) = assoc.value().as_string_node() {
                                        std::str::from_utf8(val_str.unescaped())
                                            .unwrap_or("")
                                            .to_string()
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

        // Build expected path: namespace segments + class name segments
        let expected_path = build_expected_path(namespace, &class_name, &custom_transform);

        // Get optional method description from second argument
        let method_part = if ignore_methods {
            None
        } else if arg_list.len() >= 2 {
            if let Some(s) = arg_list[1].as_string_node() {
                let val = std::str::from_utf8(s.unescaped()).unwrap_or("");
                let cleaned: String = val
                    .chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == '_' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect();
                let cleaned = cleaned.trim_matches('_').to_string();
                if cleaned.is_empty() {
                    None
                } else {
                    Some(cleaned)
                }
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

/// Recursively unwrap module/class/begin wrappers to find top-level describe calls.
/// This mirrors RuboCop's `TopLevelGroup#top_level_nodes` + `Namespace#namespace`.
fn collect_top_level_describes<'pr>(
    stmts: &[ruby_prism::Node<'pr>],
    source: &SourceFile,
    namespace: &[String],
    found: &mut Vec<(ruby_prism::CallNode<'pr>, Vec<String>)>,
) {
    for stmt in stmts {
        if let Some(call) = stmt.as_call_node() {
            let name = call.name().as_slice();
            if is_rspec_example_group(name)
                && name != b"shared_examples"
                && name != b"shared_examples_for"
                && name != b"shared_context"
            {
                found.push((call, namespace.to_vec()));
            }
            continue;
        }

        if let Some(module_node) = stmt.as_module_node() {
            let module_names = extract_defined_name(source, &module_node.constant_path());
            if !module_names.is_empty() {
                let mut new_ns = namespace.to_vec();
                new_ns.extend(module_names);
                if let Some(body) = module_node.body() {
                    let children: Vec<_> = body
                        .as_statements_node()
                        .iter()
                        .flat_map(|s| s.body().iter())
                        .collect();
                    collect_top_level_describes(&children, source, &new_ns, found);
                }
            }
            continue;
        }

        if let Some(class_node) = stmt.as_class_node() {
            let class_names = extract_defined_name(source, &class_node.constant_path());
            if !class_names.is_empty() {
                let mut new_ns = namespace.to_vec();
                new_ns.extend(class_names);
                if let Some(body) = class_node.body() {
                    let children: Vec<_> = body
                        .as_statements_node()
                        .iter()
                        .flat_map(|s| s.body().iter())
                        .collect();
                    collect_top_level_describes(&children, source, &new_ns, found);
                }
            }
            continue;
        }

        if let Some(begin_node) = stmt.as_begin_node() {
            if let Some(stmts_node) = begin_node.statements() {
                let children: Vec<_> = stmts_node.body().iter().collect();
                collect_top_level_describes(&children, source, namespace, found);
            }
        }
    }
}

/// Extract the defined name segments from a module/class constant path.
fn extract_defined_name(source: &SourceFile, node: &ruby_prism::Node<'_>) -> Vec<String> {
    if let Some(cr) = node.as_constant_read_node() {
        let name = std::str::from_utf8(cr.name().as_slice()).unwrap_or("");
        return vec![name.to_string()];
    }
    if let Some(cp) = node.as_constant_path_node() {
        let loc = cp.location();
        let text = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
        let s = std::str::from_utf8(text).unwrap_or("");
        let s = s.trim_start_matches("::");
        return s.split("::").map(|p| p.to_string()).collect();
    }
    Vec::new()
}

/// Build the expected file path from namespace + class name, applying CustomTransform.
fn build_expected_path(
    namespace: &[String],
    class_name: &str,
    custom_transform: &std::collections::HashMap<String, String>,
) -> String {
    let class_parts: Vec<&str> = class_name.split("::").collect();
    let all_segments: Vec<String> = namespace
        .iter()
        .map(|s| s.to_string())
        .chain(class_parts.iter().map(|s| s.to_string()))
        .collect();

    let path_parts: Vec<String> = all_segments
        .iter()
        .map(|name| {
            if let Some(custom) = custom_transform.get(name.as_str()) {
                custom.clone()
            } else {
                camel_to_snake(name)
            }
        })
        .collect();

    path_parts.join("/")
}

fn camel_to_snake(s: &str) -> String {
    crate::schema::camel_to_snake(s)
}

fn path_matches(path: &str, expected_class: &str, method: Option<&str>) -> bool {
    let path_lower = path.to_lowercase();
    let class_lower = expected_class.to_lowercase();

    if !path_lower.contains(&class_lower) {
        return false;
    }

    if !path_lower.ends_with("_spec.rb") {
        return false;
    }

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
        SpecFilePathFormat,
        "cops/rspec/spec_file_path_format",
        scenario_wrong_class = "wrong_class.rb",
        scenario_wrong_method = "wrong_method.rb",
        scenario_wrong_path = "wrong_path.rb",
        scenario_module_wrong_path = "module_wrong_path.rb",
        scenario_nested_module_wrong_path = "nested_module_wrong_path.rb",
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
        let source = b"describe MyClass do\nend\n";
        let diags =
            crate::testutil::run_cop_full_with_config(&SpecFilePathFormat, source, config.clone());
        assert!(!diags.is_empty(), "Should still flag with wrong filename");
        assert!(
            diags[0].message.contains("custom_dir"),
            "Expected path should use custom_dir from CustomTransform, got: {}",
            diags[0].message
        );
    }

    #[test]
    fn custom_transform_with_namespace() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let mut transform = serde_yml::Mapping::new();
        transform.insert(
            serde_yml::Value::String("FooFoo".into()),
            serde_yml::Value::String("foofoo".into()),
        );
        let config = CopConfig {
            options: HashMap::from([(
                "CustomTransform".into(),
                serde_yml::Value::Mapping(transform),
            )]),
            ..CopConfig::default()
        };
        let source = b"describe FooFoo::Some::Class, '#bar' do; end\n";
        let diags = crate::testutil::run_cop_full_internal(
            &SpecFilePathFormat,
            source,
            config,
            "foofoo/some/class/bar_spec.rb",
        );
        assert!(
            diags.is_empty(),
            "CustomTransform should apply to namespace parts, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
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
        let source = b"describe MyClass, type: :routing do\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&SpecFilePathFormat, source, config);
        assert!(
            diags.is_empty(),
            "IgnoreMetadata should skip path check when metadata value matches"
        );
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
        let source = b"describe MyClass, type: :controller do\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&SpecFilePathFormat, source, config);
        assert!(
            !diags.is_empty(),
            "IgnoreMetadata should NOT skip when metadata value differs"
        );
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
            options: HashMap::from([("IgnoreMethods".into(), serde_yml::Value::Bool(true))]),
            ..CopConfig::default()
        };
        let source = b"describe MyClass, '#create' do\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&SpecFilePathFormat, source, config);
        assert!(
            diags.iter().all(|d| !d.message.contains("create")),
            "IgnoreMethods should not check method part"
        );
    }

    #[test]
    fn module_wrapped_describe_no_offense() {
        let source = b"module Very\n  module Medium\n    describe MyClass do; end\n  end\nend\n";
        let diags = crate::testutil::run_cop_full_internal(
            &SpecFilePathFormat,
            source,
            CopConfig::default(),
            "very/medium/my_class_spec.rb",
        );
        assert!(
            diags.is_empty(),
            "Should not flag when path matches namespace + class, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn module_wrapped_describe_offense() {
        let source = b"module Very\n  module Medium\n    describe MyClass do; end\n  end\nend\n";
        let diags = crate::testutil::run_cop_full_internal(
            &SpecFilePathFormat,
            source,
            CopConfig::default(),
            "very/long/my_class_spec.rb",
        );
        assert!(
            !diags.is_empty(),
            "Should flag when path doesn't match namespace"
        );
        assert!(
            diags[0].message.contains("very/medium/my_class"),
            "Message should include namespace path, got: {}",
            diags[0].message
        );
    }

    #[test]
    fn class_wrapped_describe_with_namespace() {
        let source = b"class MyApp\n  describe Widget do; end\nend\n";
        let diags = crate::testutil::run_cop_full_internal(
            &SpecFilePathFormat,
            source,
            CopConfig::default(),
            "my_app/widget_spec.rb",
        );
        assert!(
            diags.is_empty(),
            "Should not flag when path matches class-namespace + describe class, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn no_describe_in_file_no_offense() {
        let source = b"class Foo\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(
            &SpecFilePathFormat,
            source,
            CopConfig::default(),
        );
        assert!(
            diags.is_empty(),
            "Should not flag files without describe calls"
        );
    }
}
