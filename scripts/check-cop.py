#!/usr/bin/env python3
"""Check a single cop against the 500-repo corpus for FP regressions.

Compares turbocop's offense count against the RuboCop baseline from the
latest corpus oracle CI run. Catches real-world false positive regressions
that fixture tests miss.

Usage:
    python3 scripts/check-cop.py Lint/Void              # quick aggregate check
    python3 scripts/check-cop.py Lint/Void --verbose     # per-repo breakdown
    python3 scripts/check-cop.py Lint/Void --threshold 5 # allow up to 5 excess
"""

import argparse
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
CORPUS_DIR = PROJECT_ROOT / "vendor" / "corpus"
TURBOCOP_BIN = PROJECT_ROOT / "target" / "release" / "turbocop"
BASELINE_CONFIG = PROJECT_ROOT / "bench" / "corpus" / "baseline_rubocop.yml"


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

    return path


def ensure_binary():
    """Ensure release binary exists."""
    if TURBOCOP_BIN.exists():
        return
    print("Release binary not found. Run: cargo build --release", file=sys.stderr)
    sys.exit(1)


def clear_file_cache():
    """Clear turbocop's file-level result cache to avoid stale results after rebuild."""
    import shutil
    cache_dir = Path.home() / ".cache" / "turbocop"
    if cache_dir.exists():
        shutil.rmtree(cache_dir)
        print("Cleared file cache at ~/.cache/turbocop", file=sys.stderr)


def turbocop_cmd(cop_name: str, target: str) -> list[str]:
    """Build the turbocop command for corpus checking.

    Uses --force-default-config to ignore per-repo .rubocop.yml files,
    matching the CI corpus oracle's baseline config approach.
    """
    return [
        str(TURBOCOP_BIN), "--only", cop_name, "--preview",
        "--format", "json", "--no-cache",
        "--force-default-config",
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


def run_turbocop_aggregate(cop_name: str) -> int:
    """Run turbocop --only on the full corpus, return offense count."""
    result = subprocess.run(
        turbocop_cmd(cop_name, str(CORPUS_DIR)),
        capture_output=True, text=True, timeout=300,
    )
    if result.returncode not in (0, 1):  # 1 = offenses found
        print(f"turbocop failed: {result.stderr[:500]}", file=sys.stderr)
        return -1

    try:
        data = json.loads(result.stdout)
        return count_deduplicated_offenses(data)
    except json.JSONDecodeError:
        print(f"Failed to parse turbocop JSON output", file=sys.stderr)
        return -1


def run_turbocop_per_repo(cop_name: str) -> dict[str, int]:
    """Run turbocop --only on each corpus repo, return {repo_id: count}."""
    counts = {}
    repos = sorted(CORPUS_DIR.iterdir())
    total = len(repos)

    for i, repo_dir in enumerate(repos):
        if not repo_dir.is_dir():
            continue
        repo_id = repo_dir.name

        if (i + 1) % 50 == 0:
            print(f"  [{i+1}/{total}] {repo_id}...", file=sys.stderr)

        try:
            result = subprocess.run(
                turbocop_cmd(cop_name, str(repo_dir)),
                capture_output=True, text=True, timeout=120,
            )
        except subprocess.TimeoutExpired:
            counts[repo_id] = -1
            continue

        if result.returncode not in (0, 1):
            counts[repo_id] = -1
            continue

        try:
            data = json.loads(result.stdout)
            counts[repo_id] = count_deduplicated_offenses(data)
        except json.JSONDecodeError:
            counts[repo_id] = -1

    return counts


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
    clear_file_cache()

    print(f"Checking {args.cop} against 500-repo corpus")
    print(f"Baseline (from CI): {baseline_matches:,} matches, "
          f"{baseline_fp:,} FP, {baseline_fn:,} FN")
    print(f"Expected RuboCop offenses: {expected_rubocop:,}")
    print()

    if args.verbose:
        print("Running turbocop per-repo...", file=sys.stderr)
        per_repo = run_turbocop_per_repo(args.cop)
        turbocop_total = sum(c for c in per_repo.values() if c >= 0)

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
    else:
        print("Running turbocop on full corpus...", file=sys.stderr)
        turbocop_total = run_turbocop_aggregate(args.cop)
        if turbocop_total < 0:
            print("FAIL: turbocop execution failed", file=sys.stderr)
            sys.exit(2)

    excess = max(0, turbocop_total - expected_rubocop)
    missing = max(0, expected_rubocop - turbocop_total)

    print(f"Results:")
    print(f"  Expected (RuboCop):   {expected_rubocop:>10,}")
    print(f"  Actual (turbocop):    {turbocop_total:>10,}")
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
