use std::path::Path;

use crate::cop::util::is_snake_case;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FileName;

/// Well-known Ruby files that don't follow snake_case convention.
const ALLOWED_NAMES: &[&str] = &["Gemfile", "Rakefile", "Guardfile", "Capfile", "Berksfile"];

/// Default roots for definition path hierarchy matching.
const DEFAULT_PATH_ROOTS: &[&str] = &["lib", "spec", "test", "app"];

/// Convert a snake_case filename stem to a CamelCase module name.
fn to_module_name(stem: &str) -> String {
    stem.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

/// Build expected namespace from a file path.
fn build_expected_namespace(path: &Path, roots: &Option<Vec<String>>) -> Vec<String> {
    let root_list: Vec<&str> = match roots {
        Some(list) => list.iter().map(|s| s.as_str()).collect(),
        None => DEFAULT_PATH_ROOTS.to_vec(),
    };

    let components: Vec<&str> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    // Find the last occurrence of a root directory in the path
    let mut start_index = None;
    for (i, comp) in components.iter().enumerate().rev() {
        if root_list.contains(comp) {
            start_index = Some(i + 1);
            break;
        }
    }

    match start_index {
        Some(idx) => components[idx..]
            .iter()
            .map(|c| {
                let stem = Path::new(c)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(c);
                to_module_name(stem)
            })
            .collect(),
        None => {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            vec![to_module_name(stem)]
        }
    }
}

/// Simple text-based check for whether the source defines a class/module matching
/// the expected namespace.
fn has_matching_definition(source: &str, expected_namespace: &[String]) -> bool {
    if expected_namespace.is_empty() {
        return true;
    }

    let fqn = expected_namespace.join("::");
    let last_name = expected_namespace.last().unwrap();

    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("class ").or_else(|| trimmed.strip_prefix("module ")) {
            let def_name = rest
                .split(|c: char| c == '<' || c == '(' || c.is_whitespace())
                .next()
                .unwrap_or("")
                .trim();
            if def_name == fqn || def_name == last_name {
                return true;
            }
        }
    }
    false
}

impl Cop for FileName {
    fn name(&self) -> &'static str {
        "Naming/FileName"
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig) -> Vec<Diagnostic> {
        let expect_matching_definition = config.get_bool("ExpectMatchingDefinition", false);
        let check_def_path_hierarchy = config.get_bool("CheckDefinitionPathHierarchy", true);
        let check_def_path_roots = config.get_string_array("CheckDefinitionPathHierarchyRoots");
        let regex_pattern = config.get_str("Regex", "");
        let ignore_executable_scripts = config.get_bool("IgnoreExecutableScripts", true);
        let allowed_acronyms = config.get_string_array("AllowedAcronyms");

        let path = Path::new(source.path_str());
        let file_stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s,
            None => return Vec::new(),
        };

        // IgnoreExecutableScripts: skip files with shebang (#!) on first line
        if ignore_executable_scripts {
            let bytes = source.as_bytes();
            if bytes.starts_with(b"#!") {
                return Vec::new();
            }
        }

        // Allow well-known Ruby files
        if ALLOWED_NAMES.contains(&file_stem) {
            return Vec::new();
        }

        // Also allow if the full filename (no extension) is in the allowed list
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if ALLOWED_NAMES.contains(&file_name) {
            return Vec::new();
        }

        // Regex: if a custom regex is provided, use it instead of snake_case check
        if !regex_pattern.is_empty() {
            if let Ok(re) = regex::Regex::new(regex_pattern) {
                if re.is_match(file_stem) {
                    return Vec::new();
                }
                return vec![self.diagnostic(
                    source,
                    1,
                    0,
                    format!(
                        "The name of this source file (`{file_stem}`) should match the configured Regex."
                    ),
                )];
            }
        }

        // AllowedAcronyms: allow acronyms in snake_case names (e.g., "my_HTML_parser")
        let mut check_name = file_stem.to_string();
        if let Some(acronyms) = &allowed_acronyms {
            for acronym in acronyms {
                check_name = check_name.replace(acronym.as_str(), &acronym.to_lowercase());
            }
        }

        // RuboCop allows dots in filenames (e.g., show.html.haml_spec).
        // Check snake_case on each dot-separated segment individually.
        let all_segments_snake = check_name.split('.').all(|seg| is_snake_case(seg.as_bytes()));
        if !all_segments_snake {
            return vec![self.diagnostic(
                source,
                1,
                0,
                format!(
                    "The name of this source file (`{file_stem}`) should use snake_case."
                ),
            )];
        }

        // ExpectMatchingDefinition: require that the file defines a class/module matching the filename
        if expect_matching_definition {
            let source_text = std::str::from_utf8(source.as_bytes()).unwrap_or("");

            let expected_namespace = if check_def_path_hierarchy {
                build_expected_namespace(path, &check_def_path_roots)
            } else {
                vec![to_module_name(file_stem)]
            };

            if !has_matching_definition(source_text, &expected_namespace) {
                let namespace_str = expected_namespace.join("::");
                return vec![self.diagnostic(
                    source,
                    1,
                    0,
                    format!(
                        "`{file_stem}` should define a class or module called `{namespace_str}`."
                    ),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::source::SourceFile;

    crate::cop_scenario_fixture_tests!(
        FileName, "cops/naming/file_name",
        camel_case = "camel_case.rb",
        bad_name = "bad_name.rb",
        with_dash = "with_dash.rb",
    );

    #[test]
    fn offense_bad_filename() {
        let source = SourceFile::from_bytes("BadFile.rb", b"x = 1\n".to_vec());
        let diags = FileName.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].cop_name, "Naming/FileName");
        assert!(diags[0].message.contains("BadFile"));
    }

    #[test]
    fn offense_camel_case_filename() {
        let source = SourceFile::from_bytes("MyClass.rb", b"x = 1\n".to_vec());
        let diags = FileName.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn no_offense_good_filename() {
        let source = SourceFile::from_bytes("good_file.rb", b"x = 1\n".to_vec());
        let diags = FileName.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }

    #[test]
    fn no_offense_gemfile() {
        let source = SourceFile::from_bytes("Gemfile", b"source 'https://rubygems.org'\n".to_vec());
        let diags = FileName.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }

    #[test]
    fn no_offense_rakefile() {
        let source = SourceFile::from_bytes("Rakefile", b"task :default\n".to_vec());
        let diags = FileName.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }

    #[test]
    fn no_offense_test_rb() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\n".to_vec());
        let diags = FileName.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }

    #[test]
    fn config_ignore_executable_scripts() {
        let source = SourceFile::from_bytes("MyScript", b"#!/usr/bin/env ruby\nputs 'hi'\n".to_vec());
        let diags = FileName.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty(), "Should skip executable scripts with shebang");
    }

    #[test]
    fn config_ignore_executable_scripts_false() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("IgnoreExecutableScripts".into(), serde_yml::Value::Bool(false)),
            ]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes("MyScript.rb", b"#!/usr/bin/env ruby\nputs 'hi'\n".to_vec());
        let diags = FileName.check_lines(&source, &config);
        assert!(!diags.is_empty(), "Should flag non-snake_case even with shebang when IgnoreExecutableScripts:false");
    }

    #[test]
    fn config_regex() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Regex".into(), serde_yml::Value::String(r"\A[a-z_]+\z".into())),
            ]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes("good_file.rb", b"x = 1\n".to_vec());
        let diags = FileName.check_lines(&source, &config);
        assert!(diags.is_empty(), "Should pass custom regex check");
    }

    #[test]
    fn config_allowed_acronyms() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("AllowedAcronyms".into(), serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("HTML".into()),
                ])),
            ]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes("my_HTML_parser.rb", b"x = 1\n".to_vec());
        let diags = FileName.check_lines(&source, &config);
        assert!(diags.is_empty(), "Should allow AllowedAcronyms in filename");
    }

    #[test]
    fn expect_matching_definition_flags_missing_class() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("ExpectMatchingDefinition".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes("my_class.rb", b"x = 1\n".to_vec());
        let diags = FileName.check_lines(&source, &config);
        assert!(!diags.is_empty(), "ExpectMatchingDefinition should flag file without matching class");
        assert!(diags[0].message.contains("MyClass"));
    }

    #[test]
    fn expect_matching_definition_allows_matching_class() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("ExpectMatchingDefinition".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes("my_class.rb", b"class MyClass\nend\n".to_vec());
        let diags = FileName.check_lines(&source, &config);
        assert!(diags.is_empty(), "ExpectMatchingDefinition should accept matching class");
    }

    #[test]
    fn expect_matching_definition_with_path_hierarchy() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("ExpectMatchingDefinition".into(), serde_yml::Value::Bool(true)),
                ("CheckDefinitionPathHierarchy".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes(
            "lib/my_gem/my_class.rb",
            b"class MyGem::MyClass\nend\n".to_vec(),
        );
        let diags = FileName.check_lines(&source, &config);
        assert!(diags.is_empty(), "Should accept matching namespaced class");
    }

    #[test]
    fn expect_matching_definition_no_hierarchy() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("ExpectMatchingDefinition".into(), serde_yml::Value::Bool(true)),
                ("CheckDefinitionPathHierarchy".into(), serde_yml::Value::Bool(false)),
            ]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes(
            "lib/my_gem/my_class.rb",
            b"class MyClass\nend\n".to_vec(),
        );
        let diags = FileName.check_lines(&source, &config);
        assert!(diags.is_empty(), "Without hierarchy check, just the class name should match");
    }
}
