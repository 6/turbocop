#!/usr/bin/env python3
"""Triage report from corpus oracle results.

Downloads the latest corpus-results.json from CI (or reads a local file)
and produces a ranked list of cops to fix next, with tier status and examples.

Usage:
    python3 bench/corpus/triage.py                              # auto-download from CI
    python3 bench/corpus/triage.py --input corpus-results.json  # use local file
    python3 bench/corpus/triage.py --limit 50                   # show top 50 (default 30)
    python3 bench/corpus/triage.py --department RSpec            # filter to one department
    python3 bench/corpus/triage.py --exclude-department Layout   # skip Layout cops
    python3 bench/corpus/triage.py --fp-only                    # only cops with FP
    python3 bench/corpus/triage.py --fn-only                    # only cops with FN
"""

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path


def download_latest_corpus_results() -> Path:
    """Download corpus-results.json from the latest successful corpus-oracle CI run."""
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
    print(f"Downloading corpus-report from run {run_id}...", file=sys.stderr)

    tmpdir = tempfile.mkdtemp(prefix="corpus-triage-")
    result = subprocess.run(
        ["gh", "run", "download", str(run_id), "--name=corpus-report", f"--dir={tmpdir}"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print(f"Error downloading artifact: {result.stderr}", file=sys.stderr)
        sys.exit(1)

    path = Path(tmpdir) / "corpus-results.json"
    if not path.exists():
        print(f"corpus-results.json not found in artifact", file=sys.stderr)
        sys.exit(1)

    return path


def load_tiers(project_root: Path) -> dict[str, str]:
    """Load tier overrides from tiers.json."""
    tiers_path = project_root / "src" / "resources" / "tiers.json"
    if not tiers_path.exists():
        return {}
    data = json.loads(tiers_path.read_text())
    return data.get("overrides", {})


def fmt_count(n: int) -> str:
    """Format a number with comma separators."""
    return f"{n:,}"


def extract_repos_from_examples(examples: list[str]) -> list[str]:
    """Extract unique repo short names from example location strings."""
    repos = []
    seen = set()
    for ex in examples:
        if ": " in ex:
            repo_id = ex.split(": ", 1)[0]
            # Shorten: owner__repo__sha -> repo
            parts = repo_id.split("__")
            short = parts[1] if len(parts) >= 2 else repo_id
            if short not in seen:
                seen.add(short)
                repos.append(short)
    return repos


def main():
    parser = argparse.ArgumentParser(description="Triage report from corpus oracle results")
    parser.add_argument("--input", type=Path, help="Path to corpus-results.json (default: download from CI)")
    parser.add_argument("--limit", type=int, default=30, help="Number of cops to show (default: 30)")
    parser.add_argument("--department", action="append", help="Only show cops in this department (can repeat)")
    parser.add_argument("--exclude-department", action="append", help="Exclude cops in this department (can repeat)")
    parser.add_argument("--fp-only", action="store_true", help="Only show cops with false positives")
    parser.add_argument("--fn-only", action="store_true", help="Only show cops with false negatives")
    args = parser.parse_args()

    # Load corpus results
    if args.input:
        input_path = args.input
    else:
        input_path = download_latest_corpus_results()

    data = json.loads(input_path.read_text())
    summary = data["summary"]
    by_cop = data["by_cop"]
    run_date = data.get("run_date", "unknown")[:10]

    # Load tiers
    project_root = Path(__file__).resolve().parent.parent.parent
    tier_overrides = load_tiers(project_root)

    # Filter to diverging cops
    diverging = [c for c in by_cop if c["fp"] + c["fn"] > 0]

    # Apply department filters
    if args.department:
        depts = {d.rstrip("/") for d in args.department}
        diverging = [c for c in diverging if c["cop"].split("/")[0] in depts]
    if args.exclude_department:
        exclude = {d.rstrip("/") for d in args.exclude_department}
        diverging = [c for c in diverging if c["cop"].split("/")[0] not in exclude]

    # Apply FP/FN filters
    if args.fp_only:
        diverging = [c for c in diverging if c["fp"] > 0]
    if args.fn_only:
        diverging = [c for c in diverging if c["fn"] > 0]

    # Sort by total divergence
    diverging.sort(key=lambda c: c["fp"] + c["fn"], reverse=True)

    # Print header
    print(f"Corpus Oracle Triage — {run_date}")
    print(f"{summary['total_repos']} repos, {fmt_count(summary['total_offenses_compared'])} offenses compared, "
          f"{summary['overall_match_rate']:.1%} match rate")
    print()

    if not diverging:
        print("No diverging cops match the filters.")
        return

    # Print table
    shown = diverging[:args.limit]
    print(f"Top {len(shown)} cops by divergence ({len(diverging)} total):")
    print()

    # Column widths
    cop_w = max(len(c["cop"]) for c in shown)
    cop_w = max(cop_w, 3)  # minimum "Cop"

    # Header
    print(f"  {'#':>3}  {'Cop':<{cop_w}}  {'FP':>9}  {'FN':>9}  {'Total':>9}  {'Tier':<7}  {'Match%':>6}  Examples")
    print(f"  {'':->3}  {'':->{cop_w}}  {'':->9}  {'':->9}  {'':->9}  {'':->7}  {'':->6}  {'':->40}")

    for i, c in enumerate(shown, 1):
        cop = c["cop"]
        fp = c["fp"]
        fn = c["fn"]
        total = fp + fn
        tier = tier_overrides.get(cop, "preview")
        match_pct = f"{c['match_rate']:.1%}" if (c["matches"] + c["fn"]) > 0 else "N/A"

        # Extract example repos
        all_examples = c.get("fp_examples", []) + c.get("fn_examples", [])
        repos = extract_repos_from_examples(all_examples)
        repos_str = ", ".join(repos[:3]) if repos else ""

        print(f"  {i:>3}  {cop:<{cop_w}}  {fmt_count(fp):>9}  {fmt_count(fn):>9}  "
              f"{fmt_count(total):>9}  {tier:<7}  {match_pct:>6}  {repos_str}")

    # Summary
    print()
    fp_only = sum(1 for c in diverging if c["fp"] > 0 and c["fn"] == 0)
    fn_only = sum(1 for c in diverging if c["fn"] > 0 and c["fp"] == 0)
    both = sum(1 for c in diverging if c["fp"] > 0 and c["fn"] > 0)
    total_fp = sum(c["fp"] for c in diverging)
    total_fn = sum(c["fn"] for c in diverging)

    print(f"Summary: {len(diverging)} diverging cops ({fmt_count(total_fp)} FP, {fmt_count(total_fn)} FN)")
    print(f"  FP-only:  {fp_only} cops")
    print(f"  FN-only:  {fn_only} cops")
    print(f"  Both:     {both} cops")


if __name__ == "__main__":
    main()
