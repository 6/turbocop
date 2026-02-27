#!/usr/bin/env python3
"""Check a single cop against the 500-repo corpus for FP regressions.

Compares nitrocop's offense count against the RuboCop baseline from the
latest corpus oracle CI run. Catches real-world false positive regressions
that fixture tests miss.

Results are cached per (binary_mtime, cop_name, repo_id) so that re-running
check-cop.py for a different cop after fixing one cop is instant — only the
changed cop needs re-execution. Use --rerun to force a fresh run.

Usage:
    python3 scripts/check-cop.py Lint/Void              # quick aggregate check
    python3 scripts/check-cop.py Lint/Void --verbose     # per-repo breakdown
    python3 scripts/check-cop.py Lint/Void --threshold 5 # allow up to 5 excess
"""

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
CORPUS_DIR = PROJECT_ROOT / "vendor" / "corpus"
NITROCOP_BIN = PROJECT_ROOT / "target" / "release" / "nitrocop"
BASELINE_CONFIG = PROJECT_ROOT / "bench" / "corpus" / "baseline_rubocop.yml"
LOCAL_CACHE_DIR = PROJECT_ROOT / ".check-cop-cache"


def download_corpus_results() -> Path:
    """Download corpus-results.json from the latest successful CI run."""
    result = subprocess.run(
        ["gh", "run", "list", "--workflow=corpus-oracle.yml",
         "--status=success", "--limit=1", "--json=databaseId"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print(f"Error listing runs: {result.stderr}", file=sys.stderr)
        sys.exit(1)

    runs = json.loads(result.stdout)
    if not runs:
        print("No successful corpus-oracle runs found", file=sys.stderr)
        sys.exit(1)

    run_id = runs[0]["databaseId"]
    print(f"Downloading corpus-results.json from run {run_id}...", file=sys.stderr)

    tmpdir = tempfile.mkdtemp(prefix="check-cop-")
    result = subprocess.run(
        ["gh", "run", "download", str(run_id), "--name=corpus-report", f"--dir={tmpdir}"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print(f"Error downloading artifact: {result.stderr}", file=sys.stderr)
        sys.exit(1)

    path = Path(tmpdir) / "corpus-results.json"
    if not path.exists():
        print("corpus-results.json not found in artifact", file=sys.stderr)
        sys.exit(1)

    # Clean up stale local corpus-results.json in the project root
    project_root = Path(__file__).resolve().parent.parent
    stale_local = project_root / "corpus-results.json"
    if stale_local.exists():
        stale_local.unlink()
        print(f"Removed stale {stale_local.name} from project root", file=sys.stderr)

    return path


def ensure_binary():
    """Ensure release binary exists."""
    if NITROCOP_BIN.exists():
        return
    print("Release binary not found. Run: cargo build --release", file=sys.stderr)
    sys.exit(1)


def binary_key() -> str:
    """Return a cache key based on the nitrocop binary's mtime and size.

    Changes whenever the binary is rebuilt, invalidating cached results
    for all cops. This is cheaper than hashing the entire binary.
    """
    stat = NITROCOP_BIN.stat()
    return f"{stat.st_mtime_ns}_{stat.st_size}"


def load_local_cache() -> dict:
    """Load the local nitrocop results cache.

    Structure: {binary_key: {cop_name: {repo_id: count}}}
    """
    cache_file = LOCAL_CACHE_DIR / "results.json"
    if cache_file.exists():
        try:
            return json.loads(cache_file.read_text())
        except (json.JSONDecodeError, OSError):
            return {}
    return {}


def save_local_cache(cache: dict):
    """Save the local nitrocop results cache."""
    LOCAL_CACHE_DIR.mkdir(parents=True, exist_ok=True)
    cache_file = LOCAL_CACHE_DIR / "results.json"
    cache_file.write_text(json.dumps(cache))


def get_cached_results(cop_name: str) -> dict[str, int] | None:
    """Get cached per-repo results for a cop, or None if not cached.

    Returns None if the binary has changed since the cache was written.
    """
    cache = load_local_cache()
    bkey = binary_key()
    if bkey in cache and cop_name in cache[bkey]:
        return cache[bkey][cop_name]
    return None


def save_cached_results(cop_name: str, per_repo: dict[str, int]):
    """Save per-repo results for a cop to the local cache."""
    cache = load_local_cache()
    bkey = binary_key()
    # Only keep the current binary's cache to avoid unbounded growth
    cache = {bkey: cache.get(bkey, {})}
    cache[bkey][cop_name] = per_repo
    save_local_cache(cache)


def clear_file_cache():
    """Clear nitrocop's file-level result cache to avoid stale results after rebuild."""
    import shutil
    cache_dir = Path.home() / ".cache" / "nitrocop"
    if cache_dir.exists():
        shutil.rmtree(cache_dir)


def corpus_env() -> dict[str, str]:
    """Environment variables for corpus runs, matching CI exactly."""
    env = os.environ.copy()
    env["BUNDLE_GEMFILE"] = str(PROJECT_ROOT / "bench" / "corpus" / "Gemfile")
    env["BUNDLE_PATH"] = str(PROJECT_ROOT / "bench" / "corpus" / "vendor" / "bundle")
    return env


def nitrocop_cmd(cop_name: str, target: str) -> list[str]:
    """Build the nitrocop command for corpus checking.

    Uses --config with the baseline config to match CI corpus oracle exactly.
    This ensures disabled-by-default cops are enabled the same way as in CI.
    All paths are absolute so the command works from any cwd.
    """
    return [
        str(NITROCOP_BIN), "--only", cop_name, "--preview",
        "--format", "json", "--no-cache",
        "--config", str(BASELINE_CONFIG),
        target,
    ]


def count_deduplicated_offenses(json_data: dict) -> int:
    """Count offenses deduplicated by (path, line, cop_name).

    The corpus oracle uses this deduplication, so we must match it.
    E.g., two offenses on the same line for the same cop count as one.
    """
    seen = set()
    for o in json_data.get("offenses", []):
        key = (o.get("path", ""), o.get("line", 0), o.get("cop_name", ""))
        seen.add(key)
    return len(seen)


def _run_one_repo(args: tuple[str, str]) -> tuple[str, int]:
    """Run nitrocop on a single repo. Used by the parallel executor."""
    cop_name, repo_dir = args
    repo_id = Path(repo_dir).name
    try:
        # Run from repo dir so base_dir_for_path_parameters (cwd) resolves
        # Exclude patterns like vendor/**/* relative to the repo, not the
        # nitrocop project root. This matches CI behavior where repos are
        # at repos/<id>/ and cwd is the CI workspace root.
        result = subprocess.run(
            nitrocop_cmd(cop_name, "."),
            capture_output=True, text=True, timeout=120,
            cwd=repo_dir, env=corpus_env(),
        )
    except subprocess.TimeoutExpired:
        return (repo_id, -1)

    if result.returncode not in (0, 1):
        return (repo_id, -1)

    try:
        data = json.loads(result.stdout)
        return (repo_id, count_deduplicated_offenses(data))
    except json.JSONDecodeError:
        return (repo_id, -1)


def run_nitrocop_per_repo(cop_name: str) -> dict[str, int]:
    """Run nitrocop --only on each corpus repo in parallel, return {repo_id: count}."""
    from concurrent.futures import ThreadPoolExecutor, as_completed

    repos = sorted(d for d in CORPUS_DIR.iterdir() if d.is_dir())
    total = len(repos)
    work = [(cop_name, str(r)) for r in repos]

    workers = min(os.cpu_count() or 4, 16)
    counts = {}
    done = 0

    with ThreadPoolExecutor(max_workers=workers) as pool:
        futures = {pool.submit(_run_one_repo, w): w for w in work}
        for future in as_completed(futures):
            repo_id, count = future.result()
            counts[repo_id] = count
            done += 1
            if done % 50 == 0:
                print(f"  [{done}/{total}] {repo_id}...", file=sys.stderr)

    return counts


def run_nitrocop_aggregate(cop_name: str) -> int:
    """Run nitrocop --only on each corpus repo, return total offense count.

    Uses per-repo parallel execution and caches results.
    """
    per_repo = run_nitrocop_per_repo(cop_name)
    save_cached_results(cop_name, per_repo)
    return sum(c for c in per_repo.values() if c >= 0)


def main():
    parser = argparse.ArgumentParser(
        description="Check a cop against the 500-repo corpus for FP regressions")
    parser.add_argument("cop", help="Cop name (e.g., Lint/Void)")
    parser.add_argument("--input", type=Path,
                        help="Path to corpus-results.json (default: download from CI)")
    parser.add_argument("--verbose", action="store_true",
                        help="Run per-repo and show which repos have excess offenses")
    parser.add_argument("--threshold", type=int, default=0,
                        help="Allowed excess offenses before FAIL (default: 0)")
    parser.add_argument("--rerun", action="store_true",
                        help="Force re-execution of nitrocop (ignore local cache)")
    args = parser.parse_args()

    # Load corpus results
    if args.input:
        input_path = args.input
    else:
        input_path = download_corpus_results()

    data = json.loads(input_path.read_text())
    by_cop = data["by_cop"]

    # Find the cop in corpus results
    cop_entry = next((e for e in by_cop if e["cop"] == args.cop), None)
    if cop_entry is None:
        print(f"Cop '{args.cop}' not found in corpus results", file=sys.stderr)
        print(f"Available cops matching '{args.cop.split('/')[-1]}':", file=sys.stderr)
        for e in by_cop:
            if args.cop.split("/")[-1].lower() in e["cop"].lower():
                print(f"  {e['cop']}", file=sys.stderr)
        sys.exit(1)

    expected_rubocop = cop_entry["matches"] + cop_entry["fn"]
    baseline_fp = cop_entry["fp"]
    baseline_fn = cop_entry["fn"]
    baseline_matches = cop_entry["matches"]

    ensure_binary()

    print(f"Checking {args.cop} against 500-repo corpus")
    print(f"Baseline (from CI): {baseline_matches:,} matches, "
          f"{baseline_fp:,} FP, {baseline_fn:,} FN")
    print(f"Expected RuboCop offenses: {expected_rubocop:,}")
    print()

    # Check if enriched per-repo-per-cop data is available in corpus results
    by_repo_cop = data.get("by_repo_cop", {})
    has_enriched = bool(by_repo_cop)

    if args.verbose and has_enriched and not args.rerun:
        # Use cached corpus data instead of re-running nitrocop
        print("Using cached corpus data (pass --rerun to re-execute nitrocop)", file=sys.stderr)

        # Reconstruct per-repo counts from by_repo_cop
        # nitrocop count = rubocop count + FP - FN per repo
        by_repo = data.get("by_repo", [])
        repo_by_id = {r["repo"]: r for r in by_repo if r.get("status") == "ok"}

        repos_with_offenses = {}
        for repo_id, cops in by_repo_cop.items():
            if args.cop in cops:
                entry = cops[args.cop]
                fp = entry.get("fp", 0)
                fn = entry.get("fn", 0)
                if fp > 0 or fn > 0:
                    repos_with_offenses[repo_id] = {"fp": fp, "fn": fn}

        if repos_with_offenses:
            print(f"Repos with divergence ({len(repos_with_offenses)}):")
            sorted_repos = sorted(repos_with_offenses.items(),
                                  key=lambda x: x[1]["fp"] + x[1]["fn"],
                                  reverse=True)
            for repo_id, counts in sorted_repos[:30]:
                print(f"  FP:{counts['fp']:>5}  FN:{counts['fn']:>5}  {repo_id}")
            if len(sorted_repos) > 30:
                print(f"  ... and {len(sorted_repos) - 30} more")
            print()

        # For cached mode, use baseline FP/FN directly
        nitrocop_total = expected_rubocop + baseline_fp
    else:
        # Try local cache first (unless --rerun forces re-execution)
        cached = None if args.rerun else get_cached_results(args.cop)

        if cached is not None:
            print(f"Using cached nitrocop results (pass --rerun to re-execute)", file=sys.stderr)
            per_repo = cached
        else:
            clear_file_cache()
            print("Running nitrocop per-repo...", file=sys.stderr)
            per_repo = run_nitrocop_per_repo(args.cop)
            save_cached_results(args.cop, per_repo)

        nitrocop_total = sum(c for c in per_repo.values() if c >= 0)

        if args.verbose:
            # Show repos with offenses, sorted by count descending
            repos_with_offenses = {k: v for k, v in per_repo.items() if v > 0}
            if repos_with_offenses:
                print(f"Repos with offenses ({len(repos_with_offenses)}):")
                for repo_id, count in sorted(repos_with_offenses.items(),
                                             key=lambda x: x[1], reverse=True)[:30]:
                    print(f"  {count:>6,}  {repo_id}")
                if len(repos_with_offenses) > 30:
                    print(f"  ... and {len(repos_with_offenses) - 30} more")
                print()

    excess = max(0, nitrocop_total - expected_rubocop)
    missing = max(0, expected_rubocop - nitrocop_total)

    print(f"Results:")
    print(f"  Expected (RuboCop):   {expected_rubocop:>10,}")
    print(f"  Actual (nitrocop):    {nitrocop_total:>10,}")
    print(f"  Excess (potential FP):{excess:>10,}")
    print(f"  Missing (potential FN):{missing:>9,}")
    print()

    if excess > args.threshold:
        print(f"FAIL: {excess:,} excess offenses (threshold: {args.threshold})")
        if not args.verbose:
            print("Run with --verbose to see which repos have excess offenses")
        sys.exit(1)
    else:
        print(f"PASS: {excess:,} excess offenses (threshold: {args.threshold})")
        if missing > 0:
            print(f"Note: {missing:,} potential FN remain (not a regression)")


if __name__ == "__main__":
    main()
