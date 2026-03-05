#!/usr/bin/env python3
from __future__ import annotations
"""Shared helper to download corpus-results.json from CI.

Tries multiple strategies in order:
1. `gh` CLI (if installed and authenticated)
2. `curl` with GH_TOKEN env var (for environments without `gh`)
3. Helpful error message with instructions

All scripts that need corpus-results.json should use this module instead of
duplicating the download logic.

Usage:
    from corpus_download import download_corpus_results

    # Returns (path, run_id, head_sha) or (path, run_id, "") depending on variant
    path, run_id, head_sha = download_corpus_results()
"""

import io
import json
import os
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path
from urllib.request import Request, urlopen
from urllib.error import HTTPError, URLError


def _find_project_root() -> Path:
    """Find the git repo root."""
    result = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        capture_output=True, text=True,
    )
    if result.returncode == 0:
        return Path(result.stdout.strip())
    return Path(__file__).resolve().parent.parent


def _detect_github_repo() -> str | None:
    """Detect the GitHub owner/repo from git remote origin URL.

    Supports formats:
        https://github.com/owner/repo.git
        git@github.com:owner/repo.git
        http://proxy@host:port/git/owner/repo
    """
    result = subprocess.run(
        ["git", "remote", "get-url", "origin"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        return None

    url = result.stdout.strip()

    # http(s)://...github.com/owner/repo or proxy format /git/owner/repo
    import re
    # Standard GitHub URL
    m = re.search(r"github\.com[/:]([^/]+/[^/.]+?)(?:\.git)?$", url)
    if m:
        return m.group(1)

    # Local proxy format: http://proxy@host:port/git/owner/repo
    m = re.search(r"/git/([^/]+/[^/]+?)(?:\.git)?$", url)
    if m:
        return m.group(1)

    return None


def _clean_stale_local(project_root: Path) -> None:
    """Remove stale corpus-results.json from the project root."""
    stale = project_root / "corpus-results.json"
    if stale.exists():
        stale.unlink()
        print(f"Removed stale {stale.name} from project root", file=sys.stderr)


def _cache_dir() -> Path:
    """Get the cache directory for downloaded corpus results."""
    d = Path(tempfile.gettempdir()) / "nitrocop-corpus-cache"
    d.mkdir(parents=True, exist_ok=True)
    return d


def _try_gh(repo: str | None) -> tuple[Path, int, str] | None:
    """Try downloading via gh CLI. Returns (path, run_id, head_sha) or None."""
    if not shutil.which("gh"):
        return None

    # Check if gh is authenticated
    auth_check = subprocess.run(
        ["gh", "auth", "status"],
        capture_output=True, text=True,
    )
    if auth_check.returncode != 0:
        print("gh CLI found but not authenticated, trying other methods...", file=sys.stderr)
        return None

    # Build the command with optional repo flag
    repo_args = ["-R", repo] if repo else []

    result = subprocess.run(
        ["gh", "run", "list", *repo_args, "--workflow=corpus-oracle.yml",
         "--status=success", "--limit=1", "--json=databaseId,headSha"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        return None

    runs = json.loads(result.stdout)
    if not runs:
        return None

    run_id = runs[0]["databaseId"]
    head_sha = runs[0].get("headSha", "")

    # Check cache
    cache_path = _cache_dir() / f"corpus-results-{run_id}.json"
    if cache_path.exists():
        print(f"Using cached corpus-results from run {run_id}", file=sys.stderr)
        return cache_path, run_id, head_sha

    print(f"Downloading corpus-report from run {run_id} via gh...", file=sys.stderr)

    tmpdir = tempfile.mkdtemp(prefix="corpus-dl-")
    result = subprocess.run(
        ["gh", "run", "download", *repo_args, str(run_id),
         "--name=corpus-report", f"--dir={tmpdir}"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        shutil.rmtree(tmpdir, ignore_errors=True)
        return None

    path = Path(tmpdir) / "corpus-results.json"
    if not path.exists():
        shutil.rmtree(tmpdir, ignore_errors=True)
        return None

    # Cache for next time
    shutil.copy2(path, cache_path)
    return cache_path, run_id, head_sha


def _github_api_get(url: str, token: str | None = None) -> dict:
    """Make a GET request to the GitHub API."""
    headers = {"Accept": "application/vnd.github+json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    req = Request(url, headers=headers)
    with urlopen(req, timeout=30) as resp:
        return json.loads(resp.read())


def _github_api_download(url: str, token: str) -> bytes:
    """Download binary content from the GitHub API (follows redirects)."""
    headers = {
        "Accept": "application/vnd.github+json",
        "Authorization": f"Bearer {token}",
    }
    req = Request(url, headers=headers)
    with urlopen(req, timeout=120) as resp:
        return resp.read()


def _try_curl_api(repo: str | None) -> tuple[Path, int, str] | None:
    """Try downloading via GitHub REST API with GH_TOKEN env var."""
    token = os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN")
    if not repo:
        return None

    # Step 1: List runs (works without auth for public repos)
    api_base = f"https://api.github.com/repos/{repo}"
    try:
        data = _github_api_get(
            f"{api_base}/actions/workflows/corpus-oracle.yml/runs?status=success&per_page=1"
        )
    except (HTTPError, URLError) as e:
        print(f"GitHub API error listing runs: {e}", file=sys.stderr)
        return None

    runs = data.get("workflow_runs", [])
    if not runs:
        return None

    run_id = runs[0]["id"]
    head_sha = runs[0].get("head_sha", "")

    # Check cache
    cache_path = _cache_dir() / f"corpus-results-{run_id}.json"
    if cache_path.exists():
        print(f"Using cached corpus-results from run {run_id}", file=sys.stderr)
        return cache_path, run_id, head_sha

    # Step 2: Find the corpus-report artifact
    if not token:
        # Can list artifacts without auth, but can't download
        print("Found corpus oracle run but need a token to download artifacts.", file=sys.stderr)
        print("Set GH_TOKEN or GITHUB_TOKEN env var, or run: gh auth login", file=sys.stderr)
        return None

    try:
        artifacts_data = _github_api_get(
            f"{api_base}/actions/runs/{run_id}/artifacts", token
        )
    except (HTTPError, URLError) as e:
        print(f"GitHub API error listing artifacts: {e}", file=sys.stderr)
        return None

    artifact_id = None
    for a in artifacts_data.get("artifacts", []):
        if a["name"] == "corpus-report":
            artifact_id = a["id"]
            break

    if not artifact_id:
        print("corpus-report artifact not found in run", file=sys.stderr)
        return None

    # Step 3: Download the artifact zip
    print(f"Downloading corpus-report from run {run_id} via API...", file=sys.stderr)
    try:
        zip_bytes = _github_api_download(
            f"{api_base}/actions/artifacts/{artifact_id}/zip", token
        )
    except (HTTPError, URLError) as e:
        print(f"GitHub API error downloading artifact: {e}", file=sys.stderr)
        return None

    # Step 4: Extract corpus-results.json from the zip
    try:
        with zipfile.ZipFile(io.BytesIO(zip_bytes)) as zf:
            if "corpus-results.json" not in zf.namelist():
                print("corpus-results.json not found in artifact zip", file=sys.stderr)
                return None
            with zf.open("corpus-results.json") as f:
                cache_path.write_bytes(f.read())
    except zipfile.BadZipFile:
        print("Downloaded artifact is not a valid zip file", file=sys.stderr)
        return None

    return cache_path, run_id, head_sha


def download_corpus_results(
    *, include_head_sha: bool = True
) -> tuple[Path, int, str]:
    """Download corpus-results.json from the latest successful corpus-oracle CI run.

    Tries gh CLI first, then falls back to GitHub REST API with token.

    Returns (path_to_json, run_id, head_sha).
    If include_head_sha is False, head_sha may be empty.
    """
    project_root = _find_project_root()
    repo = _detect_github_repo()

    # Try gh first
    result = _try_gh(repo)
    if result:
        _clean_stale_local(project_root)
        return result

    # Try curl/API fallback
    result = _try_curl_api(repo)
    if result:
        _clean_stale_local(project_root)
        return result

    # Nothing worked — give a helpful error
    print("\nFailed to download corpus-results.json.", file=sys.stderr)
    print("", file=sys.stderr)
    print("Options:", file=sys.stderr)
    print("  1. Install and authenticate gh: gh auth login", file=sys.stderr)
    print("  2. Set GH_TOKEN or GITHUB_TOKEN env var", file=sys.stderr)
    print("  3. Download manually and pass --input corpus-results.json", file=sys.stderr)
    if repo:
        print(f"  4. Visit https://github.com/{repo}/actions and download the", file=sys.stderr)
        print("     corpus-report artifact from the latest corpus-oracle run", file=sys.stderr)
    sys.exit(1)
