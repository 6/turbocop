#!/usr/bin/env python3
"""Merge include-gated cop results into main corpus-results.json.

The main corpus oracle comparison produces zero data for 20 Rails cops whose
Include patterns (e.g., db/**/*.rb) don't resolve when config is outside the
repo directory. A parallel include-gated comparison runs these cops with an
in-repo config. This script splices those results into the main results.

The include-gated results are authoritative for these cops. Main comparison data
is unreliable because the tools resolve Include patterns differently (nitrocop
uses scan_roots, RuboCop doesn't). The merge unconditionally replaces main data
with include-gated data for any cop that has IG activity.

See docs/investigation-target-dir-relativization.md for full context.

Usage:
    python3 bench/corpus/merge_include_gated.py \\
        --main corpus-results.json \\
        --include-gated include-gated-results.json \\
        --output corpus-results.json
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path


def trunc4(rate: float) -> float:
    """Truncate rate to 4 decimal places (never rounds up to 1.0)."""
    return math.floor(rate * 10000) / 10000


def merge(main: dict, ig: dict) -> dict:
    """Merge include-gated results into main corpus-results.json.

    Replaces main entries with IG data for any cop that has IG activity.
    IG data is authoritative — main data for these cops is unreliable.
    Returns the modified main dict.
    """
    # Build lookup of main by_cop entries by cop name
    main_cop_idx: dict[str, int] = {}
    for i, entry in enumerate(main["by_cop"]):
        main_cop_idx[entry["cop"]] = i

    # Build lookup of IG by_cop entries
    ig_cop_map: dict[str, dict] = {}
    for entry in ig.get("by_cop", []):
        ig_cop_map[entry["cop"]] = entry

    # Track which cops were merged
    merged_cops: list[str] = []

    for cop_name, ig_entry in ig_cop_map.items():
        ig_active = ig_entry["matches"] + ig_entry["fp"] + ig_entry["fn"] > 0
        if not ig_active:
            continue

        idx = main_cop_idx.get(cop_name)
        if idx is None:
            continue

        main_entry = main["by_cop"][idx]
        main_active = main_entry["matches"] + main_entry["fp"] + main_entry["fn"] > 0
        if main_active:
            # Include-gated data is authoritative for these cops. Main comparison
            # data is unreliable because Include patterns resolve differently
            # when config is outside the repo dir (scan_roots makes nitrocop
            # match but RuboCop still fails → asymmetric FP). Log the override.
            print(f"NOTE: overriding {cop_name} main data (m={main_entry['matches']}, "
                  f"fp={main_entry['fp']}, fn={main_entry['fn']}) with include-gated results "
                  f"(m={ig_entry['matches']}, fp={ig_entry['fp']}, fn={ig_entry['fn']})",
                  file=sys.stderr)

        # Replace the main entry with IG data
        main["by_cop"][idx] = ig_entry
        merged_cops.append(cop_name)

    if not merged_cops:
        print("No cops merged (include-gated results had no active cops)", file=sys.stderr)
        return main

    print(f"Merged {len(merged_cops)} cops: {', '.join(sorted(merged_cops))}", file=sys.stderr)

    # Rebuild by_department from updated by_cop
    _rebuild_departments(main)

    # Merge by_repo_cop (additive — main has no entries for these cops)
    ig_by_repo_cop = ig.get("by_repo_cop", {})
    main_by_repo_cop = main.get("by_repo_cop", {})
    for repo_id, cops in ig_by_repo_cop.items():
        for cop_name, stats in cops.items():
            if cop_name in merged_cops:
                main_by_repo_cop.setdefault(repo_id, {})[cop_name] = stats
    main["by_repo_cop"] = main_by_repo_cop

    # Merge cop_activity_repos (additive)
    ig_activity = ig.get("cop_activity_repos", {})
    main_activity = main.get("cop_activity_repos", {})
    for cop_name, repos in ig_activity.items():
        if cop_name in merged_cops:
            main_activity[cop_name] = repos
    main["cop_activity_repos"] = main_activity

    # Recalculate summary totals from by_cop
    _rebuild_summary(main)

    return main


def _rebuild_departments(main: dict) -> None:
    """Rebuild by_department from by_cop (same logic as diff_results.py)."""
    from collections import defaultdict

    dept_stats = defaultdict(lambda: {
        "matches": 0, "fp": 0, "fn": 0,
        "cops": 0, "exercised_cops": 0, "perfect_cops": 0,
        "diverging_cops": 0, "inactive_cops": 0,
    })

    for c in main["by_cop"]:
        dept = c["cop"].split("/")[0]
        dept_stats[dept]["matches"] += c["matches"]
        dept_stats[dept]["fp"] += c["fp"]
        dept_stats[dept]["fn"] += c["fn"]
        dept_stats[dept]["cops"] += 1
        if c["diverging"]:
            dept_stats[dept]["diverging_cops"] += 1
        elif c["exercised"]:
            dept_stats[dept]["perfect_cops"] += 1
        else:
            dept_stats[dept]["inactive_cops"] += 1
        if c["exercised"]:
            dept_stats[dept]["exercised_cops"] += 1

    by_department = []
    for dept in sorted(dept_stats):
        s = dept_stats[dept]
        total = s["matches"] + s["fp"] + s["fn"]
        rate = s["matches"] / total if total > 0 else 1.0
        by_department.append({
            "department": dept,
            "matches": s["matches"],
            "fp": s["fp"],
            "fn": s["fn"],
            "match_rate": trunc4(rate),
            "cops": s["cops"],
            "exercised_cops": s["exercised_cops"],
            "perfect_cops": s["perfect_cops"],
            "diverging_cops": s["diverging_cops"],
            "inactive_cops": s["inactive_cops"],
        })

    main["by_department"] = by_department


def _rebuild_summary(main: dict) -> None:
    """Recalculate summary totals from by_cop."""
    by_cop = main["by_cop"]
    total_matches = sum(c["matches"] for c in by_cop)
    total_fp = sum(c["fp"] for c in by_cop)
    total_fn = sum(c["fn"] for c in by_cop)
    oracle_total = total_matches + total_fp + total_fn
    overall_rate = total_matches / oracle_total if oracle_total > 0 else 1.0

    exercised = sum(1 for c in by_cop if c["exercised"])
    perfect = sum(1 for c in by_cop if c["perfect_match"])
    diverging = sum(1 for c in by_cop if c["diverging"])
    registered = len(by_cop)
    inactive = registered - exercised

    summary = main["summary"]
    summary["total_offenses_compared"] = oracle_total
    summary["matches"] = total_matches
    summary["fp"] = total_fp
    summary["fn"] = total_fn
    summary["overall_match_rate"] = trunc4(overall_rate)
    summary["registered_cops"] = registered
    summary["exercised_cops"] = exercised
    summary["perfect_cops"] = perfect
    summary["diverging_cops"] = diverging
    summary["inactive_cops"] = inactive


def main():
    parser = argparse.ArgumentParser(
        description="Merge include-gated cop results into main corpus-results.json")
    parser.add_argument("--main", required=True, type=Path,
                        help="Main corpus-results.json")
    parser.add_argument("--include-gated", required=True, type=Path,
                        help="Include-gated results JSON from diff_results.py")
    parser.add_argument("--output", required=True, type=Path,
                        help="Output path (can overwrite --main)")
    args = parser.parse_args()

    main_data = json.loads(args.main.read_text())
    ig_data = json.loads(args.include_gated.read_text())

    merged = merge(main_data, ig_data)

    args.output.write_text(json.dumps(merged, indent=2) + "\n")
    print(f"Wrote merged results to {args.output}", file=sys.stderr)


if __name__ == "__main__":
    main()
