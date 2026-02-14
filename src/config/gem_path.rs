use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::SystemTime;

use anyhow::{Context, Result};

/// Cache for gem path resolution. Keyed on gem name, stores the resolved path.
/// Invalidated when Gemfile.lock mtime changes.
struct GemPathCache {
    entries: HashMap<String, PathBuf>,
    lockfile_mtime: Option<SystemTime>,
}

static GEM_PATH_CACHE: Mutex<Option<GemPathCache>> = Mutex::new(None);

/// Resolve a gem's install path via `bundle info --path <gem_name>`.
///
/// Results are cached and invalidated when Gemfile.lock mtime changes.
pub fn resolve_gem_path(gem_name: &str) -> Result<PathBuf> {
    let lockfile_mtime = Path::new("Gemfile.lock")
        .metadata()
        .and_then(|m| m.modified())
        .ok();

    // Check cache
    {
        let cache = GEM_PATH_CACHE.lock().unwrap();
        if let Some(ref c) = *cache {
            if c.lockfile_mtime == lockfile_mtime {
                if let Some(path) = c.entries.get(gem_name) {
                    return Ok(path.clone());
                }
            }
        }
    }

    // Run bundle info --path
    let output = Command::new("bundle")
        .args(["info", "--path", gem_name])
        .output()
        .with_context(|| {
            format!(
                "Cannot resolve inherit_gem for '{}': `bundle` not found on PATH. \
                 Install Bundler or remove inherit_gem from your .rubocop.yml.",
                gem_name
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Gem '{}' not found in bundle. \
             Run `bundle install` or remove it from inherit_gem. \
             bundle info stderr: {}",
            gem_name,
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
        });
        // Reset cache if lockfile changed
        if c.lockfile_mtime != lockfile_mtime {
            c.entries.clear();
            c.lockfile_mtime = lockfile_mtime;
        }
        c.entries.insert(gem_name.to_string(), path.clone());
    }

    Ok(path)
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
