use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use globset::GlobBuilder;
use serde_yml::Value;

use crate::cop::CopConfig;
use crate::diagnostic::Severity;

/// Resolved configuration from .rubocop.yml.
///
/// At M0 this is a stub: reads a single YAML file, extracts per-cop
/// Enabled/Severity/Exclude/Include and AllCops.Exclude. No inherit_from
/// or inherit_gem resolution.
#[derive(Debug)]
pub struct ResolvedConfig {
    #[allow(dead_code)]
    raw: Value,
    /// Per-cop configs keyed by cop name (e.g. "Style/FrozenStringLiteralComment")
    cop_configs: HashMap<String, CopConfig>,
    global_excludes: Vec<String>,
}

impl ResolvedConfig {
    fn empty() -> Self {
        Self {
            raw: Value::Null,
            cop_configs: HashMap::new(),
            global_excludes: Vec::new(),
        }
    }
}

/// Load config from the given path, or look for `.rubocop.yml` in the
/// current directory. Returns an empty config if the file doesn't exist.
pub fn load_config(path: Option<&Path>) -> Result<ResolvedConfig> {
    let config_path = match path {
        Some(p) => p.to_path_buf(),
        None => Path::new(".rubocop.yml").to_path_buf(),
    };

    if !config_path.exists() {
        return Ok(ResolvedConfig::empty());
    }

    let contents = std::fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read config {}", config_path.display()))?;
    let raw: Value =
        serde_yml::from_str(&contents).with_context(|| "failed to parse .rubocop.yml")?;

    let mut cop_configs = HashMap::new();
    let mut global_excludes = Vec::new();

    if let Value::Mapping(map) = &raw {
        for (key, value) in map {
            let key_str = match key.as_str() {
                Some(s) => s,
                None => continue,
            };

            if key_str == "AllCops" {
                if let Some(excludes) = extract_string_list(value, "Exclude") {
                    global_excludes = excludes;
                }
                continue;
            }

            // Cop names contain "/" (e.g. "Style/FrozenStringLiteralComment")
            if key_str.contains('/') {
                let cop_config = parse_cop_config(value);
                cop_configs.insert(key_str.to_string(), cop_config);
            }
        }
    }

    Ok(ResolvedConfig {
        raw,
        cop_configs,
        global_excludes,
    })
}

impl ResolvedConfig {
    /// Check if a cop is enabled for the given file path.
    ///
    /// Evaluates in order:
    /// 1. If the cop is explicitly disabled, return false.
    /// 2. If global excludes match the path, return false.
    /// 3. Merge cop's default_include/default_exclude with user config overrides.
    /// 4. If effective Include is non-empty, path must match at least one pattern.
    /// 5. If effective Exclude is non-empty, path must NOT match any pattern.
    pub fn is_cop_enabled(
        &self,
        name: &str,
        path: &Path,
        default_include: &[&str],
        default_exclude: &[&str],
    ) -> bool {
        let config = self.cop_configs.get(name);

        // 1. Explicit Enabled: false
        if let Some(c) = config {
            if !c.enabled {
                return false;
            }
        }

        // 2. Global excludes
        for pattern in &self.global_excludes {
            if glob_matches(pattern, path) {
                return false;
            }
        }

        // 3. Build effective include/exclude lists.
        //    User config overrides defaults when non-empty.
        let effective_include: Vec<&str> = match config {
            Some(c) if !c.include.is_empty() => c.include.iter().map(|s| s.as_str()).collect(),
            _ => default_include.to_vec(),
        };
        let effective_exclude: Vec<&str> = match config {
            Some(c) if !c.exclude.is_empty() => c.exclude.iter().map(|s| s.as_str()).collect(),
            _ => default_exclude.to_vec(),
        };

        // 4. Include filter: path must match at least one
        if !effective_include.is_empty()
            && !effective_include.iter().any(|pat| glob_matches(pat, path))
        {
            return false;
        }

        // 5. Exclude filter: path must NOT match any
        if effective_exclude.iter().any(|pat| glob_matches(pat, path)) {
            return false;
        }

        true
    }

    /// Get the resolved config for a specific cop.
    pub fn cop_config(&self, name: &str) -> CopConfig {
        self.cop_configs
            .get(name)
            .cloned()
            .unwrap_or_default()
    }

    /// Global exclude patterns from AllCops.Exclude.
    pub fn global_excludes(&self) -> &[String] {
        &self.global_excludes
    }
}

fn parse_cop_config(value: &Value) -> CopConfig {
    let mut config = CopConfig::default();

    if let Value::Mapping(map) = value {
        for (k, v) in map {
            let key = match k.as_str() {
                Some(s) => s,
                None => continue,
            };
            match key {
                "Enabled" => {
                    if let Some(b) = v.as_bool() {
                        config.enabled = b;
                    }
                }
                "Severity" => {
                    if let Some(s) = v.as_str() {
                        config.severity = Severity::from_str(s);
                    }
                }
                "Exclude" => {
                    if let Some(list) = value_to_string_list(v) {
                        config.exclude = list;
                    }
                }
                "Include" => {
                    if let Some(list) = value_to_string_list(v) {
                        config.include = list;
                    }
                }
                _ => {
                    config.options.insert(key.to_string(), v.clone());
                }
            }
        }
    }

    config
}

fn extract_string_list(value: &Value, key: &str) -> Option<Vec<String>> {
    value
        .as_mapping()?
        .get(&Value::String(key.to_string()))?
        .as_sequence()
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
}

fn value_to_string_list(value: &Value) -> Option<Vec<String>> {
    value.as_sequence().map(|seq| {
        seq.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    })
}

/// Match a RuboCop-style glob pattern against a file path.
///
/// Patterns like `db/migrate/**/*.rb` or `**/*_spec.rb` are matched against
/// the path. We try matching against both the full path and just the relative
/// components to handle RuboCop's convention of patterns relative to project root.
fn glob_matches(pattern: &str, path: &Path) -> bool {
    let glob = match GlobBuilder::new(pattern)
        .literal_separator(false)
        .build()
    {
        Ok(g) => g,
        Err(_) => return false,
    };
    let matcher = glob.compile_matcher();
    // Try matching against the path as given
    if matcher.is_match(path) {
        return true;
    }
    // Also try matching against just the path string (handles both relative and absolute)
    let path_str = path.to_string_lossy();
    matcher.is_match(path_str.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn write_config(dir: &Path, content: &str) -> PathBuf {
        let path = dir.join(".rubocop.yml");
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn missing_config_returns_empty() {
        let config = load_config(Some(Path::new("/nonexistent/.rubocop.yml"))).unwrap();
        assert!(config.global_excludes().is_empty());
        assert!(config.is_cop_enabled("Style/Foo", Path::new("a.rb"), &[], &[]));
    }

    #[test]
    fn allcops_exclude() {
        let dir = std::env::temp_dir().join("rblint_test_config_exclude");
        fs::create_dir_all(&dir).unwrap();
        let path = write_config(
            &dir,
            "AllCops:\n  Exclude:\n    - 'vendor/**'\n    - 'tmp/**'\n",
        );
        let config = load_config(Some(&path)).unwrap();
        assert_eq!(
            config.global_excludes(),
            &["vendor/**".to_string(), "tmp/**".to_string()]
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cop_enabled_false() {
        let dir = std::env::temp_dir().join("rblint_test_config_disabled");
        fs::create_dir_all(&dir).unwrap();
        let path = write_config(&dir, "Style/Foo:\n  Enabled: false\n");
        let config = load_config(Some(&path)).unwrap();
        assert!(!config.is_cop_enabled("Style/Foo", Path::new("a.rb"), &[], &[]));
        // Unknown cops default to enabled
        assert!(config.is_cop_enabled("Style/Bar", Path::new("a.rb"), &[], &[]));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cop_severity_override() {
        let dir = std::env::temp_dir().join("rblint_test_config_severity");
        fs::create_dir_all(&dir).unwrap();
        let path = write_config(&dir, "Style/Foo:\n  Severity: error\n");
        let config = load_config(Some(&path)).unwrap();
        let cc = config.cop_config("Style/Foo");
        assert_eq!(cc.severity, Some(Severity::Error));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cop_exclude_include_patterns() {
        let dir = std::env::temp_dir().join("rblint_test_config_patterns");
        fs::create_dir_all(&dir).unwrap();
        let path = write_config(
            &dir,
            "Style/Foo:\n  Exclude:\n    - 'spec/**'\n  Include:\n    - '**/*.rake'\n",
        );
        let config = load_config(Some(&path)).unwrap();
        let cc = config.cop_config("Style/Foo");
        assert_eq!(cc.exclude, vec!["spec/**".to_string()]);
        assert_eq!(cc.include, vec!["**/*.rake".to_string()]);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cop_custom_options() {
        let dir = std::env::temp_dir().join("rblint_test_config_options");
        fs::create_dir_all(&dir).unwrap();
        let path = write_config(&dir, "Layout/LineLength:\n  Max: 120\n");
        let config = load_config(Some(&path)).unwrap();
        let cc = config.cop_config("Layout/LineLength");
        assert_eq!(cc.options.get("Max").and_then(|v| v.as_u64()), Some(120));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn non_cop_keys_ignored() {
        let dir = std::env::temp_dir().join("rblint_test_config_noncop");
        fs::create_dir_all(&dir).unwrap();
        let path = write_config(&dir, "AllCops:\n  Exclude: []\nrequire:\n  - rubocop-rspec\n");
        let config = load_config(Some(&path)).unwrap();
        // "require" has no "/" so should not be treated as a cop
        assert!(config.is_cop_enabled("require", Path::new("a.rb"), &[], &[]));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn default_cop_config() {
        let config = load_config(Some(Path::new("/nonexistent/.rubocop.yml"))).unwrap();
        let cc = config.cop_config("Style/Whatever");
        assert!(cc.enabled);
        assert!(cc.severity.is_none());
        assert!(cc.exclude.is_empty());
        assert!(cc.options.is_empty());
    }

    // ---- Path-based Include/Exclude tests ----

    #[test]
    fn default_include_filters_files() {
        let config = load_config(Some(Path::new("/nonexistent/.rubocop.yml"))).unwrap();
        // With default_include set, only matching files pass
        let inc = &["db/migrate/**/*.rb"];
        assert!(config.is_cop_enabled("Rails/Foo", Path::new("db/migrate/001_create.rb"), inc, &[]));
        assert!(!config.is_cop_enabled("Rails/Foo", Path::new("app/models/user.rb"), inc, &[]));
    }

    #[test]
    fn default_exclude_filters_files() {
        let config = load_config(Some(Path::new("/nonexistent/.rubocop.yml"))).unwrap();
        let exc = &["spec/**/*.rb"];
        assert!(config.is_cop_enabled("Style/Foo", Path::new("app/models/user.rb"), &[], exc));
        assert!(!config.is_cop_enabled("Style/Foo", Path::new("spec/models/user_spec.rb"), &[], exc));
    }

    #[test]
    fn user_include_overrides_default() {
        let dir = std::env::temp_dir().join("rblint_test_config_inc_override");
        fs::create_dir_all(&dir).unwrap();
        let path = write_config(
            &dir,
            "Rails/Migration:\n  Include:\n    - 'db/**/*.rb'\n",
        );
        let config = load_config(Some(&path)).unwrap();
        // Default include is narrower but user config overrides
        let default_inc = &["db/migrate/**/*.rb"];
        assert!(config.is_cop_enabled("Rails/Migration", Path::new("db/seeds.rb"), default_inc, &[]));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn global_excludes_applied() {
        let dir = std::env::temp_dir().join("rblint_test_config_global_exc");
        fs::create_dir_all(&dir).unwrap();
        let path = write_config(&dir, "AllCops:\n  Exclude:\n    - 'vendor/**'\n");
        let config = load_config(Some(&path)).unwrap();
        assert!(!config.is_cop_enabled("Style/Foo", Path::new("vendor/gems/foo.rb"), &[], &[]));
        assert!(config.is_cop_enabled("Style/Foo", Path::new("app/models/user.rb"), &[], &[]));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn glob_matches_basic() {
        assert!(glob_matches("**/*.rb", Path::new("app/models/user.rb")));
        assert!(glob_matches("db/migrate/**/*.rb", Path::new("db/migrate/001_create.rb")));
        assert!(!glob_matches("db/migrate/**/*.rb", Path::new("app/models/user.rb")));
        assert!(glob_matches("spec/**", Path::new("spec/models/user_spec.rb")));
    }
}
