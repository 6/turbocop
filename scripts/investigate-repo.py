#!/usr/bin/env python3
"""Investigate a repo's conformance from the corpus oracle data.

Answers "why is repo X at Y%?" by showing the top diverging cops for that repo.
Reads corpus-results.json (downloaded from CI or local) — no nitrocop execution needed.

Usage:
    python3 scripts/investigate-repo.py rails                    # fuzzy match repo name
    python3 scripts/investigate-repo.py rails --fp-only          # only FP-producing cops
    python3 scripts/investigate-repo.py rails --fn-only          # only FN-producing cops
    python3 scripts/investigate-repo.py rails --limit 10         # top 10 (default 20)
    python3 scripts/investigate-repo.py --list                   # list all repos by match rate
    python3 scripts/investigate-repo.py --input corpus-results.json rails
    python3 scripts/investigate-repo.py rails --exclude-cops-file fix-cops-done.txt
"""

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path


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

    tmpdir = tempfile.mkdtemp(prefix="investigate-repo-")
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


def fmt_count(n: int) -> str:
    return f"{n:,}"


def find_repo(by_repo: list[dict], by_repo_cop: dict, query: str) -> str | None:
    """Fuzzy-match a repo by name. Returns the repo_id or None."""
    # Try exact match first
    all_repo_ids = [r["repo"] for r in by_repo if r.get("status") == "ok"]

    for repo_id in all_repo_ids:
        if repo_id == query:
            return repo_id

    # Fuzzy: match if query appears anywhere in repo_id (case-insensitive)
    query_lower = query.lower()
    matches = [r for r in all_repo_ids if query_lower in r.lower()]

    if len(matches) == 1:
        return matches[0]
    elif len(matches) > 1:
        # Prefer exact match on the repo name part (owner__repo__sha → repo)
        for m in matches:
            parts = m.split("__")
            if len(parts) >= 2 and parts[1].lower() == query_lower:
                return m
        # Multiple matches, show them
        print(f"Multiple repos match '{query}':", file=sys.stderr)
        for m in matches:
            print(f"  {m}", file=sys.stderr)
        print(f"Be more specific.", file=sys.stderr)
        sys.exit(1)

    return None


def print_repo_list(by_repo: list[dict], by_repo_cop: dict):
    """Print all repos sorted by match rate (worst first)."""
    repos = []
    for r in by_repo:
        if r.get("status") != "ok":
            continue
        repo_id = r["repo"]
        files = r.get("files_inspected", r.get("files", 0))
        if isinstance(files, str):
            files = int(files) if files.isdigit() else 0
        matches = r.get("matches", 0)
        fp = r.get("fp", 0)
        fn = r.get("fn", 0)
        total = matches + fn
        match_rate = matches / total if total > 0 else 1.0
        repos.append((repo_id, files, matches, fp, fn, match_rate))

    # Sort by match rate ascending (worst first)
    repos.sort(key=lambda x: x[5])

    repo_w = max(len(r[0]) for r in repos) if repos else 20

    print(f"{'Repo':<{repo_w}}  {'Files':>6}  {'Matches':>9}  {'FP':>9}  {'FN':>9}  {'Match%':>7}")
    print(f"{'-'*repo_w}  {'-'*6}  {'-'*9}  {'-'*9}  {'-'*9}  {'-'*7}")

    for repo_id, files, matches, fp, fn, match_rate in repos:
        if fp == 0 and fn == 0:
            continue  # skip perfect repos in list mode
        print(f"{repo_id:<{repo_w}}  {files:>6}  {fmt_count(matches):>9}  "
              f"{fmt_count(fp):>9}  {fmt_count(fn):>9}  {match_rate:>6.1%}")

    # Summary
    total_repos = len([r for r in repos if r[3] > 0 or r[4] > 0])
    perfect = len(repos) - total_repos
    print(f"\n{len(repos)} repos total, {perfect} perfect, {total_repos} with divergence")


def print_repo_detail(repo_id: str, by_repo: list[dict], by_repo_cop: dict,
                      fp_only: bool = False, fn_only: bool = False, limit: int = 20,
                      exclude_cops: set[str] | None = None):
    """Print top diverging cops for a specific repo."""
    # Get repo-level stats
    repo_info = next((r for r in by_repo if r["repo"] == repo_id), None)

    if repo_info:
        files = repo_info.get("files_inspected", repo_info.get("files", 0))
        if isinstance(files, str):
            files = int(files) if files.isdigit() else 0
        matches = repo_info.get("matches", 0)
        fp = repo_info.get("fp", 0)
        fn = repo_info.get("fn", 0)
        total = matches + fn
        match_rate = matches / total if total > 0 else 1.0
        print(f"{repo_id} — {fmt_count(files)} Ruby files")
        print(f"  {fmt_count(matches)} matches, {fmt_count(fp)} FP, "
              f"{fmt_count(fn)} FN — {match_rate:.1%} match rate")
    else:
        print(f"{repo_id}")
    print()

    # Get per-cop breakdown for this repo
    cops_data = by_repo_cop.get(repo_id, {})
    if not cops_data:
        print("No per-cop data available for this repo.")
        return

    # Build list of diverging cops
    cop_divs = []
    total_fp = 0
    total_fn = 0
    excluded_count = 0
    for cop_name, entry in cops_data.items():
        if exclude_cops and cop_name in exclude_cops:
            excluded_count += 1
            continue

        cop_fp = entry.get("fp", 0)
        cop_fn = entry.get("fn", 0)
        cop_matches = entry.get("matches", 0)

        if fp_only and cop_fp == 0:
            continue
        if fn_only and cop_fn == 0:
            continue
        if cop_fp == 0 and cop_fn == 0:
            continue

        total_fp += cop_fp
        total_fn += cop_fn
        cop_divs.append((cop_name, cop_fp, cop_fn, cop_matches))

    if not cop_divs:
        label = "FP" if fp_only else "FN" if fn_only else "FP or FN"
        print(f"No cops with {label} for this repo.")
        return

    # Sort by total divergence
    cop_divs.sort(key=lambda x: x[1] + x[2], reverse=True)

    shown = cop_divs[:limit]
    cop_w = max(len(c[0]) for c in shown)
    cop_w = max(cop_w, 3)

    label = "FP-producing" if fp_only else "FN-producing" if fn_only else "diverging"
    print(f"Top {len(shown)} {label} cops (of {len(cop_divs)}):")
    print(f"  {'#':>3}  {'Cop':<{cop_w}}  {'FP':>9}  {'FN':>9}  {'FP+FN':>9}")
    print(f"  {'':->3}  {'':->{cop_w}}  {'':->9}  {'':->9}  {'':->9}")

    for i, (cop_name, cop_fp, cop_fn, cop_matches) in enumerate(shown, 1):
        print(f"  {i:>3}  {cop_name:<{cop_w}}  {fmt_count(cop_fp):>9}  "
              f"{fmt_count(cop_fn):>9}  {fmt_count(cop_fp + cop_fn):>9}")

    if len(cop_divs) > limit:
        print(f"  ... and {len(cop_divs) - limit} more (use --limit 0 to see all)")

    # Summary
    print()
    fp_only_count = sum(1 for c in cop_divs if c[1] > 0 and c[2] == 0)
    fn_only_count = sum(1 for c in cop_divs if c[1] == 0 and c[2] > 0)
    both_count = sum(1 for c in cop_divs if c[1] > 0 and c[2] > 0)
    print(f"Summary: {len(cop_divs)} diverging cops ({fmt_count(total_fp)} FP, {fmt_count(total_fn)} FN)")
    print(f"  FP-only: {fp_only_count} cops  FN-only: {fn_only_count} cops  Both: {both_count} cops")
    if excluded_count > 0:
        print(f"  ({excluded_count} already-fixed cops excluded)")


def main():
    parser = argparse.ArgumentParser(
        description="Investigate a repo's conformance from corpus oracle data")
    parser.add_argument("repo", nargs="?",
                        help="Repo name to investigate (fuzzy match, e.g. 'rails')")
    parser.add_argument("--input", type=Path,
                        help="Path to corpus-results.json (default: download from CI)")
    parser.add_argument("--list", action="store_true",
                        help="List all repos sorted by match rate (worst first)")
    parser.add_argument("--fp-only", action="store_true",
                        help="Only show cops with false positives")
    parser.add_argument("--fn-only", action="store_true",
                        help="Only show cops with false negatives")
    parser.add_argument("--limit", type=int, default=20,
                        help="Number of cops to show (default: 20, 0 = all)")
    parser.add_argument("--exclude-cops-file", type=Path,
                        help="File with cop names to exclude (one per line, # comments ok)")
    args = parser.parse_args()

    if not args.repo and not args.list:
        parser.error("Provide a repo name or use --list")

    if args.fp_only and args.fn_only:
        parser.error("Cannot use both --fp-only and --fn-only")

    # Load excluded cops
    exclude_cops: set[str] = set()
    if args.exclude_cops_file and args.exclude_cops_file.exists():
        for line in args.exclude_cops_file.read_text().splitlines():
            line = line.strip()
            if line and not line.startswith("#"):
                exclude_cops.add(line)

    # Load corpus results
    if args.input:
        input_path = args.input
    else:
        input_path = download_corpus_results()

    data = json.loads(input_path.read_text())
    by_repo = data.get("by_repo", [])
    by_repo_cop = data.get("by_repo_cop", {})

    if args.list:
        print_repo_list(by_repo, by_repo_cop)
        return

    # Find the repo
    repo_id = find_repo(by_repo, by_repo_cop, args.repo)
    if repo_id is None:
        print(f"No repo matching '{args.repo}' found in corpus results", file=sys.stderr)
        print(f"Use --list to see all available repos", file=sys.stderr)
        sys.exit(1)

    effective_limit = args.limit if args.limit > 0 else len(by_repo_cop.get(repo_id, {}))
    print_repo_detail(repo_id, by_repo, by_repo_cop,
                      fp_only=args.fp_only, fn_only=args.fn_only,
                      limit=effective_limit, exclude_cops=exclude_cops)


if __name__ == "__main__":
    main()
