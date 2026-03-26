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
check_cop.py for a different cop after fixing one cop is instant — only the
changed cop needs re-execution. Use --rerun to force a fresh run.

Usage:
    python3 scripts/check_cop.py Lint/Void              # quick aggregate count check
    python3 scripts/check_cop.py Lint/Void --verbose     # per-repo count breakdown
    python3 scripts/check_cop.py Lint/Void --verbose --rerun  # re-execute (auto-filters to relevant repos)
    python3 scripts/check_cop.py Lint/Void --rerun --clone --sample 15  # CI gate: clone + sample
    python3 scripts/check_cop.py Lint/Void --rerun --all-repos  # full scan (local only, slow)
"""

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

from shared.corpus_artifacts import download_corpus_results as _download_corpus

PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT / "bench" / "corpus"))
from run_nitrocop import run_nitrocop as _run_corpus_nitrocop  # noqa: E402, I001
from clone_repos import clone_repos as _clone_repos, load_manifest as _load_manifest_from_file  # noqa: E402, I001
from clone_repos import repo_head_sha  # noqa: E402, I001
CORPUS_DIR = PROJECT_ROOT / "vendor" / "corpus"
# Overridden to temp dir when --clone is used (see main())
_CLONE_DIR: Path | None = None
MANIFEST_PATH = PROJECT_ROOT / "bench" / "corpus" / "manifest.jsonl"
NITROCOP_BIN = Path(os.environ["NITROCOP_BIN"]).resolve() if "NITROCOP_BIN" in os.environ else PROJECT_ROOT / os.environ.get("CARGO_TARGET_DIR", "target") / "release" / "nitrocop"
BASELINE_CONFIG = PROJECT_ROOT / "bench" / "corpus" / "baseline_rubocop.yml"
LOCAL_CACHE_DIR = PROJECT_ROOT / ".check-cop-cache"


def download_corpus_results() -> Path:
    """Download corpus-results.json from the latest successful CI run."""
    path, _run_id, _sha = _download_corpus()
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
    from run_nitrocop import build_env
    env = build_env()
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
    """Rebuild binary when source is newer than the configured binary path."""
    ensure_binary()
    bin_mtime = NITROCOP_BIN.stat().st_mtime
    src_mtime = latest_source_mtime()
    if src_mtime <= bin_mtime:
        return

    # Detect profile from binary path (target/debug/ vs target/release/)
    is_release = "release" in NITROCOP_BIN.parts
    profile_flag = ["--release"] if is_release else []
    profile_name = "release" if is_release else "debug"
    print(f"Binary is stale; rebuilding with cargo build {' '.join(profile_flag)}... ({profile_name})",
          file=sys.stderr)
    result = subprocess.run(
        ["cargo", "build", *profile_flag],
        cwd=PROJECT_ROOT,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(f"Error rebuilding {profile_name} binary:", file=sys.stderr)
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



def _get_corpus_dir() -> Path:
    """Get the corpus directory — temp clone dir in CI, vendor/corpus locally."""
    return _CLONE_DIR if _CLONE_DIR is not None else CORPUS_DIR


def _run_one_repo(args: tuple[str, str]) -> tuple[str, int]:
    """Run nitrocop on a single repo via the shared corpus runner."""
    cop_name, repo_dir = args
    repo_id = Path(repo_dir).name
    result = _run_corpus_nitrocop(
        repo_dir, cop=cop_name, binary=str(NITROCOP_BIN), timeout=120,
    )
    return (repo_id, result["count"])



def load_manifest() -> dict[str, dict]:
    """Load repo info from manifest.jsonl, keyed by repo ID."""
    return _load_manifest_from_file(MANIFEST_PATH)


def relevant_repos_for_cop(
    cop_name: str, data: dict, *, sample: int | None = None,
) -> set[str]:
    """Return the repos worth rerunning for a cop in quick mode.

    This is the union of:
    - repos where RuboCop fires for the cop (`cop_activity_repos`)
    - repos with baseline divergence for the cop (`by_repo_cop`)

    Older corpus artifacts may not have `cop_activity_repos`; in that case we
    fall back to divergence-only behavior.

    When sample is set, cap to N repos — always including diverging repos,
    then filling with highest-offense repos for coverage.
    """
    relevant = set(data.get("cop_activity_repos", {}).get(cop_name, []))
    for repo_id, cops in data.get("by_repo_cop", {}).items():
        if cop_name in cops:
            relevant.add(repo_id)

    if sample is not None and len(relevant) > sample:
        # Always include repos with known divergence (FP or FN)
        by_repo_cop = data.get("by_repo_cop", {})
        diverging = set()
        offense_counts: dict[str, int] = {}
        for repo_id in relevant:
            entry = by_repo_cop.get(repo_id, {}).get(cop_name, {})
            fp = entry.get("fp", 0)
            fn = entry.get("fn", 0)
            if fp > 0 or fn > 0:
                diverging.add(repo_id)
            offense_counts[repo_id] = entry.get("matches", 0) + fp + fn

        # Start with all diverging repos, then fill by offense count
        sampled = set(diverging)
        remaining = sorted(
            relevant - sampled,
            key=lambda r: offense_counts.get(r, 0),
            reverse=True,
        )
        for repo_id in remaining:
            if len(sampled) >= sample:
                break
            sampled.add(repo_id)
        print(f"  --sample: {len(sampled)}/{len(relevant)} repos "
              f"({len(diverging)} diverging + {len(sampled) - len(diverging)} by offense count)",
              file=sys.stderr)
        return sampled

    return relevant


def clone_repos_for_cop(
    cop_name: str, data: dict,
    shard_index: int | None = None, total_shards: int | None = None,
    sample: int | None = None,
) -> Path:
    """Clone repos needed for a cop into a temp dir matching the oracle's structure.

    When sharding, only clones repos in this shard's slice.
    Returns the temp dir path. Repos are at <tmpdir>/repos/REPO_ID/.
    """
    import tempfile

    manifest = load_manifest()
    if not manifest:
        print("ERROR: manifest.jsonl not found", file=sys.stderr)
        sys.exit(1)

    needed = relevant_repos_for_cop(cop_name, data, sample=sample)
    if not needed:
        print(f"  No baseline activity or divergence for {cop_name}", file=sys.stderr)

    # When sharding, only clone this shard's repos
    if shard_index is not None and total_shards is not None and needed:
        sorted_needed = sorted(needed)
        shard_needed = {r for i, r in enumerate(sorted_needed) if i % total_shards == shard_index}
        print(f"  shard {shard_index}/{total_shards}: {len(shard_needed)}/{len(needed)} repos to clone",
              file=sys.stderr)
        needed = shard_needed

    tmpdir = Path(tempfile.mkdtemp(prefix="nitrocop_cop_check_"))
    print(f"  Cloning {len(needed)} repos for {cop_name} into {tmpdir}...", file=sys.stderr)
    _clone_repos(tmpdir, manifest, repo_ids=needed, parallel=3)
    return tmpdir


def validate_corpus():
    """Check that local corpus matches manifest.jsonl.

    Fails fast on missing or extra repos so local reruns use the exact
    same corpus checkout as CI.
    """
    manifest = load_manifest()
    if not manifest:
        return

    manifest_ids = set(manifest)

    corpus_dir = _get_corpus_dir()
    local_ids = {d.name for d in corpus_dir.iterdir() if d.is_dir()} if corpus_dir.exists() else set()
    extra = local_ids - manifest_ids
    missing = manifest_ids - local_ids
    wrong_sha = []

    for repo_id in sorted(local_ids & manifest_ids):
        actual = repo_head_sha(corpus_dir / repo_id)
        expected = manifest[repo_id].get("sha")
        if actual != expected:
            wrong_sha.append((repo_id, expected, actual or "(unknown)"))

    if extra:
        print(f"ERROR: {len(extra)} stale repos in vendor/corpus/ not in manifest:", file=sys.stderr)
        for r in sorted(extra):
            print(f"  - {r}", file=sys.stderr)
    if missing:
        pct = len(missing) / len(manifest_ids) * 100
        print(f"ERROR: {len(missing)}/{len(manifest_ids)} manifest repos not cloned locally "
              f"({pct:.0f}% missing)", file=sys.stderr)
    if wrong_sha:
        print(f"ERROR: {len(wrong_sha)} repos do not match the manifest SHA:", file=sys.stderr)
        for repo_id, expected, actual in wrong_sha[:20]:
            print(f"  - {repo_id}: expected {expected[:12]}, got {actual[:12]}", file=sys.stderr)
        if len(wrong_sha) > 20:
            print(f"  ... and {len(wrong_sha) - 20} more", file=sys.stderr)
    if extra or missing or wrong_sha:
        print("Corpus checkout does not match bench/corpus/manifest.jsonl. "
              "Run bench/corpus/clone_repos.sh to sync repos.", file=sys.stderr)
        if wrong_sha or len(missing) > 5:
            sys.exit(1)


def run_nitrocop_per_repo(
    cop_name: str,
    relevant_repos: set[str] | None = None,
    *,
    shard_index: int | None = None,
    total_shards: int | None = None,
) -> dict[str, int]:
    """Run nitrocop --only on each corpus repo in parallel, return {repo_id: count}.

    When relevant_repos is set, only run those repos and assume 0 for the rest.
    """
    from concurrent.futures import ThreadPoolExecutor, as_completed

    if relevant_repos is not None and not relevant_repos:
        print("  --quick: no baseline activity or divergence requires a local rerun", file=sys.stderr)
        return {}

    corpus_dir = _get_corpus_dir()
    if not corpus_dir.exists():
        raise FileNotFoundError(
            f"Local corpus checkout not found at {corpus_dir}. "
            "Pass --clone or run bench/corpus/clone_repos.sh."
        )

    all_repos = sorted(d for d in corpus_dir.iterdir() if d.is_dir())
    repos = all_repos
    if relevant_repos is not None:
        available = {r.name for r in all_repos}
        missing = sorted(relevant_repos - available)
        if missing:
            preview = ", ".join(missing[:5])
            if len(missing) > 5:
                preview += ", ..."
            raise FileNotFoundError(
                f"Missing {len(missing)} required corpus repo(s) under {corpus_dir}: {preview}"
            )
        repos = [r for r in all_repos if r.name in relevant_repos]
        skipped = len(all_repos) - len(repos)
        print(f"  --quick: running {len(repos)}/{len(all_repos)} repos "
              f"(skipping {skipped} with zero baseline activity)", file=sys.stderr)

    if shard_index is not None and total_shards is not None:
        full = len(repos)
        repos = [r for i, r in enumerate(repos) if i % total_shards == shard_index]
        print(f"  shard {shard_index}/{total_shards}: {len(repos)}/{full} repos", file=sys.stderr)

    total = len(repos)

    work = [(cop_name, str(r)) for r in repos]

    workers = min(os.cpu_count() or 4, 16)
    counts: dict[str, int] = {}
    done = 0

    with ThreadPoolExecutor(max_workers=workers) as pool:
        futures = {pool.submit(_run_one_repo, w): w for w in work}
        for future in as_completed(futures):
            repo_id, count = future.result()
            counts[repo_id] = count
            done += 1
            if done % 50 == 0:
                print(f"  [{done}/{total}] {repo_id}...", file=sys.stderr)

    # Fill in 0 for repos skipped by --quick (no baseline activity).
    # Don't fill for repos skipped by sharding — those are handled by other shards.
    if relevant_repos is not None and shard_index is None:
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


def rerun_local_per_repo(
    cop_name: str,
    data: dict,
    *,
    quick: bool,
    has_activity_index: bool,
    shard_index: int | None = None,
    total_shards: int | None = None,
    sample: int | None = None,
) -> dict[str, int]:
    """Re-run nitrocop locally using per-repo subprocess mode.

    Each repo is linted from its own directory with BUNDLE_GEMFILE and
    GIT_CEILING_DIRECTORIES set, matching the corpus oracle exactly.
    """
    ensure_binary_fresh()
    clear_file_cache()
    print("Running nitrocop per-repo...", file=sys.stderr)

    relevant_repos = None
    if quick:
        relevant_repos = relevant_repos_for_cop(cop_name, data, sample=sample)
        if not has_activity_index:
            print(
                "WARNING: corpus artifact lacks cop_activity_repos; "
                "quick rerun falls back to divergence-only data",
                file=sys.stderr,
            )
    # When --clone with sharding, the clone dir already contains only this
    # shard's repos. Don't double-shard in the scan.
    if shard_index is not None and _CLONE_DIR is not None:
        return run_nitrocop_per_repo(cop_name, relevant_repos=None)
    return run_nitrocop_per_repo(
        cop_name, relevant_repos=relevant_repos,
        shard_index=shard_index, total_shards=total_shards,
    )


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
                        help="Only run repos with baseline activity. Auto-enabled by --rerun.")
    parser.add_argument("--clone", action="store_true",
                        help="Auto-clone needed corpus repos from manifest into a temp dir.")
    parser.add_argument("--sample", type=int, default=None,
                        help="Cap to N repos (prioritizes diverging + highest-offense repos). "
                             "Useful for fast pre-merge gates on high-match cops.")
    parser.add_argument("--all-repos", action="store_true",
                        help="Run ALL corpus repos, not just those with baseline activity. "
                             "Slow (30+ min). Use for local debugging only.")
    parser.add_argument("--shard-index", type=int, default=None,
                        help="Shard index for parallel CI (0-based)")
    parser.add_argument("--total-shards", type=int, default=None,
                        help="Total number of shards for parallel CI")
    args = parser.parse_args()

    # --rerun implies --quick unless --all-repos is explicitly set.
    if args.rerun and not args.all_repos:
        args.quick = True

    if args.all_repos and os.environ.get("CI"):
        print("ERROR: --all-repos is disabled in CI (too slow). "
              "Use --rerun which auto-filters to relevant repos, "
              "or use --shard-index/--total-shards for parallel CI.",
              file=sys.stderr)
        sys.exit(1)

    # In CI, cap unsharded --clone runs to --sample 30 to prevent agents
    # from burning the full 45-min timeout on a corpus rerun.  The CI
    # cop-check workflow always passes --shard-index, so this only kicks
    # in for ad-hoc agent invocations.
    if (os.environ.get("CI")
            and args.clone
            and args.sample is None
            and args.shard_index is None):
        args.sample = 30
        print("NOTE: CI without --shard-index — auto-limiting to --sample 30",
              file=sys.stderr)

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

    # Validate local corpus matches manifest (warns about stale/missing repos)
    if args.rerun:
        if args.clone:
            # Clone into temp dir with oracle-identical path structure
            global _CLONE_DIR
            tmpdir = clone_repos_for_cop(
                args.cop, data,
                shard_index=args.shard_index, total_shards=args.total_shards,
                sample=args.sample,
            )
            _CLONE_DIR = tmpdir / "repos"
        else:
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
    if not by_repo_cop:
        print("ERROR: corpus artifact lacks by_repo_cop data. Run corpus oracle first.", file=sys.stderr)
        sys.exit(1)
    has_activity_index = bool(data.get("cop_activity_repos"))

    if args.verbose and not args.rerun:
        # Use baseline artifact data instead of re-running nitrocop.
        # This reflects the downloaded corpus-oracle run, not local unverified changes.
        print(
            "Using baseline corpus artifact data (pass --rerun to validate current code)",
            file=sys.stderr,
        )

        # Reconstruct per-repo counts from by_repo_cop
        # nitrocop count = rubocop count + FP - FN per repo
        by_repo = data.get("by_repo", [])
        {r["repo"]: r for r in by_repo if r.get("status") == "ok"}

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
            print("Using cached nitrocop results (pass --rerun to re-execute)", file=sys.stderr)
            per_repo = cached
        else:
            per_repo = rerun_local_per_repo(
                args.cop,
                data,
                quick=args.quick,
                has_activity_index=has_activity_index,
                shard_index=args.shard_index,
                total_shards=args.total_shards,
                sample=args.sample,
            )
            save_cached_results(args.cop, per_repo)

        # Older corpus artifacts do not include cop_activity_repos, so clone mode
        # only reruns baseline-diverging repos. Preserve the synthetic CI-baseline
        # fallback for those older artifacts.
        if args.clone and not has_activity_index:
            set(per_repo.keys())
            # For each repo NOT in per_repo, add its CI nitrocop count.
            # Repos in by_repo_cop have matches + FP - FN. Repos NOT in
            # by_repo_cop matched exactly, but we don't have per-repo counts
            # for those. We know the total: ci_nitrocop_total - sum(diverging).
            ci_diverging_total = 0
            for repo_id, cops in by_repo_cop.items():
                if args.cop in cops:
                    entry = cops[args.cop]
                    ci_diverging_total += entry.get("matches", 0) + entry.get("fp", 0)
            ci_matching_total = baseline_matches + baseline_fp - ci_diverging_total
            # Add the matching repos as a single synthetic entry
            per_repo["__ci_baseline_matching_repos__"] = ci_matching_total
            print(f"  Using CI baseline for non-diverging repos "
                  f"({ci_matching_total:,} offenses from matching repos)",
                  file=sys.stderr)

        # Filter to only repos that were "ok" in the CI corpus oracle run.
        # Local corpus may have stale/extra repos (denylisted, removed) and
        # CI may have repos that crashed. Including these inflates the excess
        # count since they have no RuboCop baseline to compare against.
        by_repo = data.get("by_repo", [])
        ci_ok_repos = {r["repo"] for r in by_repo if r.get("status") == "ok"}
        if ci_ok_repos:
            # Don't exclude synthetic baseline entry added by --clone
            ci_ok_repos.add("__ci_baseline_matching_repos__")
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

            # Debug: dump per-repo counts for comparison with oracle
            debug_path = PROJECT_ROOT / "check-cop-debug.json"
            debug_data = {
                "cop": args.cop,
                "per_repo": {k: v for k, v in sorted(per_repo.items())},
                "total": sum(c for c in per_repo.values() if c >= 0),
                "repos_run": len(per_repo),
                "errors": [k for k, v in per_repo.items() if v < 0],
            }
            debug_path.write_text(json.dumps(debug_data, indent=2))
            print(f"Debug: per-repo counts written to {debug_path}",
                  file=sys.stderr)

    excess = max(0, nitrocop_total - expected_rubocop)
    missing = max(0, expected_rubocop - nitrocop_total)

    # Debug: if there's a discrepancy and we have per-repo data, show details
    if (excess > 0 or missing > 0) and args.verbose and 'per_repo' in dir():
        print("Per-repo discrepancy analysis:", file=sys.stderr)
        print(f"  check-cop total: {nitrocop_total}, oracle expected: {expected_rubocop}, "
              f"diff: {nitrocop_total - expected_rubocop:+d}", file=sys.stderr)
        activity_repos = set(data.get("cop_activity_repos", {}).get(args.cop, []))
        local_active = {k for k, v in per_repo.items() if v > 0 and k != "__ci_baseline_matching_repos__"}
        only_local = sorted(local_active - activity_repos)
        only_oracle = sorted(activity_repos - local_active)
        if only_local:
            extra_from_local = sum(per_repo.get(k, 0) for k in only_local)
            print(f"  Repos with offenses locally but NOT in oracle activity ({len(only_local)}, "
                  f"{extra_from_local} offenses):", file=sys.stderr)
            for r in only_local[:10]:
                print(f"    {per_repo[r]:>4}  {r}", file=sys.stderr)
        if only_oracle:
            print(f"  Repos in oracle activity but 0 locally ({len(only_oracle)}):",
                  file=sys.stderr)
            for r in only_oracle[:10]:
                print(f"    {r}", file=sys.stderr)
        print(file=sys.stderr)

    # CI nitrocop baseline: the offense count CI's nitrocop produced on
    # RuboCop-inspected files. Our local count should be close to this.
    ci_nitrocop_total = baseline_matches + baseline_fp
    nitrocop_total - ci_nitrocop_total

    print("Results:")
    print(f"  Expected (RuboCop):   {expected_rubocop:>10,}")
    print(f"  Actual (nitrocop):    {nitrocop_total:>10,}")
    print(f"  CI nitrocop baseline: {ci_nitrocop_total:>10,}")
    print(f"  Excess (potential FP):{excess:>10,}")
    print(f"  Missing (potential FN):{missing:>9,}")
    if file_drop_offenses > 0:
        print(f"  File-drop noise:      {file_drop_offenses:>10,}  "
              f"({len(file_drop_repos)} repos with RuboCop parser crashes)")
    print()

    print("  Gate type: count-only / cop-level regression")
    print()

    # Gate: per-repo regression detection.
    #
    # Aggregate comparison is unreliable because local nitrocop scans files
    # the oracle filtered out (RuboCop parser crashes, excluded paths, etc.).
    # Instead, compare per-repo: for each repo in by_repo_cop, check if the
    # local count diverged from the oracle's per-repo nitrocop count.
    # Repos NOT in by_repo_cop matched exactly in the oracle — flag any
    # local count that exceeds the oracle activity count for that repo.
    if args.rerun and 'per_repo' in dir():
        new_fp = 0
        new_fn = 0
        fp_repos = []
        fn_repos = []
        # Build per-repo baseline counts from the oracle.
        # oracle_nitrocop = matches + FP (what the oracle's nitrocop found)
        # oracle_rubocop = matches + FN (what rubocop found)
        # A regression is when the PR's nitrocop diverges MORE from rubocop
        # than the oracle's nitrocop already did. Pre-existing FP/FN are not
        # regressions — they were already there on main.
        oracle_nitrocop_counts = {}
        oracle_rubocop_counts = {}
        for repo_id, cops in by_repo_cop.items():
            if args.cop in cops:
                entry = cops[args.cop]
                matches = entry.get("matches", 0)
                fp = entry.get("fp", 0)
                fn = entry.get("fn", 0)
                oracle_nitrocop_counts[repo_id] = matches + fp
                oracle_rubocop_counts[repo_id] = matches + fn

        # For repos with oracle activity but no divergence, oracle nitrocop
        # matched rubocop exactly. Use local count as proxy for both.
        for repo_id in data.get("cop_activity_repos", {}).get(args.cop, []):
            if repo_id not in oracle_nitrocop_counts:
                local = per_repo.get(repo_id, 0)
                oracle_nitrocop_counts[repo_id] = local
                oracle_rubocop_counts[repo_id] = local

        for repo_id, local_count in per_repo.items():
            if repo_id == "__ci_baseline_matching_repos__" or local_count < 0:
                continue
            baseline_nc = oracle_nitrocop_counts.get(repo_id)
            baseline_rc = oracle_rubocop_counts.get(repo_id)
            if baseline_nc is None or baseline_rc is None:
                continue
            # How far was the oracle's nitrocop from rubocop?
            baseline_fp = max(0, baseline_nc - baseline_rc)
            baseline_fn = max(0, baseline_rc - baseline_nc)
            # How far is the local nitrocop from rubocop?
            local_fp = max(0, local_count - baseline_rc)
            local_fn = max(0, baseline_rc - local_count)
            # Only flag if the PR made things WORSE
            fp_increase = max(0, local_fp - baseline_fp)
            fn_increase = max(0, local_fn - baseline_fn)
            if fp_increase > 0:
                new_fp += fp_increase
                fp_repos.append((repo_id, local_count, baseline_nc, baseline_rc, fp_increase))
            if fn_increase > 0:
                new_fn += fn_increase
                fn_repos.append((repo_id, local_count, baseline_nc, baseline_rc, fn_increase))

        print("  Gate: per-repo regression vs oracle baseline")
        print(f"  New FP (worse than baseline): {new_fp:>6,}")
        print(f"  New FN (worse than baseline): {new_fn:>6,}")
        print()

        failed = False
        if new_fp > args.threshold:
            print(f"FAIL: FP regression detected (+{new_fp:,})")
            for repo_id, local, bl_nc, bl_rc, diff in sorted(fp_repos, key=lambda x: -x[4])[:10]:
                print(f"  +{diff:>4}  {repo_id}  (local={local}, baseline_nc={bl_nc}, rubocop={bl_rc})")
            failed = True
        if new_fn > args.threshold:
            print(f"FAIL: FN regression detected (+{new_fn:,})")
            for repo_id, local, bl_nc, bl_rc, diff in sorted(fn_repos, key=lambda x: -x[4])[:10]:
                print(f"  +{diff:>4}  {repo_id}  (local={local}, baseline_nc={bl_nc}, rubocop={bl_rc})")
            failed = True

        # Machine-readable summary for CI aggregation
        result_str = "fail" if failed else "pass"
        print(f"SUMMARY|{args.cop}|{baseline_fp}|{baseline_fn}|{new_fp}|{new_fn}|{result_str}")

        if failed:
            sys.exit(1)
        print("PASS: no per-repo regressions vs baseline")
        sys.exit(0)

    # Per-repo gate should have handled this — if we reach here, something is wrong
    print("ERROR: per-repo gate did not execute. Check corpus artifact data.", file=sys.stderr)
    sys.exit(1)


if __name__ == "__main__":
    main()
