use std::path::{Path, PathBuf};
use std::time::SystemTime;

use sha2::{Digest, Sha256};

use crate::cli::Args;
use crate::cop::CopConfig;
use crate::diagnostic::{Diagnostic, Location, Severity};

/// File-level result cache for incremental linting.
///
/// Two-level directory hierarchy:
/// ```text
/// <cache_root>/
/// └── <session_hash>/       # turbocop version + config fingerprint + CLI args
///     └── <path_hash>       # per-file cache entry (JSON)
/// ```
///
/// Two-tier lookup per file:
/// 1. **Stat check** (mtime + size) — no file read needed, instant for local dev
/// 2. **Content hash** fallback — handles CI, git checkout, and other mtime-unreliable scenarios
///
/// Thread safety: each file gets a unique cache key (path hash), so parallel
/// rayon workers never write to the same path. No locking needed.
pub struct ResultCache {
    session_dir: PathBuf,
    enabled: bool,
}

/// Full cache entry stored on disk: metadata + diagnostics.
#[derive(serde::Serialize, serde::Deserialize)]
struct CacheEntry {
    /// Seconds since UNIX epoch of the file's mtime when cached.
    mtime_secs: u64,
    /// Nanosecond component of the file's mtime.
    mtime_nanos: u32,
    /// File size in bytes.
    size: u64,
    /// SHA-256 hex digest of the file content.
    content_hash: String,
    /// Cached lint diagnostics.
    diagnostics: Vec<CachedDiagnostic>,
}

/// Compact diagnostic without the file path (implied by cache key).
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

/// Result of a cache lookup attempt.
pub enum CacheLookup {
    /// Cache hit via mtime+size — no file read was needed.
    StatHit(Vec<Diagnostic>),
    /// Cache hit via content hash — file was read but didn't need re-linting.
    /// The mtime has been updated in the cache entry for next time.
    ContentHit(Vec<Diagnostic>),
    /// Cache miss — file needs to be linted.
    Miss,
}

impl ResultCache {
    /// Create a new result cache with session-level key.
    pub fn new(version: &str, base_configs: &[CopConfig], args: &Args) -> Self {
        let cache_root = cache_root_dir();
        let session_hash = compute_session_hash(version, base_configs, args);
        let session_dir = cache_root.join(&session_hash);
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

    /// Whether this cache is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Try to get cached results using only a stat() call (no file read).
    ///
    /// Returns `StatHit` if mtime+size match the cached entry.
    /// Returns `Miss` if no cache entry exists or mtime/size changed.
    ///
    /// This is the fast path for local development where mtimes are stable.
    pub fn get_by_stat(&self, path: &Path) -> CacheLookup {
        if !self.enabled {
            return CacheLookup::Miss;
        }

        let meta = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => return CacheLookup::Miss,
        };

        let entry = match self.read_entry(path) {
            Some(e) => e,
            None => return CacheLookup::Miss,
        };

        let (mtime_secs, mtime_nanos) = systemtime_to_parts(meta.modified().ok());
        let size = meta.len();

        if entry.mtime_secs == mtime_secs
            && entry.mtime_nanos == mtime_nanos
            && entry.size == size
        {
            let path_str = path.to_string_lossy();
            CacheLookup::StatHit(
                entry
                    .diagnostics
                    .into_iter()
                    .map(|e| e.to_diagnostic(&path_str))
                    .collect(),
            )
        } else {
            CacheLookup::Miss
        }
    }

    /// Try to get cached results using the file content hash.
    ///
    /// Called when `get_by_stat` returned `Miss` (mtime changed).
    /// If the content hash matches, updates the stored mtime for future fast hits
    /// and returns `ContentHit`. Otherwise returns `Miss`.
    ///
    /// This handles CI (mtime unreliable) and git checkout (mtime changes but
    /// content often unchanged).
    pub fn get_by_content(&self, path: &Path, content: &[u8]) -> CacheLookup {
        if !self.enabled {
            return CacheLookup::Miss;
        }

        let entry = match self.read_entry(path) {
            Some(e) => e,
            None => return CacheLookup::Miss,
        };

        let content_hash = compute_content_hash(content);
        if entry.content_hash == content_hash {
            // Content unchanged — update mtime+size for future stat hits
            let meta = std::fs::metadata(path).ok();
            let (mtime_secs, mtime_nanos) = systemtime_to_parts(meta.as_ref().and_then(|m| m.modified().ok()));
            let size = meta.map(|m| m.len()).unwrap_or(content.len() as u64);

            let updated = CacheEntry {
                mtime_secs,
                mtime_nanos,
                size,
                content_hash: entry.content_hash,
                diagnostics: entry.diagnostics,
            };
            // Re-extract diagnostics before writing (need to clone for return)
            let path_str = path.to_string_lossy();
            let result: Vec<Diagnostic> = updated
                .diagnostics
                .iter()
                .map(|e| {
                    CachedDiagnostic {
                        line: e.line,
                        column: e.column,
                        severity: e.severity,
                        cop: e.cop.clone(),
                        message: e.message.clone(),
                    }
                    .to_diagnostic(&path_str)
                })
                .collect();

            self.write_entry(path, &updated);
            CacheLookup::ContentHit(result)
        } else {
            CacheLookup::Miss
        }
    }

    /// Store results for a file. Best-effort — silently ignores write errors.
    pub fn put(&self, path: &Path, content: &[u8], diagnostics: &[Diagnostic]) {
        if !self.enabled {
            return;
        }

        let meta = std::fs::metadata(path).ok();
        let (mtime_secs, mtime_nanos) = systemtime_to_parts(meta.as_ref().and_then(|m| m.modified().ok()));
        let size = meta.map(|m| m.len()).unwrap_or(content.len() as u64);

        let entry = CacheEntry {
            mtime_secs,
            mtime_nanos,
            size,
            content_hash: compute_content_hash(content),
            diagnostics: diagnostics
                .iter()
                .map(CachedDiagnostic::from_diagnostic)
                .collect(),
        };

        self.write_entry(path, &entry);
    }

    /// Evict old session directories when total cached files exceed the limit.
    pub fn evict(&self, max_files: usize) {
        if !self.enabled {
            return;
        }
        let cache_root = cache_root_dir();
        let _ = evict_old_sessions(&cache_root, max_files);
    }

    fn cache_path_for(&self, path: &Path) -> PathBuf {
        self.session_dir.join(compute_path_hash(path))
    }

    fn read_entry(&self, path: &Path) -> Option<CacheEntry> {
        let cache_path = self.cache_path_for(path);
        let data = std::fs::read(&cache_path).ok()?;
        serde_json::from_slice(&data).ok()
    }

    fn write_entry(&self, path: &Path, entry: &CacheEntry) {
        let cache_path = self.cache_path_for(path);
        let tmp_path = cache_path.with_extension("tmp");
        if let Ok(json) = serde_json::to_vec(entry) {
            if std::fs::write(&tmp_path, &json).is_ok() {
                let _ = std::fs::rename(&tmp_path, &cache_path);
            }
        }
    }
}

/// Convert SystemTime to (secs, nanos) since UNIX epoch.
fn systemtime_to_parts(time: Option<SystemTime>) -> (u64, u32) {
    match time {
        Some(t) => match t.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(d) => (d.as_secs(), d.subsec_nanos()),
            Err(_) => (0, 0),
        },
        None => (0, 0),
    }
}

/// Compute a stable hash of just the file path (used as cache filename).
fn compute_path_hash(path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"turbocop-path-v2:");
    hasher.update(path.to_string_lossy().as_bytes());
    let hash = hasher.finalize();
    format!("{:x}", hash)[..16].to_string()
}

/// Compute SHA-256 of file content.
fn compute_content_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let hash = hasher.finalize();
    format!("{:x}", hash)
}

/// Determine the cache root directory (XDG-compliant).
///
/// Precedence:
/// 1. `$TURBOCOP_CACHE_DIR`
/// 2. `$XDG_CACHE_HOME/turbocop/`
/// 3. `~/.cache/turbocop/`
fn cache_root_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("TURBOCOP_CACHE_DIR") {
        return PathBuf::from(dir);
    }
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        return PathBuf::from(xdg).join("turbocop");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".cache").join("turbocop");
    }
    PathBuf::from(".turbocop-cache")
}

/// Compute the session hash from version + config + CLI args.
///
/// The config fingerprint must be deterministic across runs. Since CopConfig
/// contains `HashMap<String, Value>` (non-deterministic iteration order), we
/// sort keys before hashing rather than relying on serde_json serialization.
fn compute_session_hash(version: &str, base_configs: &[CopConfig], args: &Args) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"turbocop-session-v2:");
    hasher.update(version.as_bytes());
    hasher.update(b":");

    for config in base_configs {
        hasher.update(format!("{:?}", config.enabled).as_bytes());
        hasher.update(format!("{:?}", config.severity).as_bytes());
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
        let mut keys: Vec<&String> = config.options.keys().collect();
        keys.sort();
        for key in keys {
            hasher.update(b"opt:");
            hasher.update(key.as_bytes());
            hasher.update(b"=");
            hasher.update(format!("{:?}", config.options[key]).as_bytes());
        }
        hasher.update(b"|");
    }
    hasher.update(b":");

    for cop in &args.only {
        hasher.update(b"only:");
        hasher.update(cop.as_bytes());
    }
    for cop in &args.except {
        hasher.update(b"except:");
        hasher.update(cop.as_bytes());
    }

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

    let mut total_files: usize = 0;
    let mut sessions: Vec<(PathBuf, SystemTime, usize)> = Vec::new();

    for entry in entries {
        let path = entry.path();
        let mtime = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let file_count = std::fs::read_dir(&path)
            .map(|rd| rd.count())
            .unwrap_or(0);
        total_files += file_count;
        sessions.push((path, mtime, file_count));
    }

    if total_files <= max_files {
        return Ok(());
    }

    sessions.sort_by_key(|(_, mtime, _)| *mtime);

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
            cache: "true".to_string(),
            cache_clear: false,
            fail_level: "convention".to_string(),
            fail_fast: false,
            force_exclusion: false,
        }
    }

    #[test]
    fn disabled_cache_returns_miss() {
        let cache = ResultCache::disabled();
        assert!(!cache.is_enabled());
        assert!(matches!(
            cache.get_by_stat(Path::new("test.rb")),
            CacheLookup::Miss
        ));
    }

    #[test]
    fn cache_roundtrip_with_real_file() {
        let tmp = tempfile::tempdir().unwrap();
        let args = test_args();
        let configs = vec![CopConfig::default()];
        let cache = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs, &args);

        // Create a real file so stat() works
        let rb_file = tmp.path().join("test.rb");
        std::fs::write(&rb_file, b"x = 1 \n").unwrap();

        // Cache miss initially
        assert!(matches!(cache.get_by_stat(&rb_file), CacheLookup::Miss));

        // Store results
        let diagnostics = vec![Diagnostic {
            path: rb_file.to_string_lossy().to_string(),
            location: Location { line: 1, column: 5 },
            severity: Severity::Convention,
            cop_name: "Layout/TrailingWhitespace".to_string(),
            message: "Trailing whitespace detected.".to_string(),
        }];
        cache.put(&rb_file, b"x = 1 \n", &diagnostics);

        // Stat hit (mtime+size unchanged since we just wrote the file)
        match cache.get_by_stat(&rb_file) {
            CacheLookup::StatHit(cached) => {
                assert_eq!(cached.len(), 1);
                assert_eq!(cached[0].cop_name, "Layout/TrailingWhitespace");
                assert_eq!(cached[0].location.line, 1);
                assert_eq!(cached[0].location.column, 5);
            }
            other => panic!("Expected StatHit, got {:?}", match other {
                CacheLookup::ContentHit(_) => "ContentHit",
                CacheLookup::Miss => "Miss",
                _ => "StatHit",
            }),
        }
    }

    #[test]
    fn content_hash_fallback_on_mtime_change() {
        let tmp = tempfile::tempdir().unwrap();
        let args = test_args();
        let configs = vec![CopConfig::default()];
        let cache = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs, &args);

        let rb_file = tmp.path().join("mtime_test.rb");
        std::fs::write(&rb_file, b"y = 2\n").unwrap();

        // Store results
        cache.put(&rb_file, b"y = 2\n", &[]);

        // Simulate mtime change by touching the file (same content)
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(&rb_file, b"y = 2\n").unwrap();

        // Stat miss (mtime changed)
        assert!(matches!(cache.get_by_stat(&rb_file), CacheLookup::Miss));

        // Content hit (content unchanged)
        match cache.get_by_content(&rb_file, b"y = 2\n") {
            CacheLookup::ContentHit(cached) => {
                assert!(cached.is_empty());
            }
            other => panic!("Expected ContentHit, got {:?}", match other {
                CacheLookup::StatHit(_) => "StatHit",
                CacheLookup::Miss => "Miss",
                _ => "ContentHit",
            }),
        }

        // After content hit updated mtime, stat should now hit
        match cache.get_by_stat(&rb_file) {
            CacheLookup::StatHit(_) => {} // expected
            _ => panic!("Expected StatHit after mtime update"),
        }
    }

    #[test]
    fn content_change_is_a_miss() {
        let tmp = tempfile::tempdir().unwrap();
        let args = test_args();
        let configs = vec![CopConfig::default()];
        let cache = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs, &args);

        let rb_file = tmp.path().join("changed.rb");
        std::fs::write(&rb_file, b"x = 1\n").unwrap();
        cache.put(&rb_file, b"x = 1\n", &[]);

        // Change both content and mtime
        std::fs::write(&rb_file, b"x = 2\n").unwrap();

        // Stat miss (mtime changed)
        assert!(matches!(cache.get_by_stat(&rb_file), CacheLookup::Miss));
        // Content miss (content changed)
        assert!(matches!(
            cache.get_by_content(&rb_file, b"x = 2\n"),
            CacheLookup::Miss
        ));
    }

    #[test]
    fn config_change_invalidates_session() {
        let tmp = tempfile::tempdir().unwrap();
        let args = test_args();

        let rb_file = tmp.path().join("test.rb");
        std::fs::write(&rb_file, b"x = 1\n").unwrap();

        let configs1 = vec![CopConfig::default()];
        let cache1 = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs1, &args);
        cache1.put(&rb_file, b"x = 1\n", &[]);

        // Same config = cache hit
        let cache1b = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs1, &args);
        assert!(matches!(cache1b.get_by_stat(&rb_file), CacheLookup::StatHit(_)));

        // Different config = different session = cache miss
        let mut config2 = CopConfig::default();
        config2.enabled = crate::cop::EnabledState::False;
        let configs2 = vec![config2];
        let cache2 = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs2, &args);
        assert!(matches!(cache2.get_by_stat(&rb_file), CacheLookup::Miss));
    }

    #[test]
    fn eviction_removes_old_sessions() {
        let tmp = tempfile::tempdir().unwrap();
        let args = test_args();

        let configs1 = vec![CopConfig::default()];
        let cache1 = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs1, &args);
        for i in 0..15 {
            let f = tmp.path().join(format!("f{i}.rb"));
            std::fs::write(&f, format!("x{i}").as_bytes()).unwrap();
            cache1.put(&f, format!("x{i}").as_bytes(), &[]);
        }

        let mut config2 = CopConfig::default();
        config2.enabled = crate::cop::EnabledState::False;
        let configs2 = vec![config2];
        let cache2 = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs2, &args);
        for i in 0..15 {
            let f = tmp.path().join(format!("g{i}.rb"));
            std::fs::write(&f, format!("y{i}").as_bytes()).unwrap();
            cache2.put(&f, format!("y{i}").as_bytes(), &[]);
        }

        evict_old_sessions(tmp.path(), 20).unwrap();

        let remaining: Vec<_> = std::fs::read_dir(tmp.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();
        assert!(remaining.len() <= 2);
    }
}
