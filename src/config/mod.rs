pub mod gem_path;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use globset::GlobBuilder;
use serde_yml::Value;

use crate::cop::CopConfig;
use crate::diagnostic::Severity;

/// Resolved configuration from .rubocop.yml with full inheritance support.
///
/// Supports `inherit_from` (local YAML files) and `inherit_gem` (via
/// `bundle info --path`). Silently ignores `require:` and `plugins:` keys.
#[derive(Debug)]
pub struct ResolvedConfig {
    /// Per-cop configs keyed by cop name (e.g. "Style/FrozenStringLiteralComment")
    cop_configs: HashMap<String, CopConfig>,
    global_excludes: Vec<String>,
}

impl ResolvedConfig {
    fn empty() -> Self {
        Self {
            cop_configs: HashMap::new(),
            global_excludes: Vec::new(),
        }
    }
}

/// A single parsed config layer (before merging).
#[derive(Debug, Clone)]
struct ConfigLayer {
    cop_configs: HashMap<String, CopConfig>,
    global_excludes: Vec<String>,
}

impl ConfigLayer {
    fn empty() -> Self {
        Self {
            cop_configs: HashMap::new(),
            global_excludes: Vec::new(),
        }
    }
}

/// Load config from the given path, or look for `.rubocop.yml` in the
/// current directory. Returns an empty config if the file doesn't exist.
///
/// Resolves `inherit_from` and `inherit_gem` recursively, merging layers
/// bottom-up with RuboCop-compatible merge rules.
pub fn load_config(path: Option<&Path>) -> Result<ResolvedConfig> {
    let config_path = match path {
        Some(p) => p.to_path_buf(),
        None => Path::new(".rubocop.yml").to_path_buf(),
    };

    if !config_path.exists() {
        return Ok(ResolvedConfig::empty());
    }

    let mut visited = HashSet::new();
    let layer = load_config_recursive(&config_path, &mut visited)?;

    Ok(ResolvedConfig {
        cop_configs: layer.cop_configs,
        global_excludes: layer.global_excludes,
    })
}

/// Recursively load a config file and all its inherited configs.
///
/// `visited` tracks absolute paths to detect circular inheritance.
fn load_config_recursive(
    config_path: &Path,
    visited: &mut HashSet<PathBuf>,
) -> Result<ConfigLayer> {
    let abs_path = if config_path.is_absolute() {
        config_path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .join(config_path)
    };

    // Circular inheritance detection
    if !visited.insert(abs_path.clone()) {
        anyhow::bail!(
            "Circular config inheritance detected: {}",
            abs_path.display()
        );
    }

    let contents = std::fs::read_to_string(config_path)
        .with_context(|| format!("failed to read config {}", config_path.display()))?;
    let raw: Value = serde_yml::from_str(&contents)
        .with_context(|| format!("failed to parse {}", config_path.display()))?;

    let config_dir = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    // Collect inherited layers (in order: inherit_gem first, then inherit_from)
    let mut base_layer = ConfigLayer::empty();

    if let Value::Mapping(ref map) = raw {
        // 1. Process inherit_gem
        if let Some(gem_value) = map.get(&Value::String("inherit_gem".to_string())) {
            if let Value::Mapping(gem_map) = gem_value {
                for (gem_key, gem_paths) in gem_map {
                    if let Some(gem_name) = gem_key.as_str() {
                        let gem_layers =
                            resolve_inherit_gem(gem_name, gem_paths, visited);
                        for layer in gem_layers {
                            merge_layer_into(&mut base_layer, &layer);
                        }
                    }
                }
            }
        }

        // 2. Process inherit_from
        if let Some(inherit_value) = map.get(&Value::String("inherit_from".to_string())) {
            let paths = match inherit_value {
                Value::String(s) => vec![s.clone()],
                Value::Sequence(seq) => seq
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect(),
                _ => vec![],
            };

            for rel_path in &paths {
                let inherited_path = config_dir.join(rel_path);
                if !inherited_path.exists() {
                    eprintln!(
                        "warning: inherit_from target not found: {} (from {})",
                        inherited_path.display(),
                        config_path.display()
                    );
                    continue;
                }
                match load_config_recursive(&inherited_path, visited) {
                    Ok(layer) => merge_layer_into(&mut base_layer, &layer),
                    Err(e) => {
                        // Circular inheritance errors should propagate
                        if format!("{e:#}").contains("Circular config inheritance") {
                            return Err(e);
                        }
                        eprintln!(
                            "warning: failed to load inherited config {}: {e:#}",
                            inherited_path.display()
                        );
                    }
                }
            }
        }
    }

    // 3. Parse the local config layer and merge it on top
    let local_layer = parse_config_layer(&raw);
    merge_layer_into(&mut base_layer, &local_layer);

    Ok(base_layer)
}

/// Resolve `inherit_gem` entries. Each gem name maps to one or more YAML paths
/// relative to the gem's root directory.
fn resolve_inherit_gem(
    gem_name: &str,
    paths_value: &Value,
    visited: &mut HashSet<PathBuf>,
) -> Vec<ConfigLayer> {
    let gem_root = match gem_path::resolve_gem_path(gem_name) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("warning: {e:#}");
            return vec![];
        }
    };

    let rel_paths: Vec<String> = match paths_value {
        Value::String(s) => vec![s.clone()],
        Value::Sequence(seq) => seq
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => vec![],
    };

    let mut layers = Vec::new();
    for rel_path in &rel_paths {
        let full_path = gem_root.join(rel_path);
        if !full_path.exists() {
            eprintln!(
                "warning: inherit_gem config not found: {} (gem {})",
                full_path.display(),
                gem_name
            );
            continue;
        }
        match load_config_recursive(&full_path, visited) {
            Ok(layer) => layers.push(layer),
            Err(e) => {
                eprintln!(
                    "warning: failed to load gem config {}: {e:#}",
                    full_path.display()
                );
            }
        }
    }
    layers
}

/// Parse a single YAML Value into a ConfigLayer (no inheritance resolution).
fn parse_config_layer(raw: &Value) -> ConfigLayer {
    let mut cop_configs = HashMap::new();
    let mut global_excludes = Vec::new();

    if let Value::Mapping(map) = raw {
        for (key, value) in map {
            let key_str = match key.as_str() {
                Some(s) => s,
                None => continue,
            };

            // Skip non-cop top-level keys silently
            match key_str {
                "inherit_from" | "inherit_gem" | "require" | "plugins" => continue,
                "AllCops" => {
                    if let Some(excludes) = extract_string_list(value, "Exclude") {
                        global_excludes = excludes;
                    }
                    continue;
                }
                _ => {}
            }

            // Cop names contain "/" (e.g. "Style/FrozenStringLiteralComment")
            if key_str.contains('/') {
                let cop_config = parse_cop_config(value);
                cop_configs.insert(key_str.to_string(), cop_config);
            }
        }
    }

    ConfigLayer {
        cop_configs,
        global_excludes,
    }
}

/// Merge an overlay layer into a base layer using RuboCop merge rules:
/// - Scalar values (Enabled, Severity, options): last writer wins
/// - Exclude arrays: appended (union)
/// - Include arrays: replaced (override)
/// - Global excludes: appended
fn merge_layer_into(base: &mut ConfigLayer, overlay: &ConfigLayer) {
    // Merge global excludes: append
    for exc in &overlay.global_excludes {
        if !base.global_excludes.contains(exc) {
            base.global_excludes.push(exc.clone());
        }
    }

    // Merge per-cop configs
    for (cop_name, overlay_config) in &overlay.cop_configs {
        match base.cop_configs.get_mut(cop_name) {
            Some(base_config) => {
                merge_cop_config(base_config, overlay_config);
            }
            None => {
                base.cop_configs
                    .insert(cop_name.clone(), overlay_config.clone());
            }
        }
    }
}

/// Merge a single cop's overlay config into its base config.
fn merge_cop_config(base: &mut CopConfig, overlay: &CopConfig) {
    // Enabled: last writer wins (only if overlay explicitly set it —
    // we can't distinguish "default true" from "explicitly set true" in CopConfig,
    // so we always apply the overlay's value)
    base.enabled = overlay.enabled;

    // Severity: last writer wins (if overlay has one)
    if overlay.severity.is_some() {
        base.severity = overlay.severity;
    }

    // Exclude: append (union)
    for exc in &overlay.exclude {
        if !base.exclude.contains(exc) {
            base.exclude.push(exc.clone());
        }
    }

    // Include: replace (override) if overlay has any
    if !overlay.include.is_empty() {
        base.include.clone_from(&overlay.include);
    }

    // Options: merge (last writer wins per key)
    for (key, value) in &overlay.options {
        base.options.insert(key.clone(), value.clone());
    }
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

    /// Return all cop names from the config that are explicitly set to Enabled: true,
    /// plus cop names that appear in the config without Enabled: false.
    /// This is used by --rubocop-only to determine which cops the project uses.
    pub fn enabled_cop_names(&self) -> Vec<String> {
        self.cop_configs
            .iter()
            .filter(|(_, config)| config.enabled)
            .map(|(name, _)| name.clone())
            .collect()
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

    fn write_yaml(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
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

    // ---- Inheritance tests ----

    #[test]
    fn inherit_from_single_file() {
        let dir = std::env::temp_dir().join("rblint_test_inherit_single");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        write_yaml(
            &dir,
            "base.yml",
            "Layout/LineLength:\n  Max: 100\nStyle/Foo:\n  Enabled: true\n",
        );
        let path = write_yaml(
            &dir,
            ".rubocop.yml",
            "inherit_from: base.yml\nLayout/LineLength:\n  Max: 120\n",
        );

        let config = load_config(Some(&path)).unwrap();
        // Child overrides base's Max
        let cc = config.cop_config("Layout/LineLength");
        assert_eq!(cc.options.get("Max").and_then(|v| v.as_u64()), Some(120));
        // Base's Style/Foo is still present
        assert!(config.is_cop_enabled("Style/Foo", Path::new("a.rb"), &[], &[]));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn inherit_from_array() {
        let dir = std::env::temp_dir().join("rblint_test_inherit_array");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        write_yaml(
            &dir,
            "base1.yml",
            "AllCops:\n  Exclude:\n    - 'vendor/**'\nStyle/Foo:\n  Enabled: false\n",
        );
        write_yaml(
            &dir,
            "base2.yml",
            "AllCops:\n  Exclude:\n    - 'tmp/**'\nStyle/Foo:\n  Enabled: true\n",
        );
        let path = write_yaml(
            &dir,
            ".rubocop.yml",
            "inherit_from:\n  - base1.yml\n  - base2.yml\n",
        );

        let config = load_config(Some(&path)).unwrap();
        // Global excludes are appended from both bases
        assert!(config.global_excludes().contains(&"vendor/**".to_string()));
        assert!(config.global_excludes().contains(&"tmp/**".to_string()));
        // Style/Foo: base2 overrides base1 (last writer wins)
        assert!(config.is_cop_enabled("Style/Foo", Path::new("a.rb"), &[], &[]));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn inherit_from_child_overrides_base() {
        let dir = std::env::temp_dir().join("rblint_test_inherit_override");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        write_yaml(&dir, "base.yml", "Style/Foo:\n  Enabled: true\n");
        let path = write_yaml(
            &dir,
            ".rubocop.yml",
            "inherit_from: base.yml\nStyle/Foo:\n  Enabled: false\n",
        );

        let config = load_config(Some(&path)).unwrap();
        assert!(!config.is_cop_enabled("Style/Foo", Path::new("a.rb"), &[], &[]));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn inherit_from_exclude_appends() {
        let dir = std::env::temp_dir().join("rblint_test_inherit_exclude");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        write_yaml(
            &dir,
            "base.yml",
            "Style/Foo:\n  Exclude:\n    - 'vendor/**'\n",
        );
        let path = write_yaml(
            &dir,
            ".rubocop.yml",
            "inherit_from: base.yml\nStyle/Foo:\n  Exclude:\n    - 'tmp/**'\n",
        );

        let config = load_config(Some(&path)).unwrap();
        let cc = config.cop_config("Style/Foo");
        assert!(cc.exclude.contains(&"vendor/**".to_string()));
        assert!(cc.exclude.contains(&"tmp/**".to_string()));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn inherit_from_include_replaces() {
        let dir = std::env::temp_dir().join("rblint_test_inherit_include");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        write_yaml(
            &dir,
            "base.yml",
            "Style/Foo:\n  Include:\n    - '**/*.rb'\n",
        );
        let path = write_yaml(
            &dir,
            ".rubocop.yml",
            "inherit_from: base.yml\nStyle/Foo:\n  Include:\n    - 'app/**'\n",
        );

        let config = load_config(Some(&path)).unwrap();
        let cc = config.cop_config("Style/Foo");
        // Include is replaced, not appended
        assert_eq!(cc.include, vec!["app/**".to_string()]);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn inherit_from_missing_warns_but_succeeds() {
        let dir = std::env::temp_dir().join("rblint_test_inherit_missing");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let path = write_yaml(
            &dir,
            ".rubocop.yml",
            "inherit_from: nonexistent.yml\nStyle/Foo:\n  Enabled: false\n",
        );

        // Should succeed (prints a warning to stderr)
        let config = load_config(Some(&path)).unwrap();
        assert!(!config.is_cop_enabled("Style/Foo", Path::new("a.rb"), &[], &[]));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn circular_inherit_from_detected() {
        let dir = std::env::temp_dir().join("rblint_test_inherit_circular");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        write_yaml(&dir, "a.yml", "inherit_from: b.yml\n");
        write_yaml(&dir, "b.yml", "inherit_from: a.yml\n");

        let path = dir.join("a.yml");
        let result = load_config(Some(&path));
        assert!(result.is_err());
        let err_msg = format!("{:#}", result.unwrap_err());
        assert!(
            err_msg.contains("Circular config inheritance"),
            "Expected circular inheritance error, got: {err_msg}"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn require_key_silently_ignored() {
        let dir = std::env::temp_dir().join("rblint_test_require_ignored");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let path = write_yaml(
            &dir,
            ".rubocop.yml",
            "require:\n  - rubocop-rspec\n  - rubocop-rails\nplugins:\n  - rubocop-performance\nStyle/Foo:\n  Enabled: false\n",
        );

        let config = load_config(Some(&path)).unwrap();
        assert!(!config.is_cop_enabled("Style/Foo", Path::new("a.rb"), &[], &[]));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn deep_merge_cop_options() {
        let dir = std::env::temp_dir().join("rblint_test_deep_merge_opts");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        write_yaml(
            &dir,
            "base.yml",
            "Style/Foo:\n  Max: 100\n  EnforcedStyle: compact\n",
        );
        let path = write_yaml(
            &dir,
            ".rubocop.yml",
            "inherit_from: base.yml\nStyle/Foo:\n  Max: 120\n",
        );

        let config = load_config(Some(&path)).unwrap();
        let cc = config.cop_config("Style/Foo");
        // Max overridden by child
        assert_eq!(cc.options.get("Max").and_then(|v| v.as_u64()), Some(120));
        // EnforcedStyle preserved from base
        assert_eq!(
            cc.options.get("EnforcedStyle").and_then(|v| v.as_str()),
            Some("compact")
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn enabled_cop_names_returns_enabled_only() {
        let dir = std::env::temp_dir().join("rblint_test_enabled_names");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let path = write_yaml(
            &dir,
            ".rubocop.yml",
            "Style/Foo:\n  Enabled: true\nStyle/Bar:\n  Enabled: false\nLint/Baz:\n  Max: 10\n",
        );

        let config = load_config(Some(&path)).unwrap();
        let names = config.enabled_cop_names();
        assert!(names.contains(&"Style/Foo".to_string()));
        assert!(!names.contains(&"Style/Bar".to_string()));
        // Lint/Baz has no explicit Enabled, defaults to true
        assert!(names.contains(&"Lint/Baz".to_string()));

        fs::remove_dir_all(&dir).ok();
    }

    // ---- Merge logic unit tests ----

    #[test]
    fn merge_layer_scalars_last_writer_wins() {
        let mut base = ConfigLayer::empty();
        base.cop_configs.insert(
            "Style/Foo".to_string(),
            CopConfig {
                enabled: true,
                ..CopConfig::default()
            },
        );

        let mut overlay = ConfigLayer::empty();
        overlay.cop_configs.insert(
            "Style/Foo".to_string(),
            CopConfig {
                enabled: false,
                ..CopConfig::default()
            },
        );

        merge_layer_into(&mut base, &overlay);
        assert!(!base.cop_configs["Style/Foo"].enabled);
    }

    #[test]
    fn merge_layer_global_excludes_appended() {
        let mut base = ConfigLayer {
            cop_configs: HashMap::new(),
            global_excludes: vec!["vendor/**".to_string()],
        };
        let overlay = ConfigLayer {
            cop_configs: HashMap::new(),
            global_excludes: vec!["tmp/**".to_string()],
        };
        merge_layer_into(&mut base, &overlay);
        assert_eq!(base.global_excludes.len(), 2);
        assert!(base.global_excludes.contains(&"vendor/**".to_string()));
        assert!(base.global_excludes.contains(&"tmp/**".to_string()));
    }

    #[test]
    fn merge_layer_no_duplicate_excludes() {
        let mut base = ConfigLayer {
            cop_configs: HashMap::new(),
            global_excludes: vec!["vendor/**".to_string()],
        };
        let overlay = ConfigLayer {
            cop_configs: HashMap::new(),
            global_excludes: vec!["vendor/**".to_string()],
        };
        merge_layer_into(&mut base, &overlay);
        assert_eq!(base.global_excludes.len(), 1);
    }
}
