"""RuboCop result cache for corpus validation scripts.

Caches RuboCop JSON output keyed by (file_content_hash, cop_name,
config_hash, rubocop_version) so repeated runs with identical inputs
skip the subprocess invocation entirely.

Cache location: ~/.cache/nitrocop/rubocop-results/
Each entry is a JSON file named by its cache key hash.
"""
from __future__ import annotations

import hashlib
import json
import os
import subprocess
import tempfile
from pathlib import Path

CACHE_DIR = Path.home() / ".cache" / "nitrocop" / "rubocop-results"

_rubocop_version: str | None = None


def _get_rubocop_version(env: dict[str, str] | None = None) -> str:
    """Detect the installed RuboCop version from the corpus bundle.

    Cached after the first call within a process.
    """
    global _rubocop_version
    if _rubocop_version is not None:
        return _rubocop_version

    try:
        result = subprocess.run(
            ["bundle", "exec", "rubocop", "--version"],
            capture_output=True, text=True, timeout=15,
            env=env,
        )
        if result.returncode == 0:
            _rubocop_version = result.stdout.strip()
            return _rubocop_version
    except (subprocess.TimeoutExpired, FileNotFoundError):
        pass

    _rubocop_version = "unknown"
    return _rubocop_version


def _hash_content(content: str) -> str:
    """SHA-256 hex digest of string content."""
    return hashlib.sha256(content.encode()).hexdigest()


def _config_hash(config_path: str) -> str:
    """SHA-256 of the config file content. Cached per path within process."""
    if not hasattr(_config_hash, "_cache"):
        _config_hash._cache = {}  # type: ignore[attr-defined]
    cache = _config_hash._cache  # type: ignore[attr-defined]
    if config_path not in cache:
        try:
            content = Path(config_path).read_text()
            cache[config_path] = _hash_content(content)
        except OSError:
            cache[config_path] = "missing"
    return cache[config_path]


def _cache_key(file_content: str, cop_name: str, config_path: str, rubocop_version: str) -> str:
    """Compute the cache key hash from all inputs."""
    content_hash = _hash_content(file_content)
    cfg_hash = _config_hash(config_path)
    key_str = f"{content_hash}:{cop_name}:{cfg_hash}:{rubocop_version}"
    return hashlib.sha256(key_str.encode()).hexdigest()


def _cache_path(key: str) -> Path:
    """Return the file path for a cache entry."""
    # Use 2-char prefix directory to avoid too many files in one dir
    return CACHE_DIR / key[:2] / f"{key}.json"


def get_cached(file_content: str, cop_name: str, config_path: str,
               env: dict[str, str] | None = None) -> dict | None:
    """Look up a cached RuboCop result.

    Returns the cached JSON dict, or None on cache miss.
    """
    version = _get_rubocop_version(env)
    key = _cache_key(file_content, cop_name, config_path, version)
    path = _cache_path(key)

    if not path.exists():
        return None

    try:
        return json.loads(path.read_text())
    except (json.JSONDecodeError, OSError):
        # Corrupted cache entry — treat as miss
        try:
            path.unlink(missing_ok=True)
        except OSError:
            pass
        return None


def put_cached(file_content: str, cop_name: str, config_path: str,
               result: dict, env: dict[str, str] | None = None) -> None:
    """Store a RuboCop result in the cache.

    Uses atomic write (temp file + rename) for safe concurrent access.
    """
    version = _get_rubocop_version(env)
    key = _cache_key(file_content, cop_name, config_path, version)
    path = _cache_path(key)

    path.parent.mkdir(parents=True, exist_ok=True)

    # Atomic write: write to temp file in same dir, then rename
    try:
        fd, tmp_path = tempfile.mkstemp(dir=str(path.parent), suffix=".tmp")
        try:
            with os.fdopen(fd, "w") as f:
                json.dump(result, f, separators=(",", ":"))
            os.rename(tmp_path, str(path))
        except Exception:
            try:
                os.unlink(tmp_path)
            except OSError:
                pass
            raise
    except OSError:
        pass  # Cache write failure is non-fatal


def cached_rubocop_run(
    cmd: list[str],
    file_content: str,
    cop_name: str,
    config_path: str,
    env: dict[str, str] | None = None,
    timeout: int = 30,
) -> dict | None:
    """Run RuboCop with caching. Returns parsed JSON output or None on error.

    Args:
        cmd: Full rubocop command (e.g., ["bundle", "exec", "rubocop", ...])
        file_content: Content of the file being linted (used for cache key)
        cop_name: The cop being checked
        config_path: Path to the rubocop config file
        env: Environment variables for subprocess
        timeout: Subprocess timeout in seconds

    Returns:
        Parsed JSON dict from rubocop --format json, or None on error.
    """
    # Check cache first
    cached = get_cached(file_content, cop_name, config_path, env)
    if cached is not None:
        return cached

    # Cache miss — run RuboCop
    try:
        result = subprocess.run(
            cmd, capture_output=True, text=True, timeout=timeout, env=env,
        )
    except subprocess.TimeoutExpired:
        return None

    if result.returncode not in (0, 1):
        return None

    try:
        data = json.loads(result.stdout)
    except json.JSONDecodeError:
        return None

    # Store in cache
    put_cached(file_content, cop_name, config_path, data, env)
    return data
