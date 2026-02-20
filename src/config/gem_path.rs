use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::SystemTime;

use anyhow::{Context, Result};

/// Cache for gem path resolution. Keyed on (working_dir, gem_name), stores
/// the resolved path. Invalidated when Gemfile.lock mtime changes.
struct GemPathCache {
    entries: HashMap<(PathBuf, String), PathBuf>,
    lockfile_mtime: Option<SystemTime>,
    working_dir: PathBuf,
}

static GEM_PATH_CACHE: Mutex<Option<GemPathCache>> = Mutex::new(None);

/// Resolve a gem's install path via `bundle info --path <gem_name>`.
///
/// `working_dir` is the directory where `bundle` should run (typically the
/// project root where `Gemfile.lock` lives). Results are cached per
/// (working_dir, gem_name) and invalidated when Gemfile.lock mtime changes.
pub fn resolve_gem_path(gem_name: &str, working_dir: &Path) -> Result<PathBuf> {
    let lockfile_mtime = working_dir
        .join("Gemfile.lock")
        .metadata()
        .and_then(|m| m.modified())
        .ok();

    let cache_key = (working_dir.to_path_buf(), gem_name.to_string());

    // Check cache
    {
        let cache = GEM_PATH_CACHE.lock().unwrap();
        if let Some(ref c) = *cache {
            if c.working_dir == working_dir && c.lockfile_mtime == lockfile_mtime {
                if let Some(path) = c.entries.get(&cache_key) {
                    return Ok(path.clone());
                }
            }
        }
    }

    // Run bundle info --path from the working directory
    let bundle_start = std::time::Instant::now();
    let output = Command::new("bundle")
        .args(["info", "--path", gem_name])
        .current_dir(working_dir)
        .output()
        .with_context(|| {
            format!(
                "Cannot resolve gem '{}': `bundle` not found on PATH. \
                 Install Bundler or remove inherit_gem/require from your .rubocop.yml.",
                gem_name
            )
        })?;
    let bundle_elapsed = bundle_start.elapsed();
    eprintln!(
        "debug: bundle info --path {}: {:.0?}",
        gem_name, bundle_elapsed
    );

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Gem '{}' not found in bundle (working_dir: {}). \
             Run `bundle install` or remove it from inherit_gem. \
             bundle info stderr: {}",
            gem_name,
            working_dir.display(),
            stderr.trim()
        );
    }

    let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let path = PathBuf::from(&path_str);

    if !path.exists() {
        anyhow::bail!(
            "Gem '{}' resolved to '{}' but that directory does not exist.",
            gem_name,
            path_str
        );
    }

    // Store in cache
    {
        let mut cache = GEM_PATH_CACHE.lock().unwrap();
        let c = cache.get_or_insert_with(|| GemPathCache {
            entries: HashMap::new(),
            lockfile_mtime,
            working_dir: working_dir.to_path_buf(),
        });
        // Reset cache if lockfile or working_dir changed
        if c.lockfile_mtime != lockfile_mtime || c.working_dir != working_dir {
            c.entries.clear();
            c.lockfile_mtime = lockfile_mtime;
            c.working_dir = working_dir.to_path_buf();
        }
        c.entries.insert(cache_key, path.clone());
    }

    Ok(path)
}

/// Extract all resolved gem paths from the in-process cache.
/// Returns a map of gem_name → gem_root_path.
/// Used by `turbocop --init` to populate the lockfile.
pub fn drain_resolved_paths() -> HashMap<String, PathBuf> {
    let cache = GEM_PATH_CACHE.lock().unwrap();
    match *cache {
        Some(ref c) => c
            .entries
            .iter()
            .map(|((_, gem_name), path)| (gem_name.clone(), path.clone()))
            .collect(),
        None => HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bundle_info_output() {
        // Simulate trimming of bundle info output
        let raw = "  /home/user/.gem/ruby/3.2.0/gems/rubocop-shopify-2.15.1  \n";
        let trimmed = raw.trim();
        assert_eq!(
            trimmed,
            "/home/user/.gem/ruby/3.2.0/gems/rubocop-shopify-2.15.1"
        );
        let path = PathBuf::from(trimmed);
        assert_eq!(
            path.file_name().unwrap().to_str().unwrap(),
            "rubocop-shopify-2.15.1"
        );
    }

    #[test]
    fn cache_key_behavior() {
        // Verify None == None for lockfile mtime comparison
        let a: Option<SystemTime> = None;
        let b: Option<SystemTime> = None;
        assert_eq!(a, b);
    }
}
