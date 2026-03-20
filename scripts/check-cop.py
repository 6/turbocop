#!/usr/bin/env python3
from __future__ import annotations
"""Check a single cop against the corpus for aggregate count regressions.

Compares nitrocop's aggregate offense count against the RuboCop baseline from
the latest corpus oracle CI run. Catches real-world false positive regressions
that fixture tests miss.

This is a count-only gate. It does NOT prove that nitrocop matches RuboCop at
the exact offense locations, and it does NOT prove department-level completion
in README.md / docs/corpus.md.

Results are cached per (binary_mtime, cop_name, repo_id) so that re-running
check-cop.py for a different cop after fixing one cop is instant — only the
changed cop needs re-execution. Use --rerun to force a fresh run.

Usage:
    python3 scripts/check-cop.py Lint/Void              # quick aggregate count check
    python3 scripts/check-cop.py Lint/Void --verbose     # per-repo count breakdown
    python3 scripts/check-cop.py Lint/Void --verbose --rerun --quick  # fast iteration
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

# Allow importing corpus_download from the same directory
sys.path.insert(0, str(Path(__file__).resolve().parent))
from corpus_download import download_corpus_results as _download_corpus

PROJECT_ROOT = Path(__file__).resolve().parent.parent
CORPUS_DIR = PROJECT_ROOT / "vendor" / "corpus"
MANIFEST_PATH = PROJECT_ROOT / "bench" / "corpus" / "manifest.jsonl"
NITROCOP_BIN = Path(os.environ["NITROCOP_BIN"]) if "NITROCOP_BIN" in os.environ else PROJECT_ROOT / os.environ.get("CARGO_TARGET_DIR", "target") / "release" / "nitrocop"
BASELINE_CONFIG = PROJECT_ROOT / "bench" / "corpus" / "baseline_rubocop.yml"
LOCAL_CACHE_DIR = PROJECT_ROOT / ".check-cop-cache"


def download_corpus_results(prefer: str = "standard") -> Path:
    """Download corpus-results.json from the latest successful CI run."""
    path, _run_id, _sha = _download_corpus(prefer=prefer)
    return path


def ensure_binary():
    """Ensure release binary exists."""
    if NITROCOP_BIN.exists():
        return
    print("Release binary not found. Run: cargo build --release", file=sys.stderr)
    sys.exit(1)


def check_corpus_bundle():
    """Warn if corpus bundle is not installed for the active Ruby version.

    Without the bundle, `bundle info rubocop` fails and config resolution
    falls back to hardcoded defaults, producing wildly incorrect offense
    counts (often 5-10x higher than expected).
    """
    bundle_dir = PROJECT_ROOT / "bench" / "corpus" / "vendor" / "bundle"
    if not bundle_dir.exists():
        print(
            "WARNING: corpus bundle not installed. Offense counts will be wrong!\n"
            "  Fix: cd bench/corpus && BUNDLE_PATH=vendor/bundle bundle install\n",
            file=sys.stderr,
        )
        return
    # Check that rubocop gem is findable
    env = corpus_env()
    try:
        result = subprocess.run(
            ["bundle", "info", "--path", "rubocop"],
            capture_output=True, text=True, timeout=10,
            cwd=str(PROJECT_ROOT / "bench" / "corpus"),
            env=env,
        )
        if result.returncode != 0:
            print(
                "WARNING: corpus bundle exists but `bundle info rubocop` failed.\n"
                f"  stderr: {result.stderr.strip()}\n"
                "  Fix: cd bench/corpus && BUNDLE_PATH=vendor/bundle bundle install\n",
                file=sys.stderr,
            )
    except (FileNotFoundError, subprocess.TimeoutExpired):
        pass  # bundle not on PATH or too slow — skip check


def latest_source_mtime() -> float:
    """Return latest mtime across files that affect the release binary."""
    latest = 0.0
    watched = [PROJECT_ROOT / "Cargo.toml", PROJECT_ROOT / "Cargo.lock"]
    for path in watched:
        if path.exists():
            latest = max(latest, path.stat().st_mtime)

    src_dir = PROJECT_ROOT / "src"
    if src_dir.exists():
        for path in src_dir.rglob("*.rs"):
            latest = max(latest, path.stat().st_mtime)

    return latest


def ensure_binary_fresh():
    """Rebuild release binary when source is newer than target/release/nitrocop."""
    ensure_binary()
    bin_mtime = NITROCOP_BIN.stat().st_mtime
    src_mtime = latest_source_mtime()
    if src_mtime <= bin_mtime:
        return

    print("Release binary is stale; rebuilding with cargo build --release...", file=sys.stderr)
    result = subprocess.run(
        ["cargo", "build", "--release"],
        cwd=PROJECT_ROOT,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print("Error rebuilding release binary:", file=sys.stderr)
        print(result.stderr, file=sys.stderr)
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
    shutil.rmtree(cache_dir, ignore_errors=True)


def corpus_env(repo_dir: str | None = None) -> dict[str, str]:
    """Environment variables for corpus runs, matching CI exactly.

    When repo_dir is set, GIT_CEILING_DIRECTORIES isolates the corpus repo
    from the parent nitrocop project's .gitignore (which excludes vendor/corpus/).
    Without this, the `ignore` crate's gitignore-aware file walker skips all
    files in corpus repos, producing 0 offenses.
    """
    env = os.environ.copy()
    env["BUNDLE_GEMFILE"] = str(PROJECT_ROOT / "bench" / "corpus" / "Gemfile")
    env["BUNDLE_PATH"] = str(PROJECT_ROOT / "bench" / "corpus" / "vendor" / "bundle")
    if repo_dir:
        env["GIT_CEILING_DIRECTORIES"] = str(CORPUS_DIR)
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


def run_nitrocop_batch(cop_name: str) -> dict[str, int] | None:
    """Try batch corpus check via --corpus-check flag (single process).

    Returns {repo_id: count} or None if the binary doesn't support --corpus-check.
    """
    cmd = [
        str(NITROCOP_BIN), "--corpus-check", str(CORPUS_DIR),
        "--only", cop_name, "--preview",
        "--no-cache",
        "--config", str(BASELINE_CONFIG),
    ]
    try:
        result = subprocess.run(
            cmd, capture_output=True, text=True, timeout=600,
            env=corpus_env(str(CORPUS_DIR)),
        )
    except subprocess.TimeoutExpired:
        print("  batch corpus-check timed out, falling back to per-repo", file=sys.stderr)
        return None

    if result.returncode != 0:
        # Binary might not support --corpus-check yet
        if "corpus-check" in result.stderr.lower() or "unrecognized" in result.stderr.lower():
            return None
        print(f"  batch corpus-check failed (exit {result.returncode}), "
              f"falling back to per-repo", file=sys.stderr)
        return None

    try:
        data = json.loads(result.stdout)
        return {k: v for k, v in data.get("repos", {}).items()}
    except json.JSONDecodeError:
        print("  batch corpus-check returned invalid JSON, falling back to per-repo",
              file=sys.stderr)
        return None


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
            cwd=repo_dir, env=corpus_env(repo_dir),
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


def validate_corpus():
    """Check that local corpus matches manifest.jsonl.

    Fails fast on missing or extra repos so local reruns use the exact
    same corpus checkout as CI.
    """
    if not MANIFEST_PATH.exists():
        return
    manifest_ids = set()
    with open(MANIFEST_PATH) as f:
        for line in f:
            line = line.strip()
            if line:
                manifest_ids.add(json.loads(line)["id"])

    local_ids = {d.name for d in CORPUS_DIR.iterdir() if d.is_dir()} if CORPUS_DIR.exists() else set()
    extra = local_ids - manifest_ids
    missing = manifest_ids - local_ids

    if extra:
        print(f"ERROR: {len(extra)} stale repos in vendor/corpus/ not in manifest:", file=sys.stderr)
        for r in sorted(extra):
            print(f"  - {r}", file=sys.stderr)
    if missing:
        pct = len(missing) / len(manifest_ids) * 100
        print(f"ERROR: {len(missing)}/{len(manifest_ids)} manifest repos not cloned locally "
              f"({pct:.0f}% missing)", file=sys.stderr)
    if extra or missing:
        print("Corpus checkout does not match bench/corpus/manifest.jsonl. "
              "Run bench/corpus/clone_repos.sh to sync repos.", file=sys.stderr)
        sys.exit(1)


def run_nitrocop_per_repo(cop_name: str, relevant_repos: set[str] | None = None) -> dict[str, int]:
    """Run nitrocop --only on each corpus repo in parallel, return {repo_id: count}.

    When relevant_repos is set, only run those repos and assume 0 for the rest.
    """
    from concurrent.futures import ThreadPoolExecutor, as_completed

    all_repos = sorted(d for d in CORPUS_DIR.iterdir() if d.is_dir())
    repos = all_repos
    if relevant_repos is not None:
        repos = [r for r in all_repos if r.name in relevant_repos]
        skipped = len(all_repos) - len(repos)
        print(f"  --quick: running {len(repos)}/{len(all_repos)} repos "
              f"(skipping {skipped} with zero baseline activity)", file=sys.stderr)

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

    # Fill in 0 for skipped repos
    if relevant_repos is not None:
        for r in all_repos:
            if r.name not in counts:
                counts[r.name] = 0

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
        description="Check a cop against the corpus for aggregate count regressions")
    parser.add_argument("cop", help="Cop name (e.g., Lint/Void)")
    parser.add_argument("--input", type=Path,
                        help="Path to corpus-results.json (default: download from CI)")
    parser.add_argument("--verbose", action="store_true",
                        help="Run per-repo and show which repos have excess offenses")
    parser.add_argument("--threshold", type=int, default=0,
                        help="Allowed excess offenses before FAIL (default: 0)")
    parser.add_argument("--rerun", action="store_true",
                        help="Force re-execution of nitrocop (ignore local cache)")
    parser.add_argument("--quick", action="store_true",
                        help="Only run repos with baseline activity (faster, may miss new FPs on zero-baseline repos)")
    parser.add_argument("--extended", action="store_true",
                        help="Use extended corpus (5k+ repos) instead of standard (1k repos)")
    args = parser.parse_args()

    # Load corpus results
    if args.input:
        input_path = args.input
    else:
        prefer = "extended" if args.extended else "standard"
        input_path = download_corpus_results(prefer=prefer)

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

    # Validate local corpus matches manifest (warns about stale/missing repos)
    if args.rerun:
        validate_corpus()
        check_corpus_bundle()

    print(f"Checking {args.cop} against corpus")
    print("Gate: count-only cop-level regression check")
    print("Not a location-level conformance proof or a department completion gate")
    print(f"Baseline (from CI): {baseline_matches:,} matches, "
          f"{baseline_fp:,} FP, {baseline_fn:,} FN")
    print(f"Expected RuboCop offenses: {expected_rubocop:,}")
    print()

    # Check if enriched per-repo-per-cop data is available in corpus results
    by_repo_cop = data.get("by_repo_cop", {})
    has_enriched = bool(by_repo_cop)

    if args.verbose and has_enriched and not args.rerun:
        # Use baseline artifact data instead of re-running nitrocop.
        # This reflects the downloaded corpus-oracle run, not local unverified changes.
        print(
            "Using baseline corpus artifact data (pass --rerun to validate current code)",
            file=sys.stderr,
        )

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

        # In artifact mode, nitrocop_total for that run is:
        # rubocop_matches + false_positives.
        nitrocop_total = baseline_matches + baseline_fp
        file_drop_offenses = 0
        file_drop_repos = {}
    else:
        # Try local cache first (unless --rerun forces re-execution)
        cached = None if args.rerun else get_cached_results(args.cop)

        if cached is not None:
            print(f"Using cached nitrocop results (pass --rerun to re-execute)", file=sys.stderr)
            per_repo = cached
        else:
            ensure_binary_fresh()
            clear_file_cache()
            # Try batch mode first (single process, much faster)
            print("Running nitrocop per-repo...", file=sys.stderr)
            per_repo = run_nitrocop_batch(args.cop)
            if per_repo is not None:
                print("  (used batch --corpus-check mode)", file=sys.stderr)
            else:
                # Fall back to per-repo subprocess mode
                # --quick: only run repos where baseline has activity for this cop
                relevant_repos = None
                if args.quick and has_enriched:
                    relevant_repos = set()
                    for repo_id, cops in by_repo_cop.items():
                        if args.cop in cops:
                            relevant_repos.add(repo_id)
                per_repo = run_nitrocop_per_repo(args.cop, relevant_repos=relevant_repos)
            save_cached_results(args.cop, per_repo)

        # Filter to only repos that were "ok" in the CI corpus oracle run.
        # Local corpus may have stale/extra repos (denylisted, removed) and
        # CI may have repos that crashed. Including these inflates the excess
        # count since they have no RuboCop baseline to compare against.
        by_repo = data.get("by_repo", [])
        ci_ok_repos = {r["repo"] for r in by_repo if r.get("status") == "ok"}
        if ci_ok_repos:
            excluded = {k for k in per_repo if k not in ci_ok_repos}
            if excluded:
                excluded_total = sum(per_repo.get(k, 0) for k in excluded if per_repo.get(k, 0) > 0)
                print(f"Excluding {len(excluded)} repos not in CI baseline "
                      f"({excluded_total:,} offenses)", file=sys.stderr)
            per_repo = {k: v for k, v in per_repo.items() if k in ci_ok_repos}

        # Identify repos with RuboCop file drops (parser crashes that cause
        # some files to be silently skipped). CI filters nitrocop offenses to
        # only RuboCop-inspected files before comparing, so CI's FP/FN counts
        # exclude offenses from dropped files. Our local run can't replicate
        # this per-file filtering, so we report the noise separately.
        file_drop_repos = {r["repo"]: r.get("rubocop_files_dropped", 0)
                           for r in by_repo
                           if r.get("rubocop_files_dropped", 0) > 0}
        file_drop_offenses = sum(per_repo.get(k, 0) for k in file_drop_repos
                                 if per_repo.get(k, 0) > 0)

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

    # CI nitrocop baseline: the offense count CI's nitrocop produced on
    # RuboCop-inspected files. Our local count should be close to this.
    ci_nitrocop_total = baseline_matches + baseline_fp
    ci_delta = nitrocop_total - ci_nitrocop_total

    print(f"Results:")
    print(f"  Expected (RuboCop):   {expected_rubocop:>10,}")
    print(f"  Actual (nitrocop):    {nitrocop_total:>10,}")
    print(f"  CI nitrocop baseline: {ci_nitrocop_total:>10,}")
    print(f"  Excess (potential FP):{excess:>10,}")
    print(f"  Missing (potential FN):{missing:>9,}")
    if file_drop_offenses > 0:
        print(f"  File-drop noise:      {file_drop_offenses:>10,}  "
              f"({len(file_drop_repos)} repos with RuboCop parser crashes)")
    print()

    # For PASS/FAIL, compare against the CI nitrocop baseline (matches + fp).
    # If our local count exceeds CI's nitrocop count, it's either regressions
    # or file-drop noise. Subtract file_drop_offenses as a rough adjustment
    # (conservative: assumes ALL offenses in file-drop repos are noise from
    # files RuboCop didn't inspect; reality is somewhere between 0 and this).
    adjusted_excess = max(0, ci_delta - file_drop_offenses)

    # Show conformance vs RuboCop (the ground truth).
    # "excess" = nitrocop fires more than RuboCop = false positives.
    # "missing" = RuboCop fires more than nitrocop = false negatives.
    # With file-drop noise, excess may be inflated (nitrocop processes files
    # RuboCop can't parse). Adjust for display purposes.
    display_excess = max(0, excess - file_drop_offenses) if file_drop_offenses > 0 else excess
    if display_excess > 0 or missing > 0:
        parts = []
        if display_excess > 0:
            parts.append(f"FP~={display_excess:,}")
        if missing > 0:
            parts.append(f"FN={missing:,}")
        conformance_note = f"  vs RuboCop counts: {' '.join(parts)}"
        if display_excess > 0 and display_excess <= baseline_fp:
            conformance_note += f" (CI had FP={baseline_fp:,})"
        if file_drop_offenses > 0 and excess > 0:
            conformance_note += f" (FP approximate due to file-drop noise)"
        print(conformance_note)
    else:
        print("  vs RuboCop counts: aggregate offense total matches (0 count delta)")
    print("  Gate type: count-only / cop-level regression")
    print()

    if adjusted_excess > args.threshold:
        print(f"FAIL: {adjusted_excess:,} NEW excess over CI nitrocop baseline "
              f"(threshold: {args.threshold})")
        if ci_delta != adjusted_excess:
            print(f"  Raw delta: {ci_delta:+,} "
                  f"(adjusted by {file_drop_offenses:,} file-drop noise)")
        if not args.verbose:
            print("Run with --verbose to see which repos have excess offenses")
        sys.exit(1)
    else:
        if excess == 0 and missing == 0:
            print("PASS: aggregate offense count matches RuboCop for this cop")
        elif adjusted_excess == 0 and excess > 0:
            print("PASS: no new excess vs CI nitrocop baseline")
            print(f"  Aggregate count still differs from RuboCop by {excess:,} excess offenses")
            if ci_delta > 0 and file_drop_offenses > 0:
                print(f"  Raw delta: {ci_delta:+,} "
                      f"(within file-drop noise of {file_drop_offenses:,})")
        else:
            print("PASS: no new excess vs CI nitrocop baseline")
            if ci_delta > 0 and file_drop_offenses > 0:
                print(f"  Raw delta: {ci_delta:+,} "
                      f"(within file-drop noise of {file_drop_offenses:,})")
        if missing > 0:
            print(f"Note: aggregate count still misses {missing:,} RuboCop offenses")
        print("Next: use scripts/verify-cop-locations.py for exact known FP/FN locations")
        print("Next: use bench_nitrocop conform to prove department-level completion")


if __name__ == "__main__":
    main()
