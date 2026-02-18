pub mod gem_path;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use serde_yml::Value;

use crate::cop::registry::CopRegistry;
use crate::cop::{CopConfig, EnabledState};
use crate::diagnostic::Severity;

/// Policy for handling `Enabled: pending` cops, controlled by `AllCops.NewCops`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewCopsPolicy {
    Enable,
    Disable,
}

/// Department-level configuration (e.g., `RSpec:`, `Rails:`).
///
/// Plugin default configs use bare department keys to set Include/Exclude
/// patterns and Enabled state for all cops in that department.
#[derive(Debug, Clone, Default)]
struct DepartmentConfig {
    enabled: EnabledState,
    include: Vec<String>,
    exclude: Vec<String>,
}

/// Controls how arrays are merged during config inheritance.
///
/// By default, Exclude arrays are appended and Include arrays are replaced.
/// `inherit_mode` lets configs override this per-key.
#[derive(Debug, Clone, Default)]
struct InheritMode {
    /// Keys whose arrays should be appended (merged) instead of replaced.
    merge: HashSet<String>,
    /// Keys whose arrays should be replaced instead of appended.
    override_keys: HashSet<String>,
}

/// Pre-compiled glob filter for a single cop.
///
/// Built once at startup from resolved config + cop defaults. Avoids
/// recompiling glob patterns on every `is_cop_enabled` call.
pub struct CopFilter {
    enabled: bool,
    include_set: Option<GlobSet>, // None = match all files
    exclude_set: Option<GlobSet>, // None = exclude no files
}

impl CopFilter {
    /// Returns true if the cop is enabled in config (Enabled: true).
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Check whether this cop should run on the given file path.
    pub fn is_match(&self, path: &Path) -> bool {
        if !self.enabled {
            return false;
        }
        if let Some(ref inc) = self.include_set {
            if !inc.is_match(path) {
                return false;
            }
        }
        if let Some(ref exc) = self.exclude_set {
            if exc.is_match(path) {
                return false;
            }
        }
        true
    }

    /// Check whether the given path matches this cop's Include patterns.
    fn is_included(&self, path: &Path) -> bool {
        match self.include_set {
            Some(ref inc) => inc.is_match(path),
            None => true, // no Include = match all
        }
    }

    /// Check whether the given path matches this cop's Exclude patterns.
    fn is_excluded(&self, path: &Path) -> bool {
        match self.exclude_set {
            Some(ref exc) => exc.is_match(path),
            None => false, // no Exclude = exclude nothing
        }
    }
}

/// Pre-compiled filter set for all cops + global excludes.
///
/// Built once from `ResolvedConfig` + `CopRegistry`, then shared across
/// all rayon worker threads. Eliminates per-file glob compilation overhead.
pub struct CopFilterSet {
    global_exclude: GlobSet,
    filters: Vec<CopFilter>, // indexed by cop position in registry
    /// Config directory for relativizing file paths before glob matching.
    /// Cop Include/Exclude patterns are relative to the project root
    /// (where `.rubocop.yml` lives), but file paths may include a prefix
    /// when running from outside the project root.
    config_dir: Option<PathBuf>,
    /// Sub-directories containing their own `.rubocop.yml` files.
    /// Sorted deepest-first so `nearest_config_dir` finds the most specific match.
    /// RuboCop resolves Include/Exclude patterns relative to the nearest config
    /// directory, so files in `db/migrate/` with a local `.rubocop.yml` have their
    /// paths relativized to `db/migrate/` rather than the project root.
    sub_config_dirs: Vec<PathBuf>,
}

impl CopFilterSet {
    /// Check whether a file is globally excluded (AllCops.Exclude).
    pub fn is_globally_excluded(&self, path: &Path) -> bool {
        if self.global_exclude.is_match(path) {
            return true;
        }
        // Also try matching against the path relativized to config_dir.
        // This handles running from outside the project root, where file
        // paths include a prefix (e.g., `bench/repos/mastodon/vendor/foo.rb`
        // needs to match a pattern like `vendor/**`).
        if let Some(ref cd) = self.config_dir {
            if let Ok(rel) = path.strip_prefix(cd) {
                return self.global_exclude.is_match(rel);
            }
        }
        false
    }

    /// Get the pre-compiled filter for a cop by its registry index.
    pub fn cop_filter(&self, index: usize) -> &CopFilter {
        &self.filters[index]
    }

    /// Find the nearest sub-config directory for a file path.
    /// Returns the deepest `.rubocop.yml` directory that is an ancestor of `path`,
    /// falling back to the root `config_dir`.
    fn nearest_config_dir(&self, path: &Path) -> Option<&Path> {
        // sub_config_dirs is sorted deepest-first, so the first match is most specific
        for dir in &self.sub_config_dirs {
            if path.starts_with(dir) {
                return Some(dir.as_path());
            }
        }
        self.config_dir.as_deref()
    }

    /// Check whether a cop (by registry index) should run on the given file.
    /// Checks both the original path and the path relativized to the nearest
    /// config directory (supports per-directory `.rubocop.yml` path relativity):
    /// - Include: matches if EITHER path matches (supports absolute + relative patterns)
    /// - Exclude: matches if EITHER path matches (catches project-relative patterns)
    pub fn is_cop_match(&self, index: usize, path: &Path) -> bool {
        let filter = &self.filters[index];
        if !filter.enabled {
            return false;
        }

        let rel_path = self
            .nearest_config_dir(path)
            .and_then(|cd| path.strip_prefix(cd).ok());

        // Include: file must match on at least one path form.
        // This supports both absolute patterns (/tmp/test/db/**) and
        // relative patterns (db/migrate/**).
        let included = filter.is_included(path)
            || rel_path.is_some_and(|rel| filter.is_included(rel));
        if !included {
            return false;
        }

        // Exclude: file is excluded if EITHER path form matches.
        // This catches project-relative Exclude patterns (lib/tasks/*.rake)
        // even when the file path has a prefix (bench/repos/mastodon/...).
        let excluded = filter.is_excluded(path)
            || rel_path.is_some_and(|rel| filter.is_excluded(rel));
        if excluded {
            return false;
        }

        true
    }

    /// Check whether a file path would be matched (not excluded) by a cop's
    /// Include/Exclude patterns from its `CopConfig`. This is used for cops
    /// NOT in the registry (unimplemented cops from gem configs) to determine
    /// if a `rubocop:disable` directive is redundant.
    ///
    /// Returns true if the file WOULD be matched (cop would run on it),
    /// false if the file is excluded by Include/Exclude patterns.
    pub fn is_path_matched_by_cop_config(&self, cop_config: &CopConfig, path: &Path) -> bool {
        let include_pats: Vec<&str> = cop_config.include.iter().map(|s| s.as_str()).collect();
        let exclude_pats: Vec<&str> = cop_config.exclude.iter().map(|s| s.as_str()).collect();
        let include_set = build_glob_set(&include_pats);
        let exclude_set = build_glob_set(&exclude_pats);

        let rel_path = self
            .nearest_config_dir(path)
            .and_then(|cd| path.strip_prefix(cd).ok());

        // Include check: if patterns exist, file must match at least one form
        if let Some(ref inc) = include_set {
            let included =
                inc.is_match(path) || rel_path.is_some_and(|rel| inc.is_match(rel));
            if !included {
                return false;
            }
        }

        // Exclude check: file is excluded if either path form matches
        if let Some(ref exc) = exclude_set {
            let excluded =
                exc.is_match(path) || rel_path.is_some_and(|rel| exc.is_match(rel));
            if excluded {
                return false;
            }
        }

        true
    }
}

/// Walk the project tree and find directories containing `.rubocop.yml` files
/// (excluding the root). Returns directories sorted deepest-first so that
/// `nearest_config_dir` finds the most specific match first.
fn discover_sub_config_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let walker = ignore::WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .build();

    for entry in walker.flatten() {
        if entry.file_type().is_some_and(|ft| ft.is_file())
            && entry.file_name() == ".rubocop.yml"
        {
            if let Some(parent) = entry.path().parent() {
                // Skip the root directory itself
                if parent != root {
                    dirs.push(parent.to_path_buf());
                }
            }
        }
    }

    // Sort deepest-first: longer paths first
    dirs.sort_by(|a, b| b.as_os_str().len().cmp(&a.as_os_str().len()));
    dirs
}

/// Load per-directory cop config overrides from nested `.rubocop.yml` files.
///
/// For each subdirectory containing a `.rubocop.yml`, parses the local cop
/// settings (ignoring `inherit_from` since it typically points back to the root).
/// Returns a list of (directory, cop_configs) pairs sorted deepest-first.
fn load_dir_overrides(root: &Path) -> Vec<(PathBuf, HashMap<String, CopConfig>)> {
    let sub_dirs = discover_sub_config_dirs(root);
    let mut overrides = Vec::new();

    for dir in sub_dirs {
        let config_path = dir.join(".rubocop.yml");
        let contents = match std::fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let contents = contents.replace("!ruby/regexp ", "");
        let raw: Value = match serde_yml::from_str(&contents) {
            Ok(v) => v,
            Err(e) => {
                eprintln!(
                    "warning: failed to parse nested config {}: {e}",
                    config_path.display()
                );
                continue;
            }
        };

        // Parse only the local cop-level settings (keys containing '/').
        // We skip inherit_from, AllCops, require, etc. — those are handled
        // by the root config. We only want the cop-specific overrides.
        let layer = parse_config_layer(&raw);
        if !layer.cop_configs.is_empty() {
            overrides.push((dir, layer.cop_configs));
        }
    }

    overrides
}

/// Build a `GlobSet` from a list of pattern strings.
/// Returns `None` if the list is empty.
fn build_glob_set(patterns: &[&str]) -> Option<GlobSet> {
    if patterns.is_empty() {
        return None;
    }
    let mut builder = GlobSetBuilder::new();
    for pat in patterns {
        if let Ok(glob) = GlobBuilder::new(pat).literal_separator(false).build() {
            builder.add(glob);
        }
    }
    builder.build().ok()
}

/// Resolved configuration from .rubocop.yml with full inheritance support.
///
/// Supports `inherit_from` (local YAML files), `inherit_gem` (via
/// `bundle info --path`), `require:` (plugin default configs), department-level
/// configs, `Enabled: pending` / `AllCops.NewCops`, `AllCops.DisabledByDefault`,
/// and `inherit_mode`.
#[derive(Debug)]
pub struct ResolvedConfig {
    /// Per-cop configs keyed by cop name (e.g. "Style/FrozenStringLiteralComment")
    cop_configs: HashMap<String, CopConfig>,
    /// Department-level configs keyed by department name (e.g. "RSpec", "Rails")
    department_configs: HashMap<String, DepartmentConfig>,
    global_excludes: Vec<String>,
    /// Directory containing the resolved config file (for relative path resolution).
    config_dir: Option<PathBuf>,
    /// How to handle `Enabled: pending` cops.
    new_cops: NewCopsPolicy,
    /// When true, cops without explicit `Enabled: true` are disabled.
    disabled_by_default: bool,
    /// All cop names mentioned in `require:` gem default configs.
    /// Cops from plugin departments not in this set are treated as non-existent
    /// (the installed gem version doesn't include them).
    require_known_cops: HashSet<String>,
    /// Department names that had gems loaded via `require:`.
    require_departments: HashSet<String>,
    /// Target Ruby version from AllCops.TargetRubyVersion (e.g. 3.1, 3.2).
    /// None means not specified (cops should default to 2.7 per RuboCop convention).
    target_ruby_version: Option<f64>,
    /// Target Rails version from AllCops.TargetRailsVersion (e.g. 7.1, 8.0).
    /// None means not specified (cops should default to 5.0 per RuboCop convention).
    target_rails_version: Option<f64>,
    /// Whether ActiveSupport extensions are enabled (AllCops.ActiveSupportExtensionsEnabled).
    /// Set to true by rubocop-rails. Affects cops like Style/CollectionQuerying.
    active_support_extensions_enabled: bool,
    /// All cop names found in the installed rubocop gem's config/default.yml.
    /// When non-empty, core cops (Layout, Lint, Style, etc.) not in this set
    /// are treated as non-existent in the project's rubocop version.
    rubocop_known_cops: HashSet<String>,
    /// Cops explicitly set to Enabled:true by user config (not from rubocop defaults
    /// or require:/plugins: gem defaults). Used to determine whether a department-level
    /// Enabled:false should override a cop-level Enabled:true.
    user_enabled_cops: HashSet<String>,
    /// Per-directory cop config overrides from nested `.rubocop.yml` files.
    /// Keyed by directory path (sorted deepest-first for lookup).
    /// Each value contains only the cop-specific options from that directory's config.
    dir_overrides: Vec<(PathBuf, HashMap<String, CopConfig>)>,
}

impl ResolvedConfig {
    fn empty() -> Self {
        Self {
            cop_configs: HashMap::new(),
            department_configs: HashMap::new(),
            global_excludes: Vec::new(),
            config_dir: None,
            new_cops: NewCopsPolicy::Disable,
            disabled_by_default: false,
            require_known_cops: HashSet::new(),
            require_departments: HashSet::new(),
            target_ruby_version: None,
            target_rails_version: None,
            active_support_extensions_enabled: false,
            rubocop_known_cops: HashSet::new(),
            user_enabled_cops: HashSet::new(),
            dir_overrides: Vec::new(),
        }
    }
}

/// A single parsed config layer (before merging).
#[derive(Debug, Clone)]
struct ConfigLayer {
    cop_configs: HashMap<String, CopConfig>,
    department_configs: HashMap<String, DepartmentConfig>,
    global_excludes: Vec<String>,
    new_cops: Option<String>,
    disabled_by_default: Option<bool>,
    inherit_mode: InheritMode,
    /// Cop names where Enabled:true came from `require:` gem defaults
    /// (used to distinguish user-explicit enables from gem defaults under DisabledByDefault).
    require_enabled_cops: HashSet<String>,
    /// Department names where Enabled:true came from `require:` gem defaults.
    require_enabled_depts: HashSet<String>,
    /// ALL cop names mentioned in `require:` gem default configs (regardless of enabled state).
    /// Used to determine which cops exist in the installed gem version.
    require_known_cops: HashSet<String>,
    /// Department names that had gems loaded via `require:`.
    require_departments: HashSet<String>,
    /// Target Ruby version from AllCops.TargetRubyVersion.
    target_ruby_version: Option<f64>,
    /// Target Rails version from AllCops.TargetRailsVersion.
    target_rails_version: Option<f64>,
    /// AllCops.ActiveSupportExtensionsEnabled (set by rubocop-rails).
    active_support_extensions_enabled: Option<bool>,
}

impl ConfigLayer {
    fn empty() -> Self {
        Self {
            cop_configs: HashMap::new(),
            department_configs: HashMap::new(),
            global_excludes: Vec::new(),
            new_cops: None,
            disabled_by_default: None,
            inherit_mode: InheritMode::default(),
            require_enabled_cops: HashSet::new(),
            require_enabled_depts: HashSet::new(),
            require_known_cops: HashSet::new(),
            require_departments: HashSet::new(),
            target_ruby_version: None,
            target_rails_version: None,
            active_support_extensions_enabled: None,
        }
    }
}

/// Walk up from `start_dir` to find `.rubocop.yml`.
///
/// First tries with the original path (preserving relative paths), then falls
/// back to canonicalized path to handle symlinks and `..` components.
fn find_config(start_dir: &Path) -> Option<PathBuf> {
    // Try with original path first to preserve relative paths.
    // This ensures config_dir stays relative when input is relative,
    // matching file paths from discover_files for glob matching.
    let mut dir = start_dir.to_path_buf();
    loop {
        let candidate = dir.join(".rubocop.yml");
        if candidate.exists() {
            return Some(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    // Fallback: try with canonicalized path (resolves symlinks, ..)
    if let Ok(canonical) = std::fs::canonicalize(start_dir) {
        if canonical != start_dir {
            let mut dir = canonical;
            loop {
                let candidate = dir.join(".rubocop.yml");
                if candidate.exists() {
                    return Some(candidate);
                }
                if !dir.pop() {
                    break;
                }
            }
        }
    }
    None
}

/// Load config from the given path, or auto-discover `.rubocop.yml` by walking
/// up from `target_dir`. Returns an empty config if no config file is found.
///
/// Resolves `inherit_from`, `inherit_gem`, and `require:` recursively, merging
/// layers bottom-up with RuboCop-compatible merge rules.
pub fn load_config(path: Option<&Path>, target_dir: Option<&Path>) -> Result<ResolvedConfig> {
    let config_path = match path {
        Some(p) => {
            if p.exists() {
                Some(p.to_path_buf())
            } else {
                return Ok(ResolvedConfig::empty());
            }
        }
        None => {
            let start = target_dir
                .map(|p| {
                    if p.is_file() {
                        p.parent().unwrap_or(p).to_path_buf()
                    } else {
                        p.to_path_buf()
                    }
                })
                .or_else(|| std::env::current_dir().ok());
            match start {
                Some(dir) => find_config(&dir),
                None => None,
            }
        }
    };

    let config_path = match config_path {
        Some(p) => p,
        None => return Ok(ResolvedConfig::empty()),
    };

    let config_dir = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    // Load rubocop's own config/default.yml as the lowest-priority base layer.
    // This provides correct default Enabled states, EnforcedStyle values, etc.
    // Also collect the set of known cops for version awareness.
    let (mut base, rubocop_known_cops) = try_load_rubocop_defaults(&config_dir);

    let mut visited = HashSet::new();
    let project_layer = load_config_recursive(&config_path, &config_dir, &mut visited)?;

    // Compute the set of cops explicitly enabled by user config (not from defaults).
    // This includes cops set to Enabled:true in inherit_from, inherit_gem, or local
    // config, but NOT cops enabled by rubocop defaults or require:/plugins: gem defaults.
    let user_enabled_cops: HashSet<String> = project_layer
        .cop_configs
        .iter()
        .filter(|(name, c)| {
            c.enabled == EnabledState::True && !project_layer.require_enabled_cops.contains(*name)
        })
        .map(|(name, _)| name.clone())
        .collect();

    // Merge project config on top of rubocop defaults
    merge_layer_into(&mut base, &project_layer, None);

    let disabled_by_default = base.disabled_by_default.unwrap_or(false);

    // When DisabledByDefault is true, only cops that are *explicitly* enabled
    // in user config files (inherit_gem, inherit_from, local settings) should
    // remain enabled. Cops enabled only by gem defaults (rubocop's own defaults
    // or `require:` plugin defaults) should be treated as Unset (= disabled).
    //
    // We check the project_layer, but since `require:` defaults are merged into
    // it, we must also distinguish. The project_layer's `require_enabled_cops`
    // field tracks cops set to Enabled:true specifically by `require:` defaults,
    // which we exclude.
    if disabled_by_default {
        // Reset department-level enabled states that came from require: defaults
        for (dept_name, dept_cfg) in base.department_configs.iter_mut() {
            if dept_cfg.enabled == EnabledState::True {
                let user_enabled = project_layer
                    .department_configs
                    .get(dept_name)
                    .is_some_and(|dc| dc.enabled == EnabledState::True)
                    && !project_layer.require_enabled_depts.contains(dept_name.as_str());
                if !user_enabled {
                    dept_cfg.enabled = EnabledState::Unset;
                }
            }
        }

        // Reset cop-level enabled states that came from defaults (not user config)
        for (cop_name, cop_cfg) in base.cop_configs.iter_mut() {
            if cop_cfg.enabled == EnabledState::True {
                let user_enabled = project_layer
                    .cop_configs
                    .get(cop_name)
                    .is_some_and(|c| c.enabled == EnabledState::True)
                    && !project_layer.require_enabled_cops.contains(cop_name);
                let dept = cop_name.split('/').next().unwrap_or("");
                let dept_enabled = project_layer
                    .department_configs
                    .get(dept)
                    .is_some_and(|dc| dc.enabled == EnabledState::True)
                    && !project_layer.require_enabled_depts.contains(dept);
                if !user_enabled && !dept_enabled {
                    cop_cfg.enabled = EnabledState::Unset;
                }
            }
        }
    }

    // Fall back to .ruby-version file if TargetRubyVersion wasn't set in config.
    // RuboCop reads this file to determine the target Ruby version.
    let target_ruby_version = base.target_ruby_version.or_else(|| {
        let ruby_version_path = config_dir.join(".ruby-version");
        if let Ok(content) = std::fs::read_to_string(&ruby_version_path) {
            let trimmed = content.trim();
            // Parse version like "3.4.4" -> 3.4
            let parts: Vec<&str> = trimmed.split('.').collect();
            if parts.len() >= 2 {
                if let (Ok(major), Ok(minor)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>()) {
                    return Some(major as f64 + minor as f64 / 10.0);
                }
            }
        }
        None
    });

    // Fall back to Gemfile.lock if TargetRailsVersion wasn't set in config.
    // RuboCop looks for the 'railties' gem in the lockfile.
    let target_rails_version = base.target_rails_version.or_else(|| {
        for lock_name in &["Gemfile.lock", "gems.locked"] {
            let lock_path = config_dir.join(lock_name);
            if let Ok(content) = std::fs::read_to_string(&lock_path) {
                if let Some(ver) = parse_gem_version_from_lockfile(&content, "railties") {
                    return Some(ver);
                }
            }
        }
        None
    });

    // Discover and parse nested .rubocop.yml files for per-directory cop overrides.
    // These provide cop-specific option overrides for files in subdirectories
    // (e.g., db/migrate/.rubocop.yml setting CheckSymbols: false for Naming/VariableNumber).
    let dir_overrides = load_dir_overrides(&config_dir);

    Ok(ResolvedConfig {
        cop_configs: base.cop_configs,
        department_configs: base.department_configs,
        global_excludes: base.global_excludes,
        config_dir: Some(config_dir),
        new_cops: match base.new_cops.as_deref() {
            Some("enable") => NewCopsPolicy::Enable,
            _ => NewCopsPolicy::Disable,
        },
        disabled_by_default,
        require_known_cops: base.require_known_cops,
        require_departments: base.require_departments,
        target_ruby_version,
        target_rails_version,
        active_support_extensions_enabled: base.active_support_extensions_enabled.unwrap_or(false),
        rubocop_known_cops,
        user_enabled_cops,
        dir_overrides,
    })
}

/// Try to load rubocop's own `config/default.yml` as the base config layer.
///
/// This provides correct default Enabled states (52 cops disabled by default),
/// EnforcedStyle values, and other option defaults for all cops. Returns an
/// empty layer if the rubocop gem is not installed or the file can't be parsed.
///
/// Also returns the set of all cop names found in the installed gem's config,
/// used for core cop version awareness (cops not in the installed gem don't exist).
fn try_load_rubocop_defaults(working_dir: &Path) -> (ConfigLayer, HashSet<String>) {
    let gem_root = match gem_path::resolve_gem_path("rubocop", working_dir) {
        Ok(p) => p,
        Err(_) => return (ConfigLayer::empty(), HashSet::new()),
    };

    let default_config = gem_root.join("config").join("default.yml");
    if !default_config.exists() {
        return (ConfigLayer::empty(), HashSet::new());
    }

    let contents = match std::fs::read_to_string(&default_config) {
        Ok(c) => c,
        Err(_) => return (ConfigLayer::empty(), HashSet::new()),
    };

    // Strip Ruby-specific YAML tags (e.g., !ruby/regexp) that serde_yml can't handle
    let contents = contents.replace("!ruby/regexp ", "");

    let raw: Value = match serde_yml::from_str(&contents) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "warning: failed to parse rubocop default config {}: {e}",
                default_config.display()
            );
            return (ConfigLayer::empty(), HashSet::new());
        }
    };

    // Collect all cop names (keys containing '/') from the config.
    let known_cops: HashSet<String> = if let Value::Mapping(ref map) = raw {
        map.keys()
            .filter_map(|k| k.as_str())
            .filter(|k| k.contains('/'))
            .map(|k| k.to_string())
            .collect()
    } else {
        HashSet::new()
    };

    (parse_config_layer(&raw), known_cops)
}

/// Recursively load a config file and all its inherited configs.
///
/// `working_dir` is the top-level config directory used for gem path resolution
/// (where `Gemfile.lock` typically lives).
/// `visited` tracks absolute paths to detect circular inheritance.
fn load_config_recursive(
    config_path: &Path,
    working_dir: &Path,
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
    // Strip Ruby-specific YAML tags (e.g., !ruby/regexp) that serde_yml can't handle
    let contents = contents.replace("!ruby/regexp ", "");
    let raw: Value = serde_yml::from_str(&contents)
        .with_context(|| format!("failed to parse {}", config_path.display()))?;

    let config_dir = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    // Collect inherited layers in priority order (lowest first):
    // require: gem defaults < inherit_gem < inherit_from < local
    let mut base_layer = ConfigLayer::empty();

    if let Value::Mapping(ref map) = raw {
        // Peek at AllCops.TargetRubyVersion for version-aware standard gem config selection.
        // Needed before processing require: to select the right version-specific config file.
        let local_ruby_version: Option<f64> = map
            .get(&Value::String("AllCops".to_string()))
            .and_then(|ac| {
                if let Value::Mapping(ac_map) = ac {
                    ac_map
                        .get(&Value::String("TargetRubyVersion".to_string()))
                        .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
                } else {
                    None
                }
            });

        // 0. Process require: AND plugins: — load plugin default configs (lowest priority).
        //    A project may have both keys (e.g., `plugins: [rubocop-rspec]` and
        //    `require: [./custom_cop.rb]`), so we must process both.
        let mut gems = Vec::new();
        for key in &["plugins", "require"] {
            if let Some(val) = map.get(&Value::String(key.to_string())) {
                match val {
                    Value::String(s) => gems.push(s.clone()),
                    Value::Sequence(seq) => {
                        for v in seq {
                            if let Some(s) = v.as_str() {
                                gems.push(s.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        // Deduplicate (a gem may appear in both require: and plugins:)
        gems.dedup();
        if !gems.is_empty() {

            for gem_name in &gems {
                // Determine what config file to load for this gem.
                // rubocop-* gems use config/default.yml.
                // standard-family gems use version-specific or base config.
                // Other gems (custom cops, Ruby files, etc.) are skipped.
                let config_rel_path: Option<String> = if gem_name.starts_with("rubocop-") {
                    Some("config/default.yml".into())
                } else if let Some(path) =
                    standard_gem_config_path(gem_name, local_ruby_version)
                {
                    Some(path.into())
                } else {
                    None
                };
                let Some(config_rel_path) = config_rel_path else {
                    continue;
                };

                let gem_root = match gem_path::resolve_gem_path(gem_name, working_dir) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("warning: require '{}': {e:#}", gem_name);
                        continue;
                    }
                };
                let config_file = gem_root.join(&config_rel_path);
                if !config_file.exists() {
                    // For standard-family gems, fall back to config/base.yml if the
                    // version-specific file doesn't exist (older gem version).
                    if !gem_name.starts_with("rubocop-") {
                        let fallback = gem_root.join("config").join("base.yml");
                        if fallback.exists() {
                            match load_config_recursive(&fallback, working_dir, visited) {
                                Ok(layer) => merge_layer_into(&mut base_layer, &layer, None),
                                Err(e) => {
                                    eprintln!(
                                        "warning: failed to load default config for {}: {e:#}",
                                        gem_name
                                    );
                                }
                            }
                        }
                    }
                    continue;
                }
                match load_config_recursive(&config_file, working_dir, visited) {
                    Ok(layer) => merge_layer_into(&mut base_layer, &layer, None),
                    Err(e) => {
                        eprintln!(
                            "warning: failed to load default config for {}: {e:#}",
                            gem_name
                        );
                    }
                }
            }
        }

        // Record cops/depts enabled by require: defaults (for DisabledByDefault).
        // Under DisabledByDefault, these are NOT considered "explicitly enabled".
        let require_cops: HashSet<String> = base_layer
            .cop_configs
            .iter()
            .filter(|(_, c)| c.enabled == EnabledState::True)
            .map(|(n, _)| n.clone())
            .collect();
        let require_depts: HashSet<String> = base_layer
            .department_configs
            .iter()
            .filter(|(_, c)| c.enabled == EnabledState::True)
            .map(|(n, _)| n.clone())
            .collect();
        base_layer.require_enabled_cops = require_cops;
        base_layer.require_enabled_depts = require_depts;

        // Track ALL cops mentioned in require: gem configs (for version awareness).
        // Cops from plugin departments not in this set don't exist in the
        // installed gem version and should be treated as disabled.
        base_layer.require_known_cops = base_layer
            .cop_configs
            .keys()
            .cloned()
            .collect();
        base_layer.require_departments = base_layer
            .department_configs
            .keys()
            .cloned()
            .collect();

        // Also register departments from *requested* gems even if gem resolution
        // failed (e.g., `bundle` not on PATH). This ensures plugin departments
        // from requested gems are known, preventing false positives when we
        // disable unrequested plugin departments.
        for gem_name in &gems {
            for (dept, gem) in PLUGIN_GEM_DEPARTMENTS {
                if gem_name.as_str() == *gem {
                    base_layer.require_departments.insert(dept.to_string());
                }
            }
        }


        // 1. Process inherit_gem
        if let Some(gem_value) = map.get(&Value::String("inherit_gem".to_string())) {
            if let Value::Mapping(gem_map) = gem_value {
                for (gem_key, gem_paths) in gem_map {
                    if let Some(gem_name) = gem_key.as_str() {
                        let gem_layers =
                            resolve_inherit_gem(gem_name, gem_paths, working_dir, visited);
                        for layer in gem_layers {
                            merge_layer_into(&mut base_layer, &layer, None);
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
                match load_config_recursive(&inherited_path, working_dir, visited) {
                    Ok(layer) => merge_layer_into(&mut base_layer, &layer, None),
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

    // 3. Parse the local config layer and merge it on top (highest priority)
    let local_layer = parse_config_layer(&raw);
    merge_layer_into(&mut base_layer, &local_layer, Some(&local_layer.inherit_mode));

    Ok(base_layer)
}

/// Resolve `inherit_gem` entries. Each gem name maps to one or more YAML paths
/// relative to the gem's root directory.
fn resolve_inherit_gem(
    gem_name: &str,
    paths_value: &Value,
    working_dir: &Path,
    visited: &mut HashSet<PathBuf>,
) -> Vec<ConfigLayer> {
    let gem_root = match gem_path::resolve_gem_path(gem_name, working_dir) {
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
        match load_config_recursive(&full_path, working_dir, visited) {
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
    let mut department_configs = HashMap::new();
    let mut global_excludes = Vec::new();
    let mut new_cops = None;
    let mut disabled_by_default = None;
    let mut inherit_mode = InheritMode::default();
    let mut target_ruby_version = None;
    let mut target_rails_version = None;
    let mut active_support_extensions_enabled = None;

    if let Value::Mapping(map) = raw {
        for (key, value) in map {
            let key_str = match key.as_str() {
                Some(s) => s,
                None => continue,
            };

            // Skip non-cop top-level keys silently
            match key_str {
                "inherit_from" | "inherit_gem" | "require" | "plugins" => continue,
                "inherit_mode" => {
                    inherit_mode = parse_inherit_mode(value);
                    continue;
                }
                "AllCops" => {
                    if let Some(excludes) = extract_string_list(value, "Exclude") {
                        global_excludes = excludes;
                    }
                    if let Value::Mapping(ac_map) = value {
                        if let Some(nc) = ac_map.get(&Value::String("NewCops".to_string())) {
                            new_cops = nc.as_str().map(String::from);
                        }
                        if let Some(dbd) =
                            ac_map.get(&Value::String("DisabledByDefault".to_string()))
                        {
                            disabled_by_default = dbd.as_bool();
                        }
                        if let Some(trv) =
                            ac_map.get(&Value::String("TargetRubyVersion".to_string()))
                        {
                            target_ruby_version =
                                trv.as_f64().or_else(|| trv.as_u64().map(|u| u as f64));
                        }
                        if let Some(trv) =
                            ac_map.get(&Value::String("TargetRailsVersion".to_string()))
                        {
                            target_rails_version =
                                trv.as_f64().or_else(|| trv.as_u64().map(|u| u as f64));
                        }
                        if let Some(ase) =
                            ac_map.get(&Value::String("ActiveSupportExtensionsEnabled".to_string()))
                        {
                            active_support_extensions_enabled = ase.as_bool();
                        }
                    }
                    continue;
                }
                _ => {}
            }

            if key_str.contains('/') {
                // Cop-level config (e.g. "Style/FrozenStringLiteralComment")
                let cop_config = parse_cop_config(value);
                cop_configs.insert(key_str.to_string(), cop_config);
            } else {
                // Department-level config (e.g. "RSpec", "Rails")
                let dept_config = parse_department_config(value);
                department_configs.insert(key_str.to_string(), dept_config);
            }
        }
    }

    ConfigLayer {
        cop_configs,
        department_configs,
        global_excludes,
        new_cops,
        disabled_by_default,
        inherit_mode,
        require_enabled_cops: HashSet::new(),
        require_enabled_depts: HashSet::new(),
        require_known_cops: HashSet::new(),
        require_departments: HashSet::new(),
        target_ruby_version,
        target_rails_version,
        active_support_extensions_enabled,
    }
}

/// Merge an overlay layer into a base layer using RuboCop merge rules:
/// - Scalar values (Enabled, Severity, options): last writer wins
/// - Exclude arrays: appended (union) by default, replaced if `inherit_mode: override`
/// - Include arrays: replaced (override) by default, appended if `inherit_mode: merge`
/// - Global excludes: appended
/// - NewCops / DisabledByDefault: last writer wins
fn merge_layer_into(
    base: &mut ConfigLayer,
    overlay: &ConfigLayer,
    inherit_mode: Option<&InheritMode>,
) {
    // Merge global excludes: append
    for exc in &overlay.global_excludes {
        if !base.global_excludes.contains(exc) {
            base.global_excludes.push(exc.clone());
        }
    }

    // NewCops: last writer wins
    if overlay.new_cops.is_some() {
        base.new_cops.clone_from(&overlay.new_cops);
    }

    // DisabledByDefault: last writer wins
    if overlay.disabled_by_default.is_some() {
        base.disabled_by_default = overlay.disabled_by_default;
    }

    // TargetRubyVersion: last writer wins
    if overlay.target_ruby_version.is_some() {
        base.target_ruby_version = overlay.target_ruby_version;
    }

    // TargetRailsVersion: last writer wins
    if overlay.target_rails_version.is_some() {
        base.target_rails_version = overlay.target_rails_version;
    }

    // ActiveSupportExtensionsEnabled: last writer wins
    if overlay.active_support_extensions_enabled.is_some() {
        base.active_support_extensions_enabled = overlay.active_support_extensions_enabled;
    }

    // Merge department configs
    for (dept_name, overlay_dept) in &overlay.department_configs {
        match base.department_configs.get_mut(dept_name) {
            Some(base_dept) => {
                merge_department_config(base_dept, overlay_dept, inherit_mode);
            }
            None => {
                base.department_configs
                    .insert(dept_name.clone(), overlay_dept.clone());
            }
        }
    }

    // Merge per-cop configs
    for (cop_name, overlay_config) in &overlay.cop_configs {
        match base.cop_configs.get_mut(cop_name) {
            Some(base_config) => {
                merge_cop_config(base_config, overlay_config, inherit_mode);
            }
            None => {
                base.cop_configs
                    .insert(cop_name.clone(), overlay_config.clone());
            }
        }
        // Track require-originated enabled state through merges.
        if overlay.require_enabled_cops.contains(cop_name) {
            base.require_enabled_cops.insert(cop_name.clone());
        } else if overlay_config.enabled != EnabledState::Unset {
            base.require_enabled_cops.remove(cop_name);
        }
    }

    // Same for departments
    for (dept_name, overlay_dept) in &overlay.department_configs {
        if overlay.require_enabled_depts.contains(dept_name) {
            base.require_enabled_depts.insert(dept_name.clone());
        } else if overlay_dept.enabled != EnabledState::Unset {
            base.require_enabled_depts.remove(dept_name);
        }
    }

    // Propagate require-known cops and departments (union — once known, always known)
    for cop in &overlay.require_known_cops {
        base.require_known_cops.insert(cop.clone());
    }
    for dept in &overlay.require_departments {
        base.require_departments.insert(dept.clone());
    }
}

/// Merge a single department's overlay config into its base config.
fn merge_department_config(
    base: &mut DepartmentConfig,
    overlay: &DepartmentConfig,
    inherit_mode: Option<&InheritMode>,
) {
    // Enabled: last writer wins (only if overlay explicitly set it)
    if overlay.enabled != EnabledState::Unset {
        base.enabled = overlay.enabled;
    }

    let should_merge_include = inherit_mode
        .map(|im| im.merge.contains("Include"))
        .unwrap_or(false);
    let should_override_exclude = inherit_mode
        .map(|im| im.override_keys.contains("Exclude"))
        .unwrap_or(false);

    // Exclude: append by default, replace if inherit_mode says override
    if should_override_exclude {
        if !overlay.exclude.is_empty() {
            base.exclude.clone_from(&overlay.exclude);
        }
    } else {
        for exc in &overlay.exclude {
            if !base.exclude.contains(exc) {
                base.exclude.push(exc.clone());
            }
        }
    }

    // Include: replace by default, append if inherit_mode says merge
    if !overlay.include.is_empty() {
        if should_merge_include {
            for inc in &overlay.include {
                if !base.include.contains(inc) {
                    base.include.push(inc.clone());
                }
            }
        } else {
            base.include.clone_from(&overlay.include);
        }
    }
}

/// Merge a single cop's overlay config into its base config.
fn merge_cop_config(
    base: &mut CopConfig,
    overlay: &CopConfig,
    inherit_mode: Option<&InheritMode>,
) {
    // Enabled: last writer wins (only if overlay explicitly set it)
    if overlay.enabled != EnabledState::Unset {
        base.enabled = overlay.enabled;
    }

    // Severity: last writer wins (if overlay has one)
    if overlay.severity.is_some() {
        base.severity = overlay.severity;
    }

    let should_merge_include = inherit_mode
        .map(|im| im.merge.contains("Include"))
        .unwrap_or(false);
    let should_override_exclude = inherit_mode
        .map(|im| im.override_keys.contains("Exclude"))
        .unwrap_or(false);

    // Exclude: append (union) by default, replace if inherit_mode says override
    if should_override_exclude {
        if !overlay.exclude.is_empty() {
            base.exclude.clone_from(&overlay.exclude);
        }
    } else {
        for exc in &overlay.exclude {
            if !base.exclude.contains(exc) {
                base.exclude.push(exc.clone());
            }
        }
    }

    // Include: replace (override) by default, append if inherit_mode says merge
    if !overlay.include.is_empty() {
        if should_merge_include {
            for inc in &overlay.include {
                if !base.include.contains(inc) {
                    base.include.push(inc.clone());
                }
            }
        } else {
            base.include.clone_from(&overlay.include);
        }
    }

    // Check for cop-level inherit_mode in overlay options.
    // When a cop config contains `inherit_mode: { merge: [AllowedMethods] }`,
    // array options listed in `merge` should be appended instead of replaced.
    let cop_inherit_mode = overlay
        .options
        .get("inherit_mode")
        .and_then(|v| v.as_mapping())
        .map(|m| {
            let merge_keys: HashSet<String> = m
                .get(&Value::String("merge".to_string()))
                .and_then(|v| v.as_sequence())
                .map(|seq| {
                    seq.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            merge_keys
        })
        .unwrap_or_default();

    // Options: merge (last writer wins per key, deep-merge for Mapping values
    // to match RuboCop's behavior where Hash cop options are merged, not replaced)
    for (key, value) in &overlay.options {
        // Skip inherit_mode itself — it's a merge directive, not a cop option
        if key == "inherit_mode" {
            continue;
        }

        if let (Some(Value::Mapping(base_map)), Value::Mapping(overlay_map)) =
            (base.options.get(key), value)
        {
            let mut merged = base_map.clone();
            for (k, v) in overlay_map {
                merged.insert(k.clone(), v.clone());
            }
            base.options.insert(key.clone(), Value::Mapping(merged));
        } else if cop_inherit_mode.contains(key) {
            // Cop-level inherit_mode says merge this key — append arrays
            if let (Some(Value::Sequence(base_seq)), Value::Sequence(overlay_seq)) =
                (base.options.get(key), value)
            {
                let mut merged = base_seq.clone();
                for item in overlay_seq {
                    if !merged.contains(item) {
                        merged.push(item.clone());
                    }
                }
                base.options.insert(key.clone(), Value::Sequence(merged));
            } else {
                base.options.insert(key.clone(), value.clone());
            }
        } else {
            base.options.insert(key.clone(), value.clone());
        }
    }
}

impl ResolvedConfig {
    /// Check if a cop is enabled for the given file path.
    ///
    /// Evaluates in order:
    /// 1. Determine enabled state from cop config > department config > defaults.
    ///    - `False` → disabled
    ///    - `Pending` → disabled unless `AllCops.NewCops: enable`
    ///    - `Unset` → disabled if `AllCops.DisabledByDefault: true`
    ///    - `True` → enabled
    /// 2. If global excludes match the path, return false.
    /// 3. Merge cop's default_include/default_exclude with user/department overrides.
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
        let dept = name.split('/').next().unwrap_or("");
        let dept_config = self.department_configs.get(dept);

        // 1. Determine enabled state.
        // Standard precedence: cop-level > department-level > defaults.
        // Special case: department False overrides cop True when cop's True
        // came from defaults (not user config).
        let cop_enabled_state = config.map(|c| c.enabled).unwrap_or(EnabledState::Unset);
        let dept_enabled_state = dept_config
            .map(|dc| dc.enabled)
            .unwrap_or(EnabledState::Unset);

        let enabled_state = if cop_enabled_state == EnabledState::True
            && dept_enabled_state == EnabledState::False
            && !self.user_enabled_cops.contains(name)
        {
            EnabledState::False
        } else if cop_enabled_state != EnabledState::Unset {
            cop_enabled_state
        } else if dept_enabled_state != EnabledState::Unset {
            dept_enabled_state
        } else {
            EnabledState::Unset
        };

        match enabled_state {
            EnabledState::False => return false,
            EnabledState::Pending => {
                if self.new_cops != NewCopsPolicy::Enable {
                    return false;
                }
            }
            EnabledState::Unset => {
                if self.disabled_by_default {
                    return false;
                }
            }
            EnabledState::True => {}
        }

        // Plugin department awareness: cops from plugin departments (Rails, RSpec,
        // Performance, Migration, etc.) should only run if the corresponding gem was
        // loaded via `require:` or `plugins:`. If the project doesn't load the gem,
        // these cops must be disabled regardless of their default Enabled state.
        if is_plugin_department(dept) && !self.require_departments.contains(dept) {
            // The department wasn't loaded — cop should not fire unless the user
            // explicitly set it to Enabled:true in their project config.
            // Enabled:pending (from rubocop defaults) does not count.
            if config.is_none_or(|c| c.enabled != EnabledState::True) {
                return false;
            }
        }

        // Plugin version awareness: cop from require: department but not in gem config.
        // Only apply when the gem's config was actually loaded (has known cops for this dept).
        let dept_has_known_cops = self
            .require_known_cops
            .iter()
            .any(|c| c.starts_with(dept) && c.as_bytes().get(dept.len()) == Some(&b'/'));
        if dept_has_known_cops
            && self.require_departments.contains(dept)
            && !self.require_known_cops.contains(name)
            && config.is_none_or(|c| c.enabled == EnabledState::Unset)
        {
            return false;
        }

        // Core cop version awareness: if the installed rubocop gem's config was
        // loaded and this core cop isn't mentioned, it doesn't exist in that version.
        if !self.rubocop_known_cops.is_empty()
            && !is_plugin_department(dept)
            && !self.rubocop_known_cops.contains(name)
            && config.is_none_or(|c| c.enabled == EnabledState::Unset)
        {
            return false;
        }

        // 2. Global excludes
        for pattern in &self.global_excludes {
            if glob_matches(pattern, path) {
                return false;
            }
        }

        // 3. Build effective include/exclude lists.
        //    Precedence: cop config > department config > defaults.
        let effective_include: Vec<&str> = match config {
            Some(c) if !c.include.is_empty() => c.include.iter().map(|s| s.as_str()).collect(),
            _ => match dept_config {
                Some(dc) if !dc.include.is_empty() => {
                    dc.include.iter().map(|s| s.as_str()).collect()
                }
                _ => default_include.to_vec(),
            },
        };
        let effective_exclude: Vec<&str> = match config {
            Some(c) if !c.exclude.is_empty() => c.exclude.iter().map(|s| s.as_str()).collect(),
            _ => match dept_config {
                Some(dc) if !dc.exclude.is_empty() => {
                    dc.exclude.iter().map(|s| s.as_str()).collect()
                }
                _ => default_exclude.to_vec(),
            },
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
    ///
    /// Injects global AllCops settings (like TargetRubyVersion) into the
    /// cop's options so individual cops can access them without special plumbing.
    pub fn cop_config(&self, name: &str) -> CopConfig {
        let mut config = self.cop_configs.get(name).cloned().unwrap_or_default();
        // Inject TargetRubyVersion from AllCops into cop options
        // (only if the cop doesn't already have it set explicitly)
        if let Some(version) = self.target_ruby_version {
            config
                .options
                .entry("TargetRubyVersion".to_string())
                .or_insert_with(|| Value::Number(serde_yml::Number::from(version)));
        }
        // Inject TargetRailsVersion from AllCops into cop options
        if let Some(version) = self.target_rails_version {
            config
                .options
                .entry("TargetRailsVersion".to_string())
                .or_insert_with(|| Value::Number(serde_yml::Number::from(version)));
        }
        // Inject MaxLineLength and LineLengthEnabled from Layout/LineLength into
        // cops that need it (mirrors RuboCop's `config.for_cop('Layout/LineLength')`).
        // When Layout/LineLength is disabled, max_line_length returns nil in RuboCop,
        // which causes modifier_fits_on_single_line? to return true (no length limit).
        if matches!(
            name,
            "Style/IfUnlessModifier"
                | "Style/WhileUntilModifier"
                | "Style/GuardClause"
                | "Style/SoleNestedConditional"
                | "Layout/RedundantLineBreak"
        ) {
            let line_length_config = self.cop_configs.get("Layout/LineLength");
            if !config.options.contains_key("MaxLineLength") {
                let max = line_length_config
                    .and_then(|cc| cc.options.get("Max"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(120);
                config
                    .options
                    .insert("MaxLineLength".to_string(), Value::Number(serde_yml::Number::from(max)));
            }
            if !config.options.contains_key("LineLengthEnabled") {
                let enabled = line_length_config
                    .map(|cc| !matches!(cc.enabled, crate::cop::EnabledState::False))
                    .unwrap_or(true);
                config
                    .options
                    .insert("LineLengthEnabled".to_string(), Value::Bool(enabled));
            }
        }
        // Inject ActiveSupportExtensionsEnabled from AllCops for cops that need it
        if name == "Style/CollectionQuerying" {
            config
                .options
                .entry("ActiveSupportExtensionsEnabled".to_string())
                .or_insert_with(|| Value::Bool(self.active_support_extensions_enabled));
        }
        config
    }

    /// Get the resolved config for a cop, applying directory-specific overrides
    /// based on the file path.
    ///
    /// Finds the nearest `.rubocop.yml` in a parent directory of `file_path`
    /// (up to the project root) and merges its cop-specific settings on top of
    /// the root config. This supports per-directory config overrides like
    /// `db/migrate/.rubocop.yml` setting `CheckSymbols: false`.
    pub fn cop_config_for_file(&self, name: &str, file_path: &Path) -> CopConfig {
        let mut config = self.cop_config(name);

        if let Some(override_config) = self.find_dir_override(name, file_path) {
            // Merge the directory-specific cop config on top.
            // Options: last writer wins per key (directory config overrides root).
            for (key, value) in &override_config.options {
                config.options.insert(key.clone(), value.clone());
            }
            // Enabled: override if explicitly set
            if override_config.enabled != EnabledState::Unset {
                config.enabled = override_config.enabled;
            }
            // Severity: override if set
            if override_config.severity.is_some() {
                config.severity = override_config.severity;
            }
            // Include/Exclude: override if non-empty
            if !override_config.include.is_empty() {
                config.include.clone_from(&override_config.include);
            }
            if !override_config.exclude.is_empty() {
                config.exclude.clone_from(&override_config.exclude);
            }
        }

        config
    }

    /// Find the nearest directory-specific override for a cop, if any.
    /// Checks both the original file path and the path relativized to config_dir.
    fn find_dir_override(&self, cop_name: &str, file_path: &Path) -> Option<CopConfig> {
        if self.dir_overrides.is_empty() {
            return None;
        }

        // Try direct path match first (both paths in same form).
        // dir_overrides is sorted deepest-first, so first match is most specific.
        for (dir, cop_overrides) in &self.dir_overrides {
            if file_path.starts_with(dir) {
                return cop_overrides.get(cop_name).cloned();
            }
        }

        // Try matching with path relativized to config_dir
        // (handles running from outside the project root, e.g. bench/repos/mastodon/...)
        if let Some(ref config_dir) = self.config_dir {
            if let Ok(rel_path) = file_path.strip_prefix(config_dir) {
                for (dir, cop_overrides) in &self.dir_overrides {
                    if let Ok(rel_dir) = dir.strip_prefix(config_dir) {
                        if rel_path.starts_with(rel_dir) {
                            return cop_overrides.get(cop_name).cloned();
                        }
                    }
                }
            }
        }

        None
    }

    /// Global exclude patterns from AllCops.Exclude.
    pub fn global_excludes(&self) -> &[String] {
        &self.global_excludes
    }

    /// Directory containing the resolved config file.
    pub fn config_dir(&self) -> Option<&Path> {
        self.config_dir.as_deref()
    }

    /// Build pre-compiled cop filters for fast per-file enablement checks.
    ///
    /// This resolves all enabled states, include/exclude patterns, and global
    /// excludes into compiled `GlobSet` matchers. Call once at startup, then
    /// share across rayon workers.
    pub fn build_cop_filters(&self, registry: &CopRegistry) -> CopFilterSet {
        // Build global exclude set
        let global_exclude = {
            let pats: Vec<&str> = self.global_excludes.iter().map(|s| s.as_str()).collect();
            build_glob_set(&pats).unwrap_or_else(GlobSet::empty)
        };

        let filters = registry
            .cops()
            .iter()
            .map(|cop| {
                let name = cop.name();
                let config = self.cop_configs.get(name);
                let dept = name.split('/').next().unwrap_or("");
                let dept_config = self.department_configs.get(dept);

                // Determine enabled state.
                // Standard precedence: cop-level > department-level > defaults.
                // Special case: when the department is explicitly False and the cop is
                // True only from defaults (not user config), department wins. This matches
                // RuboCop's behavior where `Metrics: Enabled: false` disables all Metrics
                // cops even though rubocop defaults set them to Enabled: true.
                let cop_enabled_state = config.map(|c| c.enabled).unwrap_or(EnabledState::Unset);
                let dept_enabled_state = dept_config
                    .map(|dc| dc.enabled)
                    .unwrap_or(EnabledState::Unset);

                let enabled_state = if cop_enabled_state == EnabledState::True
                    && dept_enabled_state == EnabledState::False
                    && !self.user_enabled_cops.contains(name)
                {
                    // Department says False, cop says True but only from defaults.
                    // Department wins (user explicitly disabled the department).
                    EnabledState::False
                } else if cop_enabled_state != EnabledState::Unset {
                    cop_enabled_state
                } else if dept_enabled_state != EnabledState::Unset {
                    dept_enabled_state
                } else {
                    EnabledState::Unset
                };

                let mut enabled = match enabled_state {
                    EnabledState::False => false,
                    EnabledState::Pending => {
                        self.new_cops == NewCopsPolicy::Enable
                    }
                    EnabledState::Unset => !self.disabled_by_default && cop.default_enabled(),
                    EnabledState::True => true,
                };


                // Plugin department awareness: cops from plugin departments should
                // only run if the corresponding gem was loaded via require:/plugins:.
                if enabled
                    && is_plugin_department(dept)
                    && !self.require_departments.contains(dept)
                    && config.is_none_or(|c| c.enabled != EnabledState::True)
                {
                    enabled = false;
                }

                // Plugin version awareness: if this cop's department comes from a
                // `require:` gem but the cop itself is NOT mentioned in the installed
                // gem's config/default.yml, the cop doesn't exist in that gem version.
                // Disable it unless the user explicitly configured it.
                // Only apply this check when the gem's config was actually loaded
                // (i.e., require_known_cops contains at least one cop from this dept).
                let dept_has_known_cops = self
                    .require_known_cops
                    .iter()
                    .any(|c| c.starts_with(dept) && c.as_bytes().get(dept.len()) == Some(&b'/'));
                if enabled
                    && dept_has_known_cops
                    && self.require_departments.contains(dept)
                    && !self.require_known_cops.contains(name)
                    && config.is_none_or(|c| c.enabled != EnabledState::True)
                {
                    enabled = false;
                }

                // Core cop version awareness: if the installed rubocop gem's
                // config/default.yml was loaded (rubocop_known_cops is non-empty)
                // and this cop is from a core department but NOT mentioned in that
                // config, the cop doesn't exist in the project's rubocop version.
                // Disable it unless the user explicitly configured it.
                if enabled
                    && !self.rubocop_known_cops.is_empty()
                    && !is_plugin_department(dept)
                    && !self.rubocop_known_cops.contains(name)
                    && config.is_none_or(|c| c.enabled != EnabledState::True)
                {
                    enabled = false;
                }

                if !enabled {
                    return CopFilter {
                        enabled: false,
                        include_set: None,
                        exclude_set: None,
                    };
                }

                // Build effective include patterns (cop config > dept config > defaults)
                let include_patterns: Vec<&str> = match config {
                    Some(c) if !c.include.is_empty() => {
                        c.include.iter().map(|s| s.as_str()).collect()
                    }
                    _ => match dept_config {
                        Some(dc) if !dc.include.is_empty() => {
                            dc.include.iter().map(|s| s.as_str()).collect()
                        }
                        _ => cop.default_include().to_vec(),
                    },
                };

                // Build effective exclude patterns (cop config > dept config > defaults)
                let exclude_patterns: Vec<&str> = match config {
                    Some(c) if !c.exclude.is_empty() => {
                        c.exclude.iter().map(|s| s.as_str()).collect()
                    }
                    _ => match dept_config {
                        Some(dc) if !dc.exclude.is_empty() => {
                            dc.exclude.iter().map(|s| s.as_str()).collect()
                        }
                        _ => cop.default_exclude().to_vec(),
                    },
                };

                CopFilter {
                    enabled: true,
                    include_set: build_glob_set(&include_patterns),
                    exclude_set: build_glob_set(&exclude_patterns),
                }
            })
            .collect();

        // Discover sub-directory .rubocop.yml files for per-directory path relativity
        let sub_config_dirs = self
            .config_dir
            .as_ref()
            .map(|cd| discover_sub_config_dirs(cd))
            .unwrap_or_default();

        CopFilterSet {
            global_exclude,
            filters,
            config_dir: self.config_dir.clone(),
            sub_config_dirs,
        }
    }

    /// Return all cop names from the config that would be enabled given
    /// the current NewCops/DisabledByDefault settings.
    pub fn enabled_cop_names(&self) -> Vec<String> {
        self.cop_configs
            .iter()
            .filter(|(_name, config)| match config.enabled {
                EnabledState::True => true,
                EnabledState::Unset => !self.disabled_by_default,
                EnabledState::Pending => {
                    self.new_cops == NewCopsPolicy::Enable
                }
                EnabledState::False => false,
            })
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
                        config.enabled = if b {
                            EnabledState::True
                        } else {
                            EnabledState::False
                        };
                    } else if v.as_str() == Some("pending") {
                        config.enabled = EnabledState::Pending;
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

/// Parse a department-level config (e.g. `RSpec:` or `Rails:`).
fn parse_department_config(value: &Value) -> DepartmentConfig {
    let mut config = DepartmentConfig::default();

    if let Value::Mapping(map) = value {
        for (k, v) in map {
            match k.as_str() {
                Some("Enabled") => {
                    if let Some(b) = v.as_bool() {
                        config.enabled = if b {
                            EnabledState::True
                        } else {
                            EnabledState::False
                        };
                    } else if v.as_str() == Some("pending") {
                        config.enabled = EnabledState::Pending;
                    }
                }
                Some("Include") => {
                    if let Some(list) = value_to_string_list(v) {
                        config.include = list;
                    }
                }
                Some("Exclude") => {
                    if let Some(list) = value_to_string_list(v) {
                        config.exclude = list;
                    }
                }
                _ => {}
            }
        }
    }

    config
}

/// Parse the `inherit_mode` key from a config file.
fn parse_inherit_mode(value: &Value) -> InheritMode {
    let mut mode = InheritMode::default();

    if let Value::Mapping(map) = value {
        if let Some(merge_value) = map.get(&Value::String("merge".to_string())) {
            if let Some(list) = value_to_string_list(merge_value) {
                mode.merge = list.into_iter().collect();
            }
        }
        if let Some(override_value) = map.get(&Value::String("override".to_string())) {
            if let Some(list) = value_to_string_list(override_value) {
                mode.override_keys = list.into_iter().collect();
            }
        }
    }

    mode
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
/// Mapping from plugin department names to the gem that provides them.
/// Used to register departments from requested gems even when gem resolution fails.
/// Includes standard-family wrapper gems that wrap rubocop plugin gems.
const PLUGIN_GEM_DEPARTMENTS: &[(&str, &str)] = &[
    ("Rails", "rubocop-rails"),
    ("Migration", "rubocop-rails"),
    ("RSpec", "rubocop-rspec"),
    ("RSpecRails", "rubocop-rspec_rails"),
    ("FactoryBot", "rubocop-factory_bot"),
    ("Capybara", "rubocop-capybara"),
    ("Performance", "rubocop-performance"),
    // standard-family wrapper gems
    ("Rails", "standard-rails"),
    ("Migration", "standard-rails"),
    ("Performance", "standard-performance"),
];

/// Select config file for the `standard` gem based on target ruby version.
/// Mirrors Standard::Base::Plugin — each ruby-X.Y.yml inherits from
/// the next version up, chaining back to base.yml.
fn standard_version_config(ruby_version: f64) -> &'static str {
    if ruby_version < 1.9 {
        "config/ruby-1.8.yml"
    } else if ruby_version < 2.0 {
        "config/ruby-1.9.yml"
    } else if ruby_version < 2.1 {
        "config/ruby-2.0.yml"
    } else if ruby_version < 2.2 {
        "config/ruby-2.1.yml"
    } else if ruby_version < 2.3 {
        "config/ruby-2.2.yml"
    } else if ruby_version < 2.4 {
        "config/ruby-2.3.yml"
    } else if ruby_version < 2.5 {
        "config/ruby-2.4.yml"
    } else if ruby_version < 2.6 {
        "config/ruby-2.5.yml"
    } else if ruby_version < 2.7 {
        "config/ruby-2.6.yml"
    } else if ruby_version < 3.0 {
        "config/ruby-2.7.yml"
    } else if ruby_version < 3.1 {
        "config/ruby-3.0.yml"
    } else if ruby_version < 3.2 {
        "config/ruby-3.1.yml"
    } else if ruby_version < 3.3 {
        "config/ruby-3.2.yml"
    } else if ruby_version < 3.4 {
        "config/ruby-3.3.yml"
    } else {
        "config/base.yml"
    }
}

/// Select config file for the `standard-performance` gem based on target ruby version.
/// Mirrors Standard::Performance::DeterminesYamlPath.
fn standard_perf_version_config(ruby_version: f64) -> &'static str {
    if ruby_version < 1.9 {
        "config/ruby-1.8.yml"
    } else if ruby_version < 2.0 {
        "config/ruby-1.9.yml"
    } else if ruby_version < 2.1 {
        "config/ruby-2.0.yml"
    } else if ruby_version < 2.2 {
        "config/ruby-2.1.yml"
    } else if ruby_version < 2.3 {
        "config/ruby-2.2.yml"
    } else {
        "config/base.yml"
    }
}

/// Map a standard-family gem name to its config file path.
/// Returns None if the gem is not a recognized standard-family gem.
fn standard_gem_config_path(gem_name: &str, ruby_version: Option<f64>) -> Option<&'static str> {
    match gem_name {
        "standard" => Some(standard_version_config(ruby_version.unwrap_or(3.4))),
        "standard-performance" => {
            Some(standard_perf_version_config(ruby_version.unwrap_or(3.4)))
        }
        "standard-rails" | "standard-custom" => Some("config/base.yml"),
        _ => None,
    }
}

/// Returns true if the department belongs to a RuboCop plugin gem and should
/// only run when the corresponding gem is loaded via `require:` or `plugins:`.
///
/// Core departments (Layout, Lint, Style, Metrics, Naming, Security, Bundler,
/// Gemspec) are always available. Plugin departments need their gem loaded.
fn is_plugin_department(dept: &str) -> bool {
    PLUGIN_GEM_DEPARTMENTS
        .iter()
        .any(|(d, _)| *d == dept)
}

/// Parse a gem's major.minor version from a Gemfile.lock/gems.locked file.
/// Returns the version as a float (e.g. 7.1 for "7.1.3.4").
fn parse_gem_version_from_lockfile(content: &str, gem_name: &str) -> Option<f64> {
    // Gemfile.lock format has gems indented with 4 spaces in the GEM/specs section:
    //     railties (7.1.3.4)
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(gem_name) {
            if let Some(ver_str) = rest.strip_prefix(" (") {
                if let Some(ver_str) = ver_str.strip_suffix(')') {
                    let parts: Vec<&str> = ver_str.split('.').collect();
                    if parts.len() >= 2 {
                        if let (Ok(major), Ok(minor)) =
                            (parts[0].parse::<u64>(), parts[1].parse::<u64>())
                        {
                            return Some(major as f64 + minor as f64 / 10.0);
                        }
                    }
                }
            }
        }
    }
    None
}

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
        let config =
            load_config(Some(Path::new("/nonexistent/.rubocop.yml")), None).unwrap();
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
        let config = load_config(Some(&path), None).unwrap();
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
        let config = load_config(Some(&path), None).unwrap();
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
        let config = load_config(Some(&path), None).unwrap();
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
        let config = load_config(Some(&path), None).unwrap();
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
        let config = load_config(Some(&path), None).unwrap();
        let cc = config.cop_config("Layout/LineLength");
        assert_eq!(cc.options.get("Max").and_then(|v| v.as_u64()), Some(120));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn non_cop_keys_ignored() {
        let dir = std::env::temp_dir().join("rblint_test_config_noncop");
        fs::create_dir_all(&dir).unwrap();
        let path = write_config(&dir, "AllCops:\n  Exclude: []\nrequire:\n  - rubocop-rspec\n");
        let config = load_config(Some(&path), None).unwrap();
        // "require" has no "/" so should not be treated as a cop
        assert!(config.is_cop_enabled("require", Path::new("a.rb"), &[], &[]));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn default_cop_config() {
        let config =
            load_config(Some(Path::new("/nonexistent/.rubocop.yml")), None).unwrap();
        let cc = config.cop_config("Style/Whatever");
        assert_eq!(cc.enabled, EnabledState::Unset);
        assert!(cc.severity.is_none());
        assert!(cc.exclude.is_empty());
        assert!(cc.options.is_empty());
    }

    // ---- Path-based Include/Exclude tests ----

    #[test]
    fn default_include_filters_files() {
        let config =
            load_config(Some(Path::new("/nonexistent/.rubocop.yml")), None).unwrap();
        // With default_include set, only matching files pass
        // Use a core department (Style) so plugin department filtering doesn't apply.
        let inc = &["db/migrate/**/*.rb"];
        assert!(config.is_cop_enabled(
            "Style/Foo",
            Path::new("db/migrate/001_create.rb"),
            inc,
            &[]
        ));
        assert!(!config.is_cop_enabled(
            "Style/Foo",
            Path::new("app/models/user.rb"),
            inc,
            &[]
        ));
    }

    #[test]
    fn default_exclude_filters_files() {
        let config =
            load_config(Some(Path::new("/nonexistent/.rubocop.yml")), None).unwrap();
        let exc = &["spec/**/*.rb"];
        assert!(config.is_cop_enabled("Style/Foo", Path::new("app/models/user.rb"), &[], exc));
        assert!(!config.is_cop_enabled(
            "Style/Foo",
            Path::new("spec/models/user_spec.rb"),
            &[],
            exc
        ));
    }

    #[test]
    fn user_include_overrides_default() {
        let dir = std::env::temp_dir().join("rblint_test_config_inc_override");
        fs::create_dir_all(&dir).unwrap();
        // Use a core department cop (Style/) so plugin department filtering doesn't apply
        let path = write_config(
            &dir,
            "Style/Migration:\n  Include:\n    - 'db/**/*.rb'\n",
        );
        let config = load_config(Some(&path), None).unwrap();
        // Default include is narrower but user config overrides
        let default_inc = &["db/migrate/**/*.rb"];
        assert!(config.is_cop_enabled(
            "Style/Migration",
            Path::new("db/seeds.rb"),
            default_inc,
            &[]
        ));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn global_excludes_applied() {
        let dir = std::env::temp_dir().join("rblint_test_config_global_exc");
        fs::create_dir_all(&dir).unwrap();
        let path = write_config(&dir, "AllCops:\n  Exclude:\n    - 'vendor/**'\n");
        let config = load_config(Some(&path), None).unwrap();
        assert!(!config.is_cop_enabled(
            "Style/Foo",
            Path::new("vendor/gems/foo.rb"),
            &[],
            &[]
        ));
        assert!(config.is_cop_enabled("Style/Foo", Path::new("app/models/user.rb"), &[], &[]));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn glob_matches_basic() {
        assert!(glob_matches("**/*.rb", Path::new("app/models/user.rb")));
        assert!(glob_matches(
            "db/migrate/**/*.rb",
            Path::new("db/migrate/001_create.rb")
        ));
        assert!(!glob_matches(
            "db/migrate/**/*.rb",
            Path::new("app/models/user.rb")
        ));
        assert!(glob_matches(
            "spec/**",
            Path::new("spec/models/user_spec.rb")
        ));
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

        let config = load_config(Some(&path), None).unwrap();
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

        let config = load_config(Some(&path), None).unwrap();
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

        let config = load_config(Some(&path), None).unwrap();
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

        let config = load_config(Some(&path), None).unwrap();
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

        let config = load_config(Some(&path), None).unwrap();
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
        let config = load_config(Some(&path), None).unwrap();
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
        let result = load_config(Some(&path), None);
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

        let config = load_config(Some(&path), None).unwrap();
        assert!(!config.is_cop_enabled("Style/Foo", Path::new("a.rb"), &[], &[]));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn standard_version_config_selects_correct_file() {
        assert_eq!(standard_version_config(1.8), "config/ruby-1.8.yml");
        assert_eq!(standard_version_config(1.9), "config/ruby-1.9.yml");
        assert_eq!(standard_version_config(2.0), "config/ruby-2.0.yml");
        assert_eq!(standard_version_config(2.7), "config/ruby-2.7.yml");
        assert_eq!(standard_version_config(3.0), "config/ruby-3.0.yml");
        assert_eq!(standard_version_config(3.1), "config/ruby-3.1.yml");
        assert_eq!(standard_version_config(3.2), "config/ruby-3.2.yml");
        assert_eq!(standard_version_config(3.3), "config/ruby-3.3.yml");
        // 3.4+ uses base.yml (latest, no overrides needed)
        assert_eq!(standard_version_config(3.4), "config/base.yml");
        assert_eq!(standard_version_config(3.5), "config/base.yml");
    }

    #[test]
    fn standard_perf_version_config_selects_correct_file() {
        assert_eq!(
            standard_perf_version_config(1.8),
            "config/ruby-1.8.yml"
        );
        assert_eq!(
            standard_perf_version_config(2.2),
            "config/ruby-2.2.yml"
        );
        // 2.3+ uses base.yml
        assert_eq!(standard_perf_version_config(2.3), "config/base.yml");
        assert_eq!(standard_perf_version_config(3.1), "config/base.yml");
    }

    #[test]
    fn standard_gem_config_path_recognizes_family() {
        // standard gem: version-specific
        assert_eq!(
            standard_gem_config_path("standard", Some(3.1)),
            Some("config/ruby-3.1.yml")
        );
        assert_eq!(
            standard_gem_config_path("standard", None),
            Some("config/base.yml") // defaults to 3.4 → base.yml
        );

        // standard-rails: always base
        assert_eq!(
            standard_gem_config_path("standard-rails", Some(3.1)),
            Some("config/base.yml")
        );

        // standard-custom: always base
        assert_eq!(
            standard_gem_config_path("standard-custom", None),
            Some("config/base.yml")
        );

        // standard-performance: version-specific for old Ruby
        assert_eq!(
            standard_gem_config_path("standard-performance", Some(2.0)),
            Some("config/ruby-2.0.yml")
        );
        assert_eq!(
            standard_gem_config_path("standard-performance", Some(3.1)),
            Some("config/base.yml")
        );

        // Unknown gems: None
        assert_eq!(standard_gem_config_path("rubocop-rspec", None), None);
        assert_eq!(standard_gem_config_path("some-other-gem", None), None);
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

        let config = load_config(Some(&path), None).unwrap();
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

        let config = load_config(Some(&path), None).unwrap();
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
                enabled: EnabledState::True,
                ..CopConfig::default()
            },
        );

        let mut overlay = ConfigLayer::empty();
        overlay.cop_configs.insert(
            "Style/Foo".to_string(),
            CopConfig {
                enabled: EnabledState::False,
                ..CopConfig::default()
            },
        );

        merge_layer_into(&mut base, &overlay, None);
        assert_eq!(
            base.cop_configs["Style/Foo"].enabled,
            EnabledState::False
        );
    }

    #[test]
    fn merge_layer_global_excludes_appended() {
        let mut base = ConfigLayer {
            global_excludes: vec!["vendor/**".to_string()],
            ..ConfigLayer::empty()
        };
        let overlay = ConfigLayer {
            global_excludes: vec!["tmp/**".to_string()],
            ..ConfigLayer::empty()
        };
        merge_layer_into(&mut base, &overlay, None);
        assert_eq!(base.global_excludes.len(), 2);
        assert!(base.global_excludes.contains(&"vendor/**".to_string()));
        assert!(base.global_excludes.contains(&"tmp/**".to_string()));
    }

    #[test]
    fn merge_layer_no_duplicate_excludes() {
        let mut base = ConfigLayer {
            global_excludes: vec!["vendor/**".to_string()],
            ..ConfigLayer::empty()
        };
        let overlay = ConfigLayer {
            global_excludes: vec!["vendor/**".to_string()],
            ..ConfigLayer::empty()
        };
        merge_layer_into(&mut base, &overlay, None);
        assert_eq!(base.global_excludes.len(), 1);
    }

    // ---- Auto-discovery tests ----

    #[test]
    fn auto_discover_config_from_target_dir() {
        let dir = std::env::temp_dir().join("rblint_test_autodiscover");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        write_config(&dir, "Style/Foo:\n  Enabled: false\n");

        // Auto-discover from target_dir
        let config = load_config(None, Some(&dir)).unwrap();
        assert!(!config.is_cop_enabled("Style/Foo", Path::new("a.rb"), &[], &[]));
        assert!(config.config_dir().is_some());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn auto_discover_walks_up_parent() {
        let parent = std::env::temp_dir().join("rblint_test_autodiscover_parent");
        let child = parent.join("app").join("models");
        let _ = fs::remove_dir_all(&parent);
        fs::create_dir_all(&child).unwrap();

        write_config(&parent, "Style/Bar:\n  Enabled: false\n");

        // Target is a subdirectory — should find config in parent
        let config = load_config(None, Some(&child)).unwrap();
        assert!(!config.is_cop_enabled("Style/Bar", Path::new("a.rb"), &[], &[]));

        fs::remove_dir_all(&parent).ok();
    }

    #[test]
    fn no_config_found_returns_empty() {
        let dir = std::env::temp_dir().join("rblint_test_no_config");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let config = load_config(None, Some(&dir)).unwrap();
        assert!(config.global_excludes().is_empty());
        assert!(config.is_cop_enabled("Style/Foo", Path::new("a.rb"), &[], &[]));

        fs::remove_dir_all(&dir).ok();
    }

    // ---- EnabledState / Pending / NewCops tests ----

    #[test]
    fn enabled_pending_disabled_by_default() {
        let dir = std::env::temp_dir().join("rblint_test_pending_default");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let path = write_config(&dir, "Rails/Foo:\n  Enabled: pending\n");
        let config = load_config(Some(&path), None).unwrap();
        // Pending is disabled by default (no NewCops: enable)
        assert!(!config.is_cop_enabled("Rails/Foo", Path::new("a.rb"), &[], &[]));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn enabled_pending_with_new_cops_enable() {
        let dir = std::env::temp_dir().join("rblint_test_pending_enable");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        // Use a core department to test pending behavior without plugin filtering
        let path = write_config(
            &dir,
            "AllCops:\n  NewCops: enable\nLint/Foo:\n  Enabled: pending\n",
        );
        let config = load_config(Some(&path), None).unwrap();
        // Pending is enabled when NewCops: enable
        assert!(config.is_cop_enabled("Lint/Foo", Path::new("a.rb"), &[], &[]));

        fs::remove_dir_all(&dir).ok();
    }

    // ---- DisabledByDefault tests ----

    #[test]
    fn disabled_by_default_disables_unset_cops() {
        let dir = std::env::temp_dir().join("rblint_test_disabled_by_default");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let path = write_config(
            &dir,
            "AllCops:\n  DisabledByDefault: true\nStyle/Foo:\n  Enabled: true\n",
        );
        let config = load_config(Some(&path), None).unwrap();
        // Explicitly enabled cop is still enabled
        assert!(config.is_cop_enabled("Style/Foo", Path::new("a.rb"), &[], &[]));
        // Unmentioned cop is disabled
        assert!(!config.is_cop_enabled("Style/Bar", Path::new("a.rb"), &[], &[]));

        fs::remove_dir_all(&dir).ok();
    }

    // ---- Department-level config tests ----

    #[test]
    fn department_include_filters_cops() {
        let dir = std::env::temp_dir().join("rblint_test_dept_include");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        // Use a core department (Lint) to test department-level Include filtering
        // without being affected by plugin department detection.
        let path = write_config(
            &dir,
            "Lint:\n  Include:\n    - '**/*_spec.rb'\n    - '**/spec/**/*'\n",
        );
        let config = load_config(Some(&path), None).unwrap();
        // Lint cop should match spec files via department include
        assert!(config.is_cop_enabled(
            "Lint/ExampleLength",
            Path::new("spec/models/user_spec.rb"),
            &[],
            &[]
        ));
        // Lint cop should NOT match non-spec files
        assert!(!config.is_cop_enabled(
            "Lint/ExampleLength",
            Path::new("app/models/user.rb"),
            &[],
            &[]
        ));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn department_enabled_false_disables_all_cops() {
        let dir = std::env::temp_dir().join("rblint_test_dept_disabled");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        // Use core department (Lint) for testing Enabled:false since plugin
        // departments are already disabled without require:/plugins: loading
        let path = write_config(&dir, "Lint:\n  Enabled: false\n");
        let config = load_config(Some(&path), None).unwrap();
        assert!(!config.is_cop_enabled("Lint/FindBy", Path::new("a.rb"), &[], &[]));
        assert!(!config.is_cop_enabled("Lint/HttpStatus", Path::new("a.rb"), &[], &[]));
        // Other departments unaffected
        assert!(config.is_cop_enabled("Style/Foo", Path::new("a.rb"), &[], &[]));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn plugin_department_disabled_without_require() {
        // Plugin departments (Rails, RSpec, Performance, etc.) should be disabled
        // when the corresponding gem is not loaded via require:/plugins:
        let config =
            load_config(Some(Path::new("/nonexistent/.rubocop.yml")), None).unwrap();
        assert!(!config.is_cop_enabled("Rails/Output", Path::new("a.rb"), &[], &[]));
        assert!(!config.is_cop_enabled("RSpec/ExampleLength", Path::new("a.rb"), &[], &[]));
        assert!(!config.is_cop_enabled("Performance/Count", Path::new("a.rb"), &[], &[]));
        // Core departments still work
        assert!(config.is_cop_enabled("Style/Foo", Path::new("a.rb"), &[], &[]));
        assert!(config.is_cop_enabled("Lint/Foo", Path::new("a.rb"), &[], &[]));
    }

    #[test]
    fn cop_config_overrides_department() {
        let dir = std::env::temp_dir().join("rblint_test_cop_over_dept");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let path = write_config(
            &dir,
            "Rails:\n  Enabled: false\nRails/FindBy:\n  Enabled: true\n",
        );
        let config = load_config(Some(&path), None).unwrap();
        // Department says disabled, but cop says enabled — cop wins
        assert!(config.is_cop_enabled("Rails/FindBy", Path::new("a.rb"), &[], &[]));
        // Other Rails cops still disabled
        assert!(!config.is_cop_enabled("Rails/HttpStatus", Path::new("a.rb"), &[], &[]));

        fs::remove_dir_all(&dir).ok();
    }

    // ---- inherit_mode tests ----

    #[test]
    fn inherit_mode_merge_include() {
        let dir = std::env::temp_dir().join("rblint_test_inherit_mode_merge");
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
            "inherit_from: base.yml\ninherit_mode:\n  merge:\n    - Include\nStyle/Foo:\n  Include:\n    - '**/*.rake'\n",
        );

        let config = load_config(Some(&path), None).unwrap();
        let cc = config.cop_config("Style/Foo");
        // With merge mode, Include is appended instead of replaced
        assert!(cc.include.contains(&"**/*.rb".to_string()));
        assert!(cc.include.contains(&"**/*.rake".to_string()));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn inherit_mode_override_exclude() {
        let dir = std::env::temp_dir().join("rblint_test_inherit_mode_override");
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
            "inherit_from: base.yml\ninherit_mode:\n  override:\n    - Exclude\nStyle/Foo:\n  Exclude:\n    - 'tmp/**'\n",
        );

        let config = load_config(Some(&path), None).unwrap();
        let cc = config.cop_config("Style/Foo");
        // With override mode, Exclude is replaced instead of appended
        assert!(!cc.exclude.contains(&"vendor/**".to_string()));
        assert!(cc.exclude.contains(&"tmp/**".to_string()));

        fs::remove_dir_all(&dir).ok();
    }

    // ---- enabled_cop_names with pending/disabled_by_default ----

    #[test]
    fn enabled_cop_names_respects_pending_and_disabled_by_default() {
        let dir = std::env::temp_dir().join("rblint_test_names_pending");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let path = write_yaml(
            &dir,
            ".rubocop.yml",
            "AllCops:\n  NewCops: enable\n  DisabledByDefault: true\nStyle/Foo:\n  Enabled: true\nStyle/Bar:\n  Enabled: pending\nStyle/Baz:\n  Max: 10\n",
        );

        let config = load_config(Some(&path), None).unwrap();
        let names = config.enabled_cop_names();
        // Explicitly enabled
        assert!(names.contains(&"Style/Foo".to_string()));
        // Pending + NewCops: enable → enabled
        assert!(names.contains(&"Style/Bar".to_string()));
        // Unset + DisabledByDefault → disabled
        assert!(!names.contains(&"Style/Baz".to_string()));

        fs::remove_dir_all(&dir).ok();
    }

    // --- is_cop_match tests ---
    // These test the Include-OR / Exclude-OR logic that handles running
    // from outside the project root where file paths have a prefix.

    fn make_filter(
        enabled: bool,
        include: &[&str],
        exclude: &[&str],
    ) -> CopFilter {
        CopFilter {
            enabled,
            include_set: build_glob_set(include),
            exclude_set: build_glob_set(exclude),
        }
    }

    #[test]
    fn is_cop_match_exclude_works_on_relativized_path() {
        // Simulates running `rblint bench/repos/mastodon` where file paths
        // have a prefix but Exclude patterns are project-relative.
        let filter = make_filter(true, &[], &["lib/tasks/*.rake"]);
        let filter_set = CopFilterSet {
            global_exclude: GlobSet::empty(),
            filters: vec![filter],
            config_dir: Some(PathBuf::from("bench/repos/mastodon")),
            sub_config_dirs: Vec::new(),
        };
        let path = Path::new("bench/repos/mastodon/lib/tasks/emojis.rake");
        assert!(
            !filter_set.is_cop_match(0, path),
            "Exclude lib/tasks/*.rake should match relativized path"
        );
    }

    #[test]
    fn is_cop_match_include_works_with_absolute_patterns() {
        // Integration tests use absolute Include patterns like /tmp/test/db/migrate/**/*.rb
        let filter = make_filter(true, &["/tmp/test/db/migrate/**/*.rb"], &[]);
        let filter_set = CopFilterSet {
            global_exclude: GlobSet::empty(),
            filters: vec![filter],
            config_dir: Some(PathBuf::from("/tmp/test")),
            sub_config_dirs: Vec::new(),
        };
        let path = Path::new("/tmp/test/db/migrate/001_create_users.rb");
        assert!(
            filter_set.is_cop_match(0, path),
            "Absolute Include pattern should match full path"
        );
    }

    #[test]
    fn is_cop_match_include_works_with_relative_patterns() {
        // Relative Include pattern (e.g., spec/**/*_spec.rb) should match
        // both direct and prefixed paths.
        let filter = make_filter(true, &["**/spec/**/*_spec.rb"], &[]);
        let filter_set = CopFilterSet {
            global_exclude: GlobSet::empty(),
            filters: vec![filter],
            config_dir: Some(PathBuf::from("bench/repos/discourse")),
            sub_config_dirs: Vec::new(),
        };
        let path = Path::new("bench/repos/discourse/spec/models/user_spec.rb");
        assert!(
            filter_set.is_cop_match(0, path),
            "Relative Include with ** prefix should match prefixed path"
        );
    }

    #[test]
    fn is_cop_match_exclude_on_relativized_path_overrides_include() {
        // RSpec/EmptyExampleGroup scenario: Include matches via ** prefix,
        // but project-relative Exclude should still block the file.
        let filter = make_filter(
            true,
            &["**/spec/**/*_spec.rb"],
            &["spec/requests/api/*"],
        );
        let filter_set = CopFilterSet {
            global_exclude: GlobSet::empty(),
            filters: vec![filter],
            config_dir: Some(PathBuf::from("bench/repos/discourse")),
            sub_config_dirs: Vec::new(),
        };
        let path = Path::new("bench/repos/discourse/spec/requests/api/invites_spec.rb");
        assert!(
            !filter_set.is_cop_match(0, path),
            "Exclude spec/requests/api/* should block even when Include matches via **"
        );
    }

    #[test]
    fn is_cop_match_no_config_dir_uses_original_path() {
        // When config_dir is None, only the original path is checked.
        let filter = make_filter(true, &["**/*.rb"], &["vendor/**"]);
        let filter_set = CopFilterSet {
            global_exclude: GlobSet::empty(),
            filters: vec![filter],
            config_dir: None,
            sub_config_dirs: Vec::new(),
        };
        assert!(filter_set.is_cop_match(0, Path::new("app/models/user.rb")));
        assert!(!filter_set.is_cop_match(0, Path::new("vendor/gems/foo.rb")));
    }

    #[test]
    fn is_cop_match_disabled_filter_returns_false() {
        let filter = make_filter(false, &[], &[]);
        let filter_set = CopFilterSet {
            global_exclude: GlobSet::empty(),
            filters: vec![filter],
            config_dir: None,
            sub_config_dirs: Vec::new(),
        };
        assert!(!filter_set.is_cop_match(0, Path::new("anything.rb")));
    }
}
