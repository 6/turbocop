#!/usr/bin/env python3
"""Corpus smoke test: run nitrocop + rubocop on a few small repos and compare.

Catches systemic regressions (file discovery, config resolution, directive handling)
that silently break many cops at once. Runs in ~3 min on CI.

The test compares current results against a checked-in baseline snapshot. It fails
if matches decrease or FP/FN increase beyond a small tolerance per repo. To update
the baseline after intentional changes:

    python3 scripts/corpus_smoke_test.py --update-baseline

Usage:
    python3 scripts/corpus_smoke_test.py                    # auto-detect binary
    python3 scripts/corpus_smoke_test.py --binary path/to/nitrocop
    python3 scripts/corpus_smoke_test.py --update-baseline  # regenerate baseline
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile

# Small repos pinned to exact SHAs from the corpus manifest.
# Chosen for speed: each takes <30s to clone+lint.
SMOKE_REPOS = [
    {
        "id": "multi_json__multi_json__c5fa9fc",
        "repo_url": "https://github.com/sferik/multi_json",
        "sha": "c5fa9fce50aec2d98c438f5d5e751b6f6980805c",
    },
    {
        "id": "bkeepers__dotenv__34156bf",
        "repo_url": "https://github.com/bkeepers/dotenv",
        "sha": "34156bf400cd67387fa6ed9f146778f6a2f5f743",
    },
    {
        "id": "doorkeeper__doorkeeper__b305358",
        "repo_url": "https://github.com/doorkeeper-gem/doorkeeper",
        "sha": "b30535805477bc4a2568d68968595484d6163b31",
    },
    {
        # Rufo has 121 .rb.spec files — catches file discovery regressions
        # like removing "spec" from RUBY_EXTENSIONS.
        "id": "ruby-formatter__rufo__a90e654",
        "repo_url": "https://github.com/ruby-formatter/rufo",
        "sha": "a90e6541b7b718a031145a0725e7491d98cee41f",
    },
]

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BASELINE_CONFIG = os.path.join(ROOT, "bench", "corpus", "baseline_rubocop.yml")
SNAPSHOT_PATH = os.path.join(ROOT, "bench", "corpus", "smoke_baseline.json")

# How much worse a repo can get before failing. Allows for minor fluctuations
# from rubocop version differences without masking real regressions.
REGRESSION_TOLERANCE = 5  # absolute offense count


def find_binary(explicit: str | None) -> str:
    if explicit:
        return explicit
    target_dir = os.environ.get("CARGO_TARGET_DIR", "target")
    for profile in ["release", "ci", "debug"]:
        path = os.path.join(ROOT, target_dir, profile, "nitrocop")
        if os.path.isfile(path):
            return path
    sys.exit("ERROR: nitrocop binary not found. Build with `cargo build --release` first.")


def clone_repo(repo: dict, dest: str) -> bool:
    try:
        subprocess.run(["git", "init", dest], capture_output=True, check=True)
        subprocess.run(
            ["git", "-C", dest, "fetch", "--depth", "1", repo["repo_url"], repo["sha"]],
            capture_output=True, check=True, timeout=120,
        )
        subprocess.run(
            ["git", "-C", dest, "checkout", "FETCH_HEAD"],
            capture_output=True, check=True,
        )
        return True
    except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as e:
        print(f"  WARNING: clone failed for {repo['id']}: {e}", file=sys.stderr)
        return False


def run_nitrocop(binary: str, repo_dir: str) -> dict:
    env = os.environ.copy()
    env["BUNDLE_GEMFILE"] = os.path.join(ROOT, "bench", "corpus", "Gemfile")
    env["BUNDLE_PATH"] = os.path.join(ROOT, "bench", "corpus", "vendor", "bundle")
    result = subprocess.run(
        [binary, "--preview", "--format", "json", "--no-cache", "--cache", "false",
         "--config", BASELINE_CONFIG, repo_dir],
        capture_output=True, text=True, env=env, timeout=300,
    )
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        return {}


def run_rubocop(repo_dir: str) -> dict:
    env = os.environ.copy()
    env["BUNDLE_GEMFILE"] = os.path.join(ROOT, "bench", "corpus", "Gemfile")
    env["BUNDLE_PATH"] = os.path.join(ROOT, "bench", "corpus", "vendor", "bundle")
    result = subprocess.run(
        ["bundle", "exec", "rubocop", "--config", BASELINE_CONFIG,
         "--format", "json", "--force-exclusion", "--cache", "false", repo_dir],
        capture_output=True, text=True, env=env, timeout=300,
    )
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        return {}


def diff_results(rc_data: dict, nc_data: dict) -> tuple[int, int, int, int, int]:
    """Compare rubocop vs nitrocop output. Returns (rc_files, nc_files, matches, fp, fn)."""
    rc_offenses = set()
    for f in rc_data.get("files", []):
        path = f.get("path", "")
        for o in f.get("offenses", []):
            rc_offenses.add((path, o.get("location", {}).get("line", 0), o.get("cop_name", "")))

    nc_offenses = set()
    for o in nc_data.get("offenses", []):
        nc_offenses.add((o.get("path", ""), o.get("line", 0), o.get("cop_name", "")))

    matches = len(rc_offenses & nc_offenses)
    fp = len(nc_offenses - rc_offenses)
    fn = len(rc_offenses - nc_offenses)

    rc_files = len(rc_data.get("files", []))
    nc_files = len(nc_data.get("metadata", {}).get("inspected_files", []))
    if nc_files == 0 and nc_data.get("offenses"):
        nc_files = len({o.get("path", "") for o in nc_data.get("offenses", [])})

    return rc_files, nc_files, matches, fp, fn


def run_all(binary: str) -> dict:
    """Run smoke test on all repos, return per-repo results."""
    results = {}
    with tempfile.TemporaryDirectory() as tmpdir:
        for repo in SMOKE_REPOS:
            repo_id = repo["id"]
            print(f"\n{'=' * 60}")
            print(f"  {repo_id}")
            print(f"{'=' * 60}")

            dest = os.path.join(tmpdir, repo_id)
            if not clone_repo(repo, dest):
                print(f"  SKIP (clone failed)")
                continue

            print(f"  Running rubocop...")
            rc_data = run_rubocop(dest)
            print(f"  Running nitrocop...")
            nc_data = run_nitrocop(binary, dest)

            rc_files, nc_files, matches, fp, fn = diff_results(rc_data, nc_data)
            total = matches + fp + fn
            rate = matches / total * 100 if total > 0 else 100.0

            print(f"  Files: rubocop={rc_files}, nitrocop={nc_files}")
            print(f"  Offenses: matches={matches}, FP={fp}, FN={fn}, rate={rate:.1f}%")

            results[repo_id] = {
                "rc_files": rc_files,
                "nc_files": nc_files,
                "matches": matches,
                "fp": fp,
                "fn": fn,
            }
    return results


def check_regression(current: dict, baseline: dict) -> list[str]:
    """Compare current results against baseline snapshot. Returns list of failure messages."""
    failures = []
    for repo_id, cur in current.items():
        base = baseline.get(repo_id)
        if base is None:
            continue  # New repo, no baseline to compare against

        # Fail if matches decreased (we lost correct detections)
        match_drop = base["matches"] - cur["matches"]
        if match_drop > REGRESSION_TOLERANCE:
            failures.append(
                f"{repo_id}: matches dropped by {match_drop} "
                f"({base['matches']} -> {cur['matches']})"
            )

        # Fail if FP increased (new false positives)
        fp_increase = cur["fp"] - base["fp"]
        if fp_increase > REGRESSION_TOLERANCE:
            failures.append(
                f"{repo_id}: FP increased by {fp_increase} "
                f"({base['fp']} -> {cur['fp']})"
            )

        # Fail if FN increased (new false negatives)
        fn_increase = cur["fn"] - base["fn"]
        if fn_increase > REGRESSION_TOLERANCE:
            failures.append(
                f"{repo_id}: FN increased by {fn_increase} "
                f"({base['fn']} -> {cur['fn']})"
            )

        # Fail if file count diverged significantly
        if base["rc_files"] > 0:
            file_diff = abs(cur["nc_files"] - cur["rc_files"])
            base_file_diff = abs(base["nc_files"] - base["rc_files"])
            if file_diff > base_file_diff + REGRESSION_TOLERANCE:
                failures.append(
                    f"{repo_id}: file count divergence grew "
                    f"({base_file_diff} -> {file_diff})"
                )

    return failures


def main():
    parser = argparse.ArgumentParser(description="Corpus smoke test")
    parser.add_argument("--binary", help="Path to nitrocop binary")
    parser.add_argument("--update-baseline", action="store_true",
                        help="Regenerate the baseline snapshot from current results")
    args = parser.parse_args()

    binary = find_binary(args.binary)
    print(f"Using binary: {binary}")

    # Check corpus bundle is available
    bundle_path = os.path.join(ROOT, "bench", "corpus", "vendor", "bundle")
    if not os.path.isdir(bundle_path):
        sys.exit(
            "ERROR: Corpus bundle not installed. Run:\n"
            "  cd bench/corpus && bundle config set --local path vendor/bundle && bundle install"
        )

    results = run_all(binary)

    # Summary
    total_m = sum(r["matches"] for r in results.values())
    total_fp = sum(r["fp"] for r in results.values())
    total_fn = sum(r["fn"] for r in results.values())
    grand = total_m + total_fp + total_fn
    rate = total_m / grand * 100 if grand > 0 else 100.0
    print(f"\n{'=' * 60}")
    print(f"  OVERALL: matches={total_m}, FP={total_fp}, FN={total_fn}, rate={rate:.1f}%")
    print(f"{'=' * 60}")

    if args.update_baseline:
        with open(SNAPSHOT_PATH, "w") as f:
            json.dump(results, f, indent=2, sort_keys=True)
        print(f"\nBaseline written to {SNAPSHOT_PATH}")
        return

    # Compare against baseline
    if not os.path.isfile(SNAPSHOT_PATH):
        print(f"\nWARNING: No baseline at {SNAPSHOT_PATH}")
        print("Run with --update-baseline to create one.")
        print("Falling back to absolute threshold check (90%)...")
        if rate < 90:
            print(f"FAIL: match rate {rate:.1f}% below 90%")
            sys.exit(1)
        print("PASS (no baseline)")
        return

    with open(SNAPSHOT_PATH) as f:
        baseline = json.load(f)

    failures = check_regression(results, baseline)

    if failures:
        print(f"\nFAIL: {len(failures)} regression(s) vs baseline (tolerance={REGRESSION_TOLERANCE}):")
        for msg in failures:
            print(f"  {msg}")
        print(f"\nIf this is intentional, update the baseline:")
        print(f"  python3 scripts/corpus_smoke_test.py --update-baseline")
        sys.exit(1)

    # Ratchet: auto-tighten baseline when results improve.
    # Keeps the baseline current as cops are fixed so future regressions
    # are caught relative to the latest high-water mark.
    updated = False
    for repo_id, cur in results.items():
        base = baseline.get(repo_id)
        if base is None:
            baseline[repo_id] = cur
            updated = True
            continue
        if (cur["matches"] > base["matches"]
                or cur["fp"] < base["fp"]
                or cur["fn"] < base["fn"]
                or abs(cur["nc_files"] - cur["rc_files"]) < abs(base["nc_files"] - base["rc_files"])):
            baseline[repo_id] = cur
            updated = True
            print(f"  Baseline improved for {repo_id}")

    if updated:
        with open(SNAPSHOT_PATH, "w") as f:
            json.dump(baseline, f, indent=2, sort_keys=True)
            f.write("\n")
        print(f"\nPASS (baseline auto-tightened — commit {SNAPSHOT_PATH})")
    else:
        print("\nPASS (no regression vs baseline)")


if __name__ == "__main__":
    main()
