use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
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
    pub fn is_cop_enabled(&self, name: &str, _path: &Path) -> bool {
        match self.cop_configs.get(name) {
            Some(config) => config.enabled,
            None => true, // enabled by default
        }
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
        assert!(config.is_cop_enabled("Style/Foo", Path::new("a.rb")));
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
        assert!(!config.is_cop_enabled("Style/Foo", Path::new("a.rb")));
        // Unknown cops default to enabled
        assert!(config.is_cop_enabled("Style/Bar", Path::new("a.rb")));
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
        assert!(config.is_cop_enabled("require", Path::new("a.rb")));
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
}
