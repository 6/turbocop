#!/usr/bin/env python3
from __future__ import annotations
"""Investigate a cop's false positives/negatives from the corpus oracle data.

Reads corpus-results.json (downloaded from CI or local) and shows all FP/FN
locations grouped by repo with optional source context from vendor/corpus/.

No nitrocop execution needed — this reads pre-computed data.

Usage:
    python3 scripts/investigate-cop.py Style/PercentQLiterals
    python3 scripts/investigate-cop.py Style/PercentQLiterals --context
    python3 scripts/investigate-cop.py Style/PercentQLiterals --repos-only
    python3 scripts/investigate-cop.py Style/PercentQLiterals --fp-only
    python3 scripts/investigate-cop.py Style/PercentQLiterals --fn-only
    python3 scripts/investigate-cop.py Style/PercentQLiterals --input corpus-results.json
"""

import argparse
import json
import subprocess
import sys
import tempfile
from collections import defaultdict
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
CORPUS_DIR = PROJECT_ROOT / "vendor" / "corpus"


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
    print(f"Downloading corpus-report from run {run_id}...", file=sys.stderr)

    tmpdir = tempfile.mkdtemp(prefix="investigate-cop-")
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


def parse_example(ex: str) -> tuple[str, str, int] | None:
    """Parse 'repo_id: filepath:line' into (repo_id, filepath, line)."""
    if ": " not in ex:
        return None
    repo_id, rest = ex.split(": ", 1)
    # filepath:line — find last colon for line number
    last_colon = rest.rfind(":")
    if last_colon < 0:
        return None
    filepath = rest[:last_colon]
    try:
        line = int(rest[last_colon + 1:])
    except ValueError:
        return None
    return repo_id, filepath, line


def show_context(repo_id: str, filepath: str, line: int, context_lines: int = 3):
    """Show source lines around the offense from vendor/corpus/."""
    source_path = CORPUS_DIR / repo_id / filepath
    if not source_path.exists():
        print(f"      (file not found: {source_path})")
        return

    try:
        lines = source_path.read_text(errors="replace").splitlines()
    except Exception as e:
        print(f"      (error reading file: {e})")
        return

    start = max(0, line - 1 - context_lines)
    end = min(len(lines), line + context_lines)

    for i in range(start, end):
        marker = " >> " if i == line - 1 else "    "
        print(f"      {marker}{i + 1:>5}: {lines[i]}")


def main():
    parser = argparse.ArgumentParser(
        description="Investigate a cop's FP/FN from corpus oracle data")
    parser.add_argument("cop", help="Cop name (e.g., Style/PercentQLiterals)")
    parser.add_argument("--input", type=Path,
                        help="Path to corpus-results.json (default: download from CI)")
    parser.add_argument("--context", action="store_true",
                        help="Show source lines around each FP/FN from vendor/corpus/")
    parser.add_argument("--repos-only", action="store_true",
                        help="Show only the per-repo breakdown table")
    parser.add_argument("--fp-only", action="store_true",
                        help="Show only false positives")
    parser.add_argument("--fn-only", action="store_true",
                        help="Show only false negatives")
    parser.add_argument("--limit", type=int, default=0,
                        help="Limit number of examples shown (0 = all)")
    args = parser.parse_args()

    if args.fp_only and args.fn_only:
        print("Cannot use both --fp-only and --fn-only", file=sys.stderr)
        sys.exit(1)

    # Load corpus results
    if args.input:
        input_path = args.input
    else:
        input_path = download_corpus_results()

    data = json.loads(input_path.read_text())
    by_cop = data["by_cop"]
    by_repo_cop = data.get("by_repo_cop", {})

    # Find the cop
    cop_entry = next((e for e in by_cop if e["cop"] == args.cop), None)
    if cop_entry is None:
        print(f"Cop '{args.cop}' not found in corpus results", file=sys.stderr)
        # Suggest similar names
        query = args.cop.split("/")[-1].lower()
        matches = [e["cop"] for e in by_cop if query in e["cop"].lower()]
        if matches:
            print(f"Similar cops:", file=sys.stderr)
            for m in matches[:10]:
                print(f"  {m}", file=sys.stderr)
        sys.exit(1)

    fp_count = cop_entry["fp"]
    fn_count = cop_entry["fn"]
    match_count = cop_entry["matches"]
    total_oracle = match_count + fn_count

    print(f"{args.cop}")
    print(f"  Matches: {match_count:,}  FP: {fp_count:,}  FN: {fn_count:,}  "
          f"Match: {cop_entry['match_rate']:.1%}")
    print()

    # Use by_repo_cop for repo breakdown if available
    if by_repo_cop:
        repo_data = {}
        for repo_id, cops in by_repo_cop.items():
            if args.cop in cops:
                entry = cops[args.cop]
                fp = entry.get("fp", 0)
                fn = entry.get("fn", 0)
                if args.fp_only and fp == 0:
                    continue
                if args.fn_only and fn == 0:
                    continue
                if fp > 0 or fn > 0:
                    repo_data[repo_id] = {"fp": fp, "fn": fn}

        if repo_data:
            # Sort by total divergence
            sorted_repos = sorted(repo_data.items(),
                                  key=lambda x: x[1]["fp"] + x[1]["fn"],
                                  reverse=True)
            label = "FP" if args.fp_only else "FN" if args.fn_only else "FP+FN"
            print(f"Per-repo breakdown ({len(sorted_repos)} repos with {label}):")
            print(f"  {'Repo':<60} {'FP':>6} {'FN':>6}")
            print(f"  {'-'*60} {'-'*6} {'-'*6}")
            for repo_id, counts in sorted_repos:
                fp = counts["fp"]
                fn = counts["fn"]
                print(f"  {repo_id:<60} {fp:>6} {fn:>6}")
            print()
        else:
            print("No repos with divergence for this cop.")
            print()

    if args.repos_only:
        return

    # Show individual FP/FN locations
    fp_examples = cop_entry.get("fp_examples", [])
    fn_examples = cop_entry.get("fn_examples", [])

    if not args.fn_only and fp_examples:
        # Group by repo
        by_repo = defaultdict(list)
        for ex in fp_examples:
            parsed = parse_example(ex)
            if parsed:
                repo_id, filepath, line = parsed
                by_repo[repo_id].append((filepath, line))
            else:
                by_repo["(unknown)"].append((ex, 0))

        print(f"False positives ({len(fp_examples):,} total):")
        shown = 0
        for repo_id in sorted(by_repo, key=lambda r: -len(by_repo[r])):
            locations = by_repo[repo_id]
            print(f"\n  {repo_id} ({len(locations)}):")
            for filepath, line in sorted(locations):
                if args.limit and shown >= args.limit:
                    remaining = len(fp_examples) - shown
                    print(f"\n  ... {remaining:,} more (use --limit 0 to see all)")
                    break
                print(f"    {filepath}:{line}")
                if args.context and line > 0:
                    show_context(repo_id, filepath, line)
                shown += 1
            else:
                continue
            break
        print()

    if not args.fp_only and fn_examples:
        by_repo = defaultdict(list)
        for ex in fn_examples:
            parsed = parse_example(ex)
            if parsed:
                repo_id, filepath, line = parsed
                by_repo[repo_id].append((filepath, line))
            else:
                by_repo["(unknown)"].append((ex, 0))

        print(f"False negatives ({len(fn_examples):,} total):")
        shown = 0
        for repo_id in sorted(by_repo, key=lambda r: -len(by_repo[r])):
            locations = by_repo[repo_id]
            print(f"\n  {repo_id} ({len(locations)}):")
            for filepath, line in sorted(locations):
                if args.limit and shown >= args.limit:
                    remaining = len(fn_examples) - shown
                    print(f"\n  ... {remaining:,} more (use --limit 0 to see all)")
                    break
                print(f"    {filepath}:{line}")
                if args.context and line > 0:
                    show_context(repo_id, filepath, line)
                shown += 1
            else:
                continue
            break
        print()

    # Show note about data freshness
    if not fp_examples and not fn_examples:
        if fp_count > 0 or fn_count > 0:
            print("Note: corpus-results.json has counts but no example locations.")
            print("This may be from an older corpus run before full example storage.")
            print("Re-run the corpus oracle to get full example data.")


if __name__ == "__main__":
    main()
