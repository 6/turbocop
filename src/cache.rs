use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

use sha2::{Digest, Sha256};

use crate::cli::Args;
use crate::cop::CopConfig;
use crate::diagnostic::{Diagnostic, Location, Severity};

/// File-level result cache for incremental linting.
///
/// Single-file index layout:
/// ```text
/// <cache_root>/
/// └── <session_hash>.index    # all entries for this session (JSON)
/// ```
///
/// Two-tier lookup per file:
/// 1. **Stat check** (mtime + size) — no file read needed, instant for local dev
/// 2. **Content hash** fallback — handles CI, git checkout, and other mtime-unreliable scenarios
///
/// Thread safety: all entries stored in a `RwLock<HashMap>`. Rayon workers take
/// read locks on stat hits (zero contention), write locks only on cache misses.
pub struct ResultCache {
    index_path: PathBuf,
    enabled: bool,
    /// All cache entries loaded into memory at construction.
    /// Key: path_hash (String), Value: CacheEntry
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// Track whether any entries were added/updated (need flush on drop).
    dirty: Arc<AtomicBool>,
}

/// Full cache entry stored in the index: metadata + diagnostics.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
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
#[derive(serde::Serialize, serde::Deserialize, Clone)]
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

    fn to_diagnostic(&self, path: &str) -> Diagnostic {
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
            cop_name: self.cop.clone(),
            message: self.message.clone(),
            corrected: false,
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
        let _ = std::fs::create_dir_all(&cache_root);
        let session_hash = compute_session_hash(version, base_configs, args);
        let index_path = cache_root.join(format!("{session_hash}.index"));
        let entries = load_index(&index_path);
        Self {
            index_path,
            enabled: true,
            entries: Arc::new(RwLock::new(entries)),
            dirty: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create a cache rooted at the given directory (for testing).
    pub fn with_root(root: &Path, version: &str, base_configs: &[CopConfig], args: &Args) -> Self {
        let session_hash = compute_session_hash(version, base_configs, args);
        let index_path = root.join(format!("{session_hash}.index"));
        let entries = load_index(&index_path);
        Self {
            index_path,
            enabled: true,
            entries: Arc::new(RwLock::new(entries)),
            dirty: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create a disabled (no-op) cache.
    pub fn disabled() -> Self {
        Self {
            index_path: PathBuf::new(),
            enabled: false,
            entries: Arc::new(RwLock::new(HashMap::new())),
            dirty: Arc::new(AtomicBool::new(false)),
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

        let hash = compute_path_hash(path);
        let entries = self.entries.read().unwrap();
        let entry = match entries.get(&hash) {
            Some(e) => e,
            None => return CacheLookup::Miss,
        };

        let (mtime_secs, mtime_nanos) = systemtime_to_parts(meta.modified().ok());
        let size = meta.len();

        if entry.mtime_secs == mtime_secs && entry.mtime_nanos == mtime_nanos && entry.size == size
        {
            let path_str = path.to_string_lossy();
            CacheLookup::StatHit(
                entry
                    .diagnostics
                    .iter()
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

        let hash = compute_path_hash(path);
        let content_hash = compute_content_hash(content);

        // Read lock to check if content hash matches
        {
            let entries = self.entries.read().unwrap();
            let entry = match entries.get(&hash) {
                Some(e) => e,
                None => return CacheLookup::Miss,
            };

            if entry.content_hash != content_hash {
                return CacheLookup::Miss;
            }
        }

        // Content matched — update mtime+size with a write lock
        let meta = std::fs::metadata(path).ok();
        let (mtime_secs, mtime_nanos) =
            systemtime_to_parts(meta.as_ref().and_then(|m| m.modified().ok()));
        let size = meta.map(|m| m.len()).unwrap_or(content.len() as u64);

        let mut entries = self.entries.write().unwrap();
        let entry = match entries.get_mut(&hash) {
            Some(e) => e,
            None => return CacheLookup::Miss,
        };

        // Build result before mutating the entry
        let path_str = path.to_string_lossy();
        let result: Vec<Diagnostic> = entry
            .diagnostics
            .iter()
            .map(|e| e.to_diagnostic(&path_str))
            .collect();

        entry.mtime_secs = mtime_secs;
        entry.mtime_nanos = mtime_nanos;
        entry.size = size;
        self.dirty.store(true, Ordering::Relaxed);

        CacheLookup::ContentHit(result)
    }

    /// Store results for a file. Best-effort — writes to in-memory map only.
    /// Call `flush()` to persist to disk.
    pub fn put(&self, path: &Path, content: &[u8], diagnostics: &[Diagnostic]) {
        if !self.enabled {
            return;
        }

        let meta = std::fs::metadata(path).ok();
        let (mtime_secs, mtime_nanos) =
            systemtime_to_parts(meta.as_ref().and_then(|m| m.modified().ok()));
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

        let hash = compute_path_hash(path);
        let mut entries = self.entries.write().unwrap();
        entries.insert(hash, entry);
        self.dirty.store(true, Ordering::Relaxed);
    }

    /// Persist the in-memory cache to disk if any entries were added/updated.
    /// Uses atomic write (temp file + rename) to avoid corruption.
    pub fn flush(&self) {
        if !self.enabled || !self.dirty.load(Ordering::Relaxed) {
            return;
        }

        let entries = self.entries.read().unwrap();
        let json = match serde_json::to_vec(&*entries) {
            Ok(j) => j,
            Err(_) => return,
        };
        drop(entries);

        let tmp_path = self.index_path.with_extension("tmp");
        if std::fs::write(&tmp_path, &json).is_ok() {
            let _ = std::fs::rename(&tmp_path, &self.index_path);
        }
    }

    /// Evict old session index files when total sessions exceed the limit.
    pub fn evict(&self, max_sessions: usize) {
        if !self.enabled {
            return;
        }
        let cache_root = cache_root_dir();
        let _ = evict_old_sessions(&cache_root, max_sessions);
    }
}

/// Load the index file into a HashMap. Returns empty map if file doesn't exist
/// or can't be parsed.
fn load_index(index_path: &Path) -> HashMap<String, CacheEntry> {
    let data = match std::fs::read(index_path) {
        Ok(d) => d,
        Err(_) => return HashMap::new(),
    };
    serde_json::from_slice(&data).unwrap_or_default()
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

/// Compute a stable hash of just the file path (used as cache key).
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
pub(crate) fn cache_root_dir() -> PathBuf {
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
    hasher.update(b"turbocop-session-v3:");
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
    if args.ignore_disable_comments {
        hasher.update(b"ignore_disable_comments");
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

/// Evict old session index files when total count exceeds max_sessions.
///
/// Counts `.index` files in the cache root. When the count exceeds the limit,
/// removes the oldest sessions (by mtime) until count drops to half the limit.
/// Also cleans up any leftover old-format session directories (from v2 layout).
fn evict_old_sessions(cache_root: &Path, max_sessions: usize) -> std::io::Result<()> {
    let dir_entries: Vec<_> = std::fs::read_dir(cache_root)?
        .filter_map(|e| e.ok())
        .collect();

    // Clean up leftover old-format session directories (not "lockfiles")
    for entry in &dir_entries {
        if entry.path().is_dir() && entry.file_name() != "lockfiles" {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }

    let mut sessions: Vec<(PathBuf, SystemTime)> = dir_entries
        .iter()
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "index"))
        .map(|e| {
            let mtime = e
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            (e.path(), mtime)
        })
        .collect();

    if sessions.len() <= max_sessions {
        return Ok(());
    }

    sessions.sort_by_key(|(_, mtime)| *mtime);

    let target = std::cmp::max(max_sessions / 2, 1);
    let mut remaining = sessions.len();
    for (path, _) in &sessions {
        if remaining <= target {
            break;
        }
        let _ = std::fs::remove_file(path);
        remaining -= 1;
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
            list_autocorrectable_cops: false,
            migrate: false,
            doctor: false,
            rules: false,
            tier: None,
            stdin: None,
            init: false,
            no_cache: false,
            cache: "true".to_string(),
            cache_clear: false,
            fail_level: "convention".to_string(),
            fail_fast: false,
            force_exclusion: false,
            list_target_files: false,
            display_cop_names: false,
            parallel: false,
            require_libs: vec![],
            ignore_disable_comments: false,
            force_default_config: false,
            autocorrect: false,
            autocorrect_all: false,
            preview: false,
            quiet_skips: false,
            strict: None,
            verify: false,
            rubocop_cmd: "bundle exec rubocop".to_string(),
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
            corrected: false,
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
            other => panic!(
                "Expected StatHit, got {:?}",
                match other {
                    CacheLookup::ContentHit(_) => "ContentHit",
                    CacheLookup::Miss => "Miss",
                    _ => "StatHit",
                }
            ),
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
            other => panic!(
                "Expected ContentHit, got {:?}",
                match other {
                    CacheLookup::StatHit(_) => "StatHit",
                    CacheLookup::Miss => "Miss",
                    _ => "ContentHit",
                }
            ),
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

        // Change content with a different size so stat detects the change even
        // when both writes land in the same filesystem timestamp granularity
        std::fs::write(&rb_file, b"x = 22\n").unwrap();

        // Stat miss (size changed)
        assert!(matches!(cache.get_by_stat(&rb_file), CacheLookup::Miss));
        // Content miss (content changed)
        assert!(matches!(
            cache.get_by_content(&rb_file, b"x = 22\n"),
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

        // Same config = cache hit (same in-memory instance)
        assert!(matches!(
            cache1.get_by_stat(&rb_file),
            CacheLookup::StatHit(_)
        ));

        // Flush and reload — should still hit
        cache1.flush();
        let cache1b = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs1, &args);
        assert!(matches!(
            cache1b.get_by_stat(&rb_file),
            CacheLookup::StatHit(_)
        ));

        // Different config = different session = cache miss
        let mut config2 = CopConfig::default();
        config2.enabled = crate::cop::EnabledState::False;
        let configs2 = vec![config2];
        let cache2 = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs2, &args);
        assert!(matches!(cache2.get_by_stat(&rb_file), CacheLookup::Miss));
    }

    #[test]
    fn flush_and_reload_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let args = test_args();
        let configs = vec![CopConfig::default()];

        let rb_file = tmp.path().join("persist.rb");
        std::fs::write(&rb_file, b"z = 3\n").unwrap();

        // Populate and flush
        let cache1 = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs, &args);
        let diagnostics = vec![Diagnostic {
            path: rb_file.to_string_lossy().to_string(),
            location: Location { line: 1, column: 0 },
            severity: Severity::Warning,
            cop_name: "Lint/UselessAssignment".to_string(),
            message: "Useless assignment.".to_string(),
            corrected: false,
        }];
        cache1.put(&rb_file, b"z = 3\n", &diagnostics);
        cache1.flush();

        // Verify index file exists
        let session_hash = compute_session_hash("0.1.0-test", &configs, &args);
        let index_path = tmp.path().join(format!("{session_hash}.index"));
        assert!(index_path.exists(), "index file should exist after flush");

        // Reload from disk
        let cache2 = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs, &args);
        match cache2.get_by_stat(&rb_file) {
            CacheLookup::StatHit(cached) => {
                assert_eq!(cached.len(), 1);
                assert_eq!(cached[0].cop_name, "Lint/UselessAssignment");
            }
            _ => panic!("Expected StatHit after reload"),
        }
    }

    #[test]
    fn no_flush_when_not_dirty() {
        let tmp = tempfile::tempdir().unwrap();
        let args = test_args();
        let configs = vec![CopConfig::default()];

        // Create cache, don't put anything
        let cache = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs, &args);
        cache.flush();

        // No index file should be written
        let session_hash = compute_session_hash("0.1.0-test", &configs, &args);
        let index_path = tmp.path().join(format!("{session_hash}.index"));
        assert!(
            !index_path.exists(),
            "index file should not exist when nothing was cached"
        );
    }

    #[test]
    fn eviction_removes_old_sessions() {
        let tmp = tempfile::tempdir().unwrap();
        let args = test_args();

        // Create session 1 and flush
        let configs1 = vec![CopConfig::default()];
        let cache1 = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs1, &args);
        let f = tmp.path().join("f0.rb");
        std::fs::write(&f, b"x0").unwrap();
        cache1.put(&f, b"x0", &[]);
        cache1.flush();

        // Small delay so mtimes differ
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Create session 2 and flush
        let mut config2 = CopConfig::default();
        config2.enabled = crate::cop::EnabledState::False;
        let configs2 = vec![config2];
        let cache2 = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs2, &args);
        let g = tmp.path().join("g0.rb");
        std::fs::write(&g, b"y0").unwrap();
        cache2.put(&g, b"y0", &[]);
        cache2.flush();

        // Both index files should exist
        let index_count = || {
            std::fs::read_dir(tmp.path())
                .unwrap()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "index"))
                .count()
        };
        assert_eq!(index_count(), 2);

        // Evict with max_sessions=1 — should remove the oldest, keeping 1
        evict_old_sessions(tmp.path(), 1).unwrap();
        assert_eq!(index_count(), 1);
    }

    #[test]
    fn eviction_cleans_up_old_format_directories() {
        let tmp = tempfile::tempdir().unwrap();

        // Create an old-format session directory (simulating v2 layout)
        let old_session_dir = tmp.path().join("abcdef1234567890");
        std::fs::create_dir_all(&old_session_dir).unwrap();
        std::fs::write(old_session_dir.join("somefile"), b"data").unwrap();

        // Create a lockfiles directory (should be preserved)
        let lockfiles_dir = tmp.path().join("lockfiles");
        std::fs::create_dir_all(&lockfiles_dir).unwrap();
        std::fs::write(lockfiles_dir.join("lock.json"), b"{}").unwrap();

        // Run eviction
        evict_old_sessions(tmp.path(), 100).unwrap();

        // Old session dir should be removed
        assert!(
            !old_session_dir.exists(),
            "old-format session directory should be cleaned up"
        );
        // lockfiles directory should be preserved
        assert!(
            lockfiles_dir.exists(),
            "lockfiles directory should be preserved"
        );
    }
}
