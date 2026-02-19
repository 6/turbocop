use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Serialize, Deserialize)]
pub struct RblintLock {
    pub version: u32,
    pub generated_at: String,
    /// SHA-256 of Gemfile.lock content (for staleness detection)
    pub gemfile_lock_sha256: Option<String>,
    /// Gem name → absolute path to gem root directory
    pub gems: HashMap<String, PathBuf>,
}

/// Write `.rblint.lock` to the given directory.
pub fn write_lock(gems: &HashMap<String, PathBuf>, dir: &Path) -> Result<()> {
    let gemfile_lock_sha256 = hash_file(&dir.join("Gemfile.lock"));

    let lock = RblintLock {
        version: 1,
        generated_at: chrono_now(),
        gemfile_lock_sha256,
        gems: gems.clone(),
    };

    let json = serde_json::to_string_pretty(&lock)?;
    let lock_path = dir.join(".rblint.lock");
    std::fs::write(&lock_path, json)
        .with_context(|| format!("Failed to write {}", lock_path.display()))?;
    Ok(())
}

/// Read and parse `.rblint.lock` from the given directory.
/// Returns an error if the file is missing.
pub fn read_lock(dir: &Path) -> Result<RblintLock> {
    let lock_path = dir.join(".rblint.lock");
    if !lock_path.exists() {
        anyhow::bail!(
            "No .rblint.lock found in {}. Run 'rblint --init' first.",
            dir.display()
        );
    }
    let content = std::fs::read_to_string(&lock_path)
        .with_context(|| format!("Failed to read {}", lock_path.display()))?;
    let lock: RblintLock = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse {}", lock_path.display()))?;
    if lock.version != 1 {
        anyhow::bail!(
            ".rblint.lock has version {} (expected 1). Run 'rblint --init' to regenerate.",
            lock.version
        );
    }
    Ok(lock)
}

/// Check that the lockfile is still fresh (Gemfile.lock hasn't changed).
/// Returns an error if stale.
pub fn check_freshness(lock: &RblintLock, dir: &Path) -> Result<()> {
    let current_hash = hash_file(&dir.join("Gemfile.lock"));
    if lock.gemfile_lock_sha256 != current_hash {
        anyhow::bail!(
            "Stale .rblint.lock (Gemfile.lock changed). Run 'rblint --init' to refresh."
        );
    }
    Ok(())
}

/// SHA-256 hash of a file's content, or None if the file doesn't exist.
fn hash_file(path: &Path) -> Option<String> {
    let content = std::fs::read(path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Some(format!("{:x}", hasher.finalize()))
}

/// Simple ISO-8601 timestamp without pulling in chrono.
fn chrono_now() -> String {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    // Rough UTC timestamp (good enough for a lockfile)
    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;

    // Days since epoch to Y-M-D (simplified)
    let (year, month, day) = days_to_ymd(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z"
    )
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    // Simplified Gregorian calendar conversion
    let mut year = 1970;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let month_days: [u64; 12] = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}
