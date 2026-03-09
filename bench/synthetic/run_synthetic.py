#!/usr/bin/env python3
"""Run nitrocop and RuboCop on the synthetic project and compare results.

Usage:
    python3 bench/synthetic/run_synthetic.py
    python3 bench/synthetic/run_synthetic.py --verbose
    python3 bench/synthetic/run_synthetic.py --output /tmp/results.json
"""
from __future__ import annotations

import argparse
import json
import math
import os
import subprocess
import sys
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path

# Cops with no corpus data. Originally 62; reduced to 55 because 7 cannot
# be triggered under Ruby 3.4 / our config:
#   - Lint/ItWithoutArgumentsInBlock: Ruby 3.4 treats `it` as block param
#   - Lint/NonDeterministicRequireOrder: Ruby 3.0+ sorts Dir results
#   - Lint/NumberedParameterAssignment: Ruby 3.4 syntax error on `_1 = x`
#   - Lint/UselessElseWithoutRescue: Ruby 3.4 syntax error
#   - Security/YAMLLoad: max Ruby 3.0 (YAML.load is safe in 3.1+)
#   - Style/ReverseFind: requires Ruby 4.0
#   - Rails/StrongParametersExpect: requires railties >= 8.0 in Gemfile.lock
TARGET_COPS = sorted([
    "Lint/ArrayLiteralInRegexp",
    "Lint/DuplicateRescueException",
    "Lint/PercentSymbolArray",
    "Lint/RegexpAsCondition",
    "Lint/TrailingCommaInAttributeDeclaration",
    "RSpec/DuplicatedMetadata",
    "RSpec/InstanceSpy",
    "RSpec/SkipBlockInsideExample",
    "RSpecRails/HttpStatusNameConsistency",
    "Rails/ActionControllerTestCase",
    "Rails/AddColumnIndex",
    "Rails/AfterCommitOverride",
    "Rails/ApplicationJob",
    "Rails/ApplicationMailer",
    "Rails/ApplicationRecord",
    "Rails/BelongsTo",
    "Rails/BulkChangeTable",
    "Rails/CompactBlank",
    "Rails/ContentTag",
    "Rails/CreateTableWithTimestamps",
    "Rails/DangerousColumnNames",
    "Rails/DelegateAllowBlank",
    "Rails/EnumSyntax",
    "Rails/EnumUniqueness",
    "Rails/EnvLocal",
    "Rails/ExpandedDateRange",
    "Rails/FreezeTime",
    "Rails/HttpPositionalArguments",
    "Rails/HttpStatusNameConsistency",
    "Rails/I18nLocaleAssignment",
    "Rails/IndexWith",
    "Rails/MigrationClassName",
    "Rails/NotNullColumn",
    "Rails/Pick",
    "Rails/Pluck",
    "Rails/RedirectBackOrTo",
    "Rails/RedundantPresenceValidationOnBelongsTo",
    "Rails/RedundantTravelBack",
    "Rails/RequireDependency",
    "Rails/ResponseParsedBody",
    "Rails/ReversibleMigration",
    "Rails/ReversibleMigrationMethodDefinition",
    "Rails/ThreeStateBooleanColumn",
    "Rails/TimeZoneAssignment",
    "Rails/ToFormattedS",
    "Rails/ToSWithArgument",
    "Rails/TopLevelHashWithIndifferentAccess",
    "Rails/UniqueValidationWithoutIndex",
    "Rails/UnusedIgnoredColumns",
    "Rails/WhereMissing",
    "Rails/WhereRange",
    "Style/Copyright",
    "Style/DoubleCopDisableDirective",
    "Style/MultilineInPatternThen",
    "Style/RedundantConstantBase",
])


def trunc4(rate: float) -> float:
    """Truncate rate to 4 decimal places (never rounds up to 1.0)."""
    return math.floor(rate * 10000) / 10000


def normalize_path(filepath: str, project_dir: str) -> str:
    """Normalize a file path to be relative to the project directory."""
    project_prefix = os.path.abspath(project_dir) + "/"
    if filepath.startswith(project_prefix):
        filepath = filepath[len(project_prefix):]
    elif filepath.startswith("project/"):
        filepath = filepath[len("project/"):]
    # Strip leading ./ from relative paths
    if filepath.startswith("./"):
        filepath = filepath[2:]
    return filepath


def parse_nitrocop_json(data: dict, project_dir: str) -> set[tuple[str, int, str]]:
    """Parse nitrocop JSON output into offense tuples."""
    offenses = set()
    for o in data.get("offenses", []):
        filepath = normalize_path(o.get("path", ""), project_dir)
        line = o.get("line", 0)
        cop = o.get("cop_name", "")
        if filepath and cop:
            offenses.add((filepath, line, cop))
    return offenses


def parse_rubocop_json(data: dict, project_dir: str) -> set[tuple[str, int, str]]:
    """Parse RuboCop JSON output into offense tuples."""
    offenses = set()
    for f in data.get("files", []):
        filepath = normalize_path(f.get("path", ""), project_dir)
        for o in f.get("offenses", []):
            line = o.get("location", {}).get("line", 0)
            cop = o.get("cop_name", "")
            if filepath and cop:
                offenses.add((filepath, line, cop))
    return offenses


def main():
    parser = argparse.ArgumentParser(
        description="Run nitrocop and RuboCop on the synthetic project and compare results."
    )
    parser.add_argument("--verbose", action="store_true", help="Print per-cop detail table")
    parser.add_argument("--output", type=str, default=None,
                        help="Output JSON path (default: synthetic-results.json in script dir)")
    args = parser.parse_args()

    script_dir = os.path.dirname(os.path.abspath(__file__))
    project_dir = os.path.join(script_dir, "project")
    rubocop_yml = os.path.join(project_dir, ".rubocop.yml")
    gemfile = os.path.join(script_dir, "Gemfile")
    nitrocop_binary = os.path.join(script_dir, "..", "..", "target", "release", "nitrocop")
    output_path = args.output or os.path.join(script_dir, "synthetic-results.json")

    if not os.path.isfile(nitrocop_binary):
        print(f"Error: nitrocop binary not found at {nitrocop_binary}\n"
              f"Run 'cargo build --release' first.", file=sys.stderr)
        sys.exit(1)

    target_cops = TARGET_COPS
    target_set = set(target_cops)
    print(f"Synthetic corpus: {len(target_cops)} cops targeted", file=sys.stderr)

    # ── Run nitrocop ──
    print("Running nitrocop...", end="", file=sys.stderr, flush=True)
    nc_result = subprocess.run(
        [nitrocop_binary, "--preview", "--no-cache", "--format", "json",
         "--config", rubocop_yml, "."],
        capture_output=True, text=True, cwd=project_dir,
    )
    if not nc_result.stdout.strip():
        print(f"\nError: nitrocop produced no output.\nstderr: {nc_result.stderr}", file=sys.stderr)
        sys.exit(1)
    try:
        nc_data = json.loads(nc_result.stdout)
    except json.JSONDecodeError as e:
        print(f"\nError: failed to parse nitrocop JSON: {e}", file=sys.stderr)
        sys.exit(1)
    nc_all = parse_nitrocop_json(nc_data, project_dir)
    nc_offenses = {o for o in nc_all if o[2] in target_set}
    print(f" {len(nc_offenses)} offenses (of {len(nc_all)} total)", file=sys.stderr)

    # ── Run RuboCop ──
    print("Running RuboCop...", end="", file=sys.stderr, flush=True)
    rc_env = os.environ.copy()
    rc_env["BUNDLE_GEMFILE"] = gemfile
    rc_result = subprocess.run(
        ["bundle", "exec", "rubocop", "--config", rubocop_yml, "--format", "json", "."],
        capture_output=True, text=True, env=rc_env, cwd=project_dir,
    )
    if not rc_result.stdout.strip():
        print(f"\nError: RuboCop produced no output.\nstderr: {rc_result.stderr}", file=sys.stderr)
        sys.exit(1)
    try:
        rc_data = json.loads(rc_result.stdout)
    except json.JSONDecodeError as e:
        print(f"\nError: failed to parse RuboCop JSON: {e}", file=sys.stderr)
        sys.exit(1)
    rc_all = parse_rubocop_json(rc_data, project_dir)
    rc_offenses = {o for o in rc_all if o[2] in target_set}
    print(f" {len(rc_offenses)} offenses (of {len(rc_all)} total)", file=sys.stderr)

    # ── Compare ──
    matches_set = nc_offenses & rc_offenses
    fp_set = nc_offenses - rc_offenses
    fn_set = rc_offenses - nc_offenses

    by_cop_matches = defaultdict(int)
    by_cop_fp = defaultdict(int)
    by_cop_fn = defaultdict(int)
    by_cop_fp_examples = defaultdict(list)
    by_cop_fn_examples = defaultdict(list)

    for filepath, line, cop in matches_set:
        by_cop_matches[cop] += 1
    for filepath, line, cop in fp_set:
        by_cop_fp[cop] += 1
        by_cop_fp_examples[cop].append(f"{filepath}:{line}")
    for filepath, line, cop in fn_set:
        by_cop_fn[cop] += 1
        by_cop_fn_examples[cop].append(f"{filepath}:{line}")

    by_cop = []
    for cop in sorted(target_set | set(by_cop_matches) | set(by_cop_fp) | set(by_cop_fn)):
        m = by_cop_matches.get(cop, 0)
        fp = by_cop_fp.get(cop, 0)
        fn = by_cop_fn.get(cop, 0)
        total = m + fp + fn
        rate = m / total if total > 0 else 1.0
        exercised = total > 0
        diverging = fp + fn > 0
        by_cop.append({
            "cop": cop,
            "matches": m,
            "fp": fp,
            "fn": fn,
            "match_rate": trunc4(rate),
            "exercised": exercised,
            "perfect_match": exercised and not diverging,
            "diverging": diverging,
            "fp_examples": sorted(by_cop_fp_examples.get(cop, [])),
            "fn_examples": sorted(by_cop_fn_examples.get(cop, [])),
        })

    by_cop.sort(key=lambda x: (-x["fp"] - x["fn"], x["cop"]))

    total_matches = len(matches_set)
    total_fp = len(fp_set)
    total_fn = len(fn_set)
    total_compared = total_matches + total_fp + total_fn
    overall_rate = total_matches / total_compared if total_compared > 0 else 1.0
    exercised_count = sum(1 for c in by_cop if c["exercised"])

    print(file=sys.stderr)
    print(f"Results: {len(by_cop)} cops, {exercised_count} exercised, "
          f"{total_fp} FP, {total_fn} FN", file=sys.stderr)
    print(file=sys.stderr)

    if args.verbose:
        print(f"  {'Cop':<45s} {'Matches':>7s} {'FP':>5s} {'FN':>5s}  Status", file=sys.stderr)
        print("  " + "-" * 44 + "  " + "-" * 7 + "  " + "-" * 4 + "  " + "-" * 4 + "  " + "-" * 15,
              file=sys.stderr)
        for c in sorted(by_cop, key=lambda x: x["cop"]):
            if c["diverging"]:
                status = "FAIL" if c["fp"] > 0 else "FN"
            elif c["exercised"]:
                status = "ok"
            else:
                status = "-- not exercised"
            print(f"  {c['cop']:<45s} {c['matches']:>7d} {c['fp']:>5d} {c['fn']:>5d}  {status}",
                  file=sys.stderr)
        print(file=sys.stderr)
    elif total_fp > 0 or total_fn > 0:
        diverging = [c for c in by_cop if c["diverging"]]
        if diverging:
            print("  Diverging cops:", file=sys.stderr)
            for c in diverging:
                status = "FAIL" if c["fp"] > 0 else "FN"
                print(f"    {c['cop']:<45s} matches={c['matches']} FP={c['fp']} FN={c['fn']}  {status}",
                      file=sys.stderr)
                for ex in c.get("fp_examples", [])[:5]:
                    print(f"      FP: {ex}", file=sys.stderr)
                for ex in c.get("fn_examples", [])[:5]:
                    print(f"      FN: {ex}", file=sys.stderr)
            print(file=sys.stderr)

    json_output = {
        "source": "synthetic",
        "run_date": datetime.now(timezone.utc).isoformat(),
        "summary": {
            "target_cops": len(target_cops),
            "exercised_cops": exercised_count,
            "total_offenses_compared": total_compared,
            "matches": total_matches,
            "fp": total_fp,
            "fn": total_fn,
            "overall_match_rate": trunc4(overall_rate),
        },
        "by_cop": by_cop,
    }
    Path(output_path).write_text(json.dumps(json_output, indent=2) + "\n")
    print(f"Wrote {output_path}", file=sys.stderr)

    if total_fp > 0:
        sys.exit(1)


if __name__ == "__main__":
    main()
