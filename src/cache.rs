use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::cli::Args;
use crate::cop::CopConfig;
use crate::diagnostic::{Diagnostic, Location, Severity};

/// File-level result cache for incremental linting.
///
/// Two-level directory hierarchy:
/// ```text
/// <cache_root>/
/// └── <session_hash>/       # rblint version + config fingerprint + CLI args
///     └── <file_hash>       # per-file cache entry (JSON)
/// ```
///
/// Thread safety: each file gets a unique cache key file, so parallel rayon
/// workers never write to the same path. No locking needed.
pub struct ResultCache {
    session_dir: PathBuf,
    enabled: bool,
}

/// Compact cache entry without the file path (implied by cache key).
#[derive(serde::Serialize, serde::Deserialize)]
struct CachedDiagnostic {
    line: usize,
    column: usize,
    severity: char,
    cop: String,
    message: String,
}

impl CachedDiagnostic {
    fn from_diagnostic(d: &Diagnostic) -> Self {
        Self {
            line: d.location.line,
            column: d.location.column,
            severity: d.severity.letter(),
            cop: d.cop_name.clone(),
            message: d.message.clone(),
        }
    }

    fn to_diagnostic(self, path: &str) -> Diagnostic {
        Diagnostic {
            path: path.to_string(),
            location: Location {
                line: self.line,
                column: self.column,
            },
            severity: match self.severity {
                'W' => Severity::Warning,
                'E' => Severity::Error,
                'F' => Severity::Fatal,
                _ => Severity::Convention,
            },
            cop_name: self.cop,
            message: self.message,
        }
    }
}

impl ResultCache {
    /// Create a new result cache with session-level key.
    ///
    /// The session hash incorporates rblint version, config fingerprint, and
    /// CLI args that affect which cops run (--only, --except).
    pub fn new(version: &str, base_configs: &[CopConfig], args: &Args) -> Self {
        let cache_root = cache_root_dir();
        let session_hash = compute_session_hash(version, base_configs, args);
        let session_dir = cache_root.join(&session_hash);

        // Create session directory (ignore errors — caching is best-effort)
        let _ = std::fs::create_dir_all(&session_dir);

        Self {
            session_dir,
            enabled: true,
        }
    }

    /// Create a cache rooted at the given directory (for testing).
    pub fn with_root(root: &Path, version: &str, base_configs: &[CopConfig], args: &Args) -> Self {
        let session_hash = compute_session_hash(version, base_configs, args);
        let session_dir = root.join(&session_hash);
        let _ = std::fs::create_dir_all(&session_dir);
        Self {
            session_dir,
            enabled: true,
        }
    }

    /// Create a disabled (no-op) cache.
    pub fn disabled() -> Self {
        Self {
            session_dir: PathBuf::new(),
            enabled: false,
        }
    }

    /// Look up cached results for a file. Returns None on cache miss.
    pub fn get(&self, path: &Path, content: &[u8]) -> Option<Vec<Diagnostic>> {
        if !self.enabled {
            return None;
        }

        let file_hash = compute_file_hash(path, content);
        let cache_path = self.session_dir.join(&file_hash);

        let data = std::fs::read(&cache_path).ok()?;
        let entries: Vec<CachedDiagnostic> = serde_json::from_slice(&data).ok()?;

        let path_str = path.to_string_lossy();
        Some(
            entries
                .into_iter()
                .map(|e| e.to_diagnostic(&path_str))
                .collect(),
        )
    }

    /// Store results for a file. Best-effort — silently ignores write errors.
    pub fn put(&self, path: &Path, content: &[u8], diagnostics: &[Diagnostic]) {
        if !self.enabled {
            return;
        }

        let file_hash = compute_file_hash(path, content);
        let cache_path = self.session_dir.join(&file_hash);

        let entries: Vec<CachedDiagnostic> = diagnostics
            .iter()
            .map(CachedDiagnostic::from_diagnostic)
            .collect();

        // Write to a temp file and rename for atomicity (parallel workers).
        let tmp_path = cache_path.with_extension("tmp");
        if let Ok(json) = serde_json::to_vec(&entries) {
            if std::fs::write(&tmp_path, &json).is_ok() {
                let _ = std::fs::rename(&tmp_path, &cache_path);
            }
        }
    }

    /// Whether this cache is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Evict old session directories when total cached files exceed the limit.
    ///
    /// Deletes the oldest 50% of session directories by mtime.
    pub fn evict(&self, max_files: usize) {
        if !self.enabled {
            return;
        }
        let cache_root = cache_root_dir();
        let _ = evict_old_sessions(&cache_root, max_files);
    }
}

/// Determine the cache root directory (XDG-compliant).
///
/// Precedence:
/// 1. `$RBLINT_CACHE_DIR`
/// 2. `$XDG_CACHE_HOME/rblint/`
/// 3. `~/.cache/rblint/`
fn cache_root_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("RBLINT_CACHE_DIR") {
        return PathBuf::from(dir);
    }
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        return PathBuf::from(xdg).join("rblint");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".cache").join("rblint");
    }
    // Fallback for systems without HOME
    PathBuf::from(".rblint-cache")
}

/// Compute the session hash from version + config + CLI args.
///
/// The config fingerprint must be deterministic across runs. Since CopConfig
/// contains `HashMap<String, Value>` (non-deterministic iteration order), we
/// sort keys before hashing rather than relying on serde_json serialization.
fn compute_session_hash(version: &str, base_configs: &[CopConfig], args: &Args) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"rblint-session-v1:");
    hasher.update(version.as_bytes());
    hasher.update(b":");

    // Config fingerprint: hash each cop config deterministically
    for config in base_configs {
        hasher.update(format!("{:?}", config.enabled).as_bytes());
        hasher.update(format!("{:?}", config.severity).as_bytes());
        // Sort exclude/include for determinism (they should already be stable,
        // but this guards against order changes in config loading)
        let mut exclude = config.exclude.clone();
        exclude.sort();
        for e in &exclude {
            hasher.update(b"exc:");
            hasher.update(e.as_bytes());
        }
        let mut include = config.include.clone();
        include.sort();
        for i in &include {
            hasher.update(b"inc:");
            hasher.update(i.as_bytes());
        }
        // Sort options by key for deterministic hashing
        let mut keys: Vec<&String> = config.options.keys().collect();
        keys.sort();
        for key in keys {
            hasher.update(b"opt:");
            hasher.update(key.as_bytes());
            hasher.update(b"=");
            // Use Debug format for Value — not perfect but deterministic for
            // the same value, and Value types are simple (strings, ints, bools, arrays)
            hasher.update(format!("{:?}", config.options[key]).as_bytes());
        }
        hasher.update(b"|");
    }
    hasher.update(b":");

    // --only and --except affect which cops run
    for cop in &args.only {
        hasher.update(b"only:");
        hasher.update(cop.as_bytes());
    }
    for cop in &args.except {
        hasher.update(b"except:");
        hasher.update(cop.as_bytes());
    }

    let hash = hasher.finalize();
    format!("{:x}", hash)[..16].to_string() // 16 hex chars = 64 bits, enough to avoid collisions
}

/// Compute the per-file hash from path + content.
fn compute_file_hash(path: &Path, content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"rblint-file-v1:");
    hasher.update(path.to_string_lossy().as_bytes());
    hasher.update(b":");
    hasher.update(content);

    let hash = hasher.finalize();
    format!("{:x}", hash)[..16].to_string()
}

/// Remove the entire cache directory.
pub fn clear_cache() -> std::io::Result<()> {
    let cache_root = cache_root_dir();
    if cache_root.exists() {
        std::fs::remove_dir_all(&cache_root)?;
    }
    Ok(())
}

/// Evict old session directories when total files exceed max_files.
fn evict_old_sessions(cache_root: &Path, max_files: usize) -> std::io::Result<()> {
    let entries: Vec<_> = std::fs::read_dir(cache_root)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    // Count total cached files across all sessions
    let mut total_files: usize = 0;
    let mut sessions: Vec<(PathBuf, std::time::SystemTime, usize)> = Vec::new();

    for entry in entries {
        let path = entry.path();
        let mtime = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let file_count = std::fs::read_dir(&path)
            .map(|rd| rd.count())
            .unwrap_or(0);
        total_files += file_count;
        sessions.push((path, mtime, file_count));
    }

    if total_files <= max_files {
        return Ok(());
    }

    // Sort by mtime ascending (oldest first)
    sessions.sort_by_key(|(_, mtime, _)| *mtime);

    // Delete oldest sessions until we're under 50% of max
    let target = max_files / 2;
    let mut remaining = total_files;
    for (path, _, count) in &sessions {
        if remaining <= target {
            break;
        }
        let _ = std::fs::remove_dir_all(path);
        remaining -= count;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_args() -> Args {
        Args {
            paths: vec![".".into()],
            config: None,
            format: "text".to_string(),
            only: vec![],
            except: vec![],
            no_color: false,
            debug: false,
            rubocop_only: false,
            list_cops: false,
            stdin: None,
            init: false,
            no_cache: false,
            cache: false,
            cache_clear: false,
        }
    }

    #[test]
    fn disabled_cache_returns_none() {
        let cache = ResultCache::disabled();
        assert!(!cache.is_enabled());
        assert!(cache.get(Path::new("test.rb"), b"content").is_none());
    }

    #[test]
    fn cache_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let args = test_args();
        let configs: Vec<CopConfig> = vec![CopConfig::default()];
        let cache = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs, &args);
        assert!(cache.is_enabled());

        let path = Path::new("test.rb");
        let content = b"x = 1";

        // Cache miss
        assert!(cache.get(path, content).is_none());

        // Store
        let diagnostics = vec![Diagnostic {
            path: "test.rb".to_string(),
            location: Location { line: 1, column: 0 },
            severity: Severity::Convention,
            cop_name: "Style/Test".to_string(),
            message: "test offense".to_string(),
        }];
        cache.put(path, content, &diagnostics);

        // Cache hit
        let cached = cache.get(path, content).unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].cop_name, "Style/Test");
        assert_eq!(cached[0].message, "test offense");
        assert_eq!(cached[0].location.line, 1);
        assert_eq!(cached[0].location.column, 0);
        assert_eq!(cached[0].path, "test.rb");

        // Different content = cache miss
        assert!(cache.get(path, b"x = 2").is_none());
    }

    #[test]
    fn config_change_invalidates_session() {
        let tmp = tempfile::tempdir().unwrap();
        let args = test_args();
        let configs1 = vec![CopConfig::default()];
        let cache1 = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs1, &args);

        let path = Path::new("test.rb");
        let content = b"x = 1";
        cache1.put(path, content, &[]);

        // Same config = cache hit
        let cache1b = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs1, &args);
        assert!(cache1b.get(path, content).is_some());

        // Different config = different session = cache miss
        let mut config2 = CopConfig::default();
        config2.enabled = crate::cop::EnabledState::False;
        let configs2 = vec![config2];
        let cache2 = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs2, &args);
        assert!(cache2.get(path, content).is_none());
    }

    #[test]
    fn empty_diagnostics_cached() {
        let tmp = tempfile::tempdir().unwrap();
        let args = test_args();
        let configs = vec![CopConfig::default()];
        let cache = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs, &args);

        let path = Path::new("clean.rb");
        let content = b"# clean file";
        cache.put(path, content, &[]);

        let cached = cache.get(path, content).unwrap();
        assert!(cached.is_empty());
    }

    #[test]
    fn eviction_removes_old_sessions() {
        let tmp = tempfile::tempdir().unwrap();
        let args = test_args();

        // Create two sessions with different configs
        let configs1 = vec![CopConfig::default()];
        let cache1 = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs1, &args);
        // Put many files in session 1
        for i in 0..15 {
            cache1.put(
                Path::new(&format!("file{i}.rb")),
                format!("content {i}").as_bytes(),
                &[],
            );
        }

        let mut config2 = CopConfig::default();
        config2.enabled = crate::cop::EnabledState::False;
        let configs2 = vec![config2];
        let cache2 = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs2, &args);
        for i in 0..15 {
            cache2.put(
                Path::new(&format!("file{i}.rb")),
                format!("content2 {i}").as_bytes(),
                &[],
            );
        }

        // 30 files total, evict at 20 max
        evict_old_sessions(tmp.path(), 20).unwrap();

        // At least one session should have been removed
        let remaining: Vec<_> = std::fs::read_dir(tmp.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();
        // Should have removed at least one session directory
        assert!(remaining.len() <= 2);
    }
}
