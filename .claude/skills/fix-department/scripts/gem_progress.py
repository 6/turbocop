#!/usr/bin/env python3
"""Gem conformance progress report from corpus oracle results.

Shows per-gem conformance status to help prioritize which gem to bring
to 100% corpus conformance next. Supports both a summary overview and
a deep-dive into a specific gem's cops.

Usage:
    python3 .claude/skills/fix-department/scripts/gem_progress.py --summary
    python3 .claude/skills/fix-department/scripts/gem_progress.py --gem rubocop-performance
    python3 .claude/skills/fix-department/scripts/gem_progress.py --gem rubocop-performance --input corpus-results.json
"""

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path

# Maps gem names to the cop department prefixes they own.
# Must match the mapping in src/bin/coverage_table.rs VENDOR_SOURCES.
GEM_DEPARTMENTS = {
    "rubocop": [
        "Bundler", "Gemspec", "Layout", "Lint", "Metrics",
        "Migration", "Naming", "Security", "Style",
    ],
    "rubocop-performance": ["Performance"],
    "rubocop-rails": ["Rails"],
    "rubocop-rspec": ["RSpec"],
    "rubocop-rspec_rails": ["RSpecRails"],
    "rubocop-factory_bot": ["FactoryBot"],
}

# Reverse map: department -> gem
DEPT_TO_GEM = {}
for gem, depts in GEM_DEPARTMENTS.items():
    for dept in depts:
        DEPT_TO_GEM[dept] = gem


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

    tmpdir = tempfile.mkdtemp(prefix="gem-progress-")
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


def fmt_count(n: int) -> str:
    return f"{n:,}"


def cop_department(cop_name: str) -> str:
    return cop_name.split("/")[0]


def cop_gem(cop_name: str) -> str:
    dept = cop_department(cop_name)
    return DEPT_TO_GEM.get(dept, "unknown")


def get_registry_cops() -> set[str]:
    """Get all cop names from turbocop's registry via --list-cops."""
    project_root = Path(__file__).resolve().parent.parent.parent.parent
    result = subprocess.run(
        ["cargo", "run", "--release", "--", "--list-cops"],
        capture_output=True, text=True, cwd=project_root,
    )
    if result.returncode != 0:
        print("Warning: could not get cop list from registry, skipping untested cop tracking", file=sys.stderr)
        return set()
    return {line.strip() for line in result.stdout.strip().splitlines() if "/" in line}


def build_gem_stats(by_cop: list[dict], registry_cops: set[str] | None = None) -> dict[str, dict]:
    """Aggregate per-cop data into per-gem stats.

    If registry_cops is provided, also tracks cops that exist in the registry
    but have no corpus data (never triggered on the 500 repos).
    """
    corpus_cop_names = {c["cop"] for c in by_cop}

    gems = {}
    for gem_name in GEM_DEPARTMENTS:
        gems[gem_name] = {
            "total_in_corpus": 0,
            "total_in_registry": 0,
            "untested": 0,       # in registry but not in corpus (never triggered)
            "perfect": 0,        # in corpus, matches>0, 0 FP, 0 FN
            "diverging": 0,
            "fp_only": 0,
            "fn_only": 0,
            "both": 0,
            "total_fp": 0,
            "total_fn": 0,
            "total_matches": 0,
            "cops": [],          # all cops in this gem from corpus data
            "untested_cops": [],  # cop names missing from corpus
        }

    # Count registry cops per gem and find untested ones
    if registry_cops:
        for cop in registry_cops:
            gem = cop_gem(cop)
            if gem not in gems:
                continue
            gems[gem]["total_in_registry"] += 1
            if cop not in corpus_cop_names:
                gems[gem]["untested"] += 1
                gems[gem]["untested_cops"].append(cop)

    for c in by_cop:
        gem = cop_gem(c["cop"])
        if gem not in gems:
            continue
        g = gems[gem]
        g["total_in_corpus"] += 1
        g["total_fp"] += c["fp"]
        g["total_fn"] += c["fn"]
        g["total_matches"] += c["matches"]
        g["cops"].append(c)

        if c["fp"] == 0 and c["fn"] == 0:
            g["perfect"] += 1
        elif c["fp"] > 0 and c["fn"] == 0:
            g["fp_only"] += 1
            g["diverging"] += 1
        elif c["fp"] == 0 and c["fn"] > 0:
            g["fn_only"] += 1
            g["diverging"] += 1
        else:
            g["both"] += 1
            g["diverging"] += 1

    # Sort untested cop lists for stable output
    for g in gems.values():
        g["untested_cops"].sort()

    return gems


def print_summary(gems: dict[str, dict], run_date: str, summary: dict, has_registry: bool):
    """Print the overview scoreboard of all gems."""
    print(f"Gem Conformance Progress — {run_date}")
    print(f"{summary['total_repos']} repos, {fmt_count(summary['total_offenses_compared'])} offenses compared")
    print()

    # Sort by diverging count (ascending = closest to done first), then untested
    sorted_gems = sorted(gems.items(), key=lambda kv: (kv[1]["diverging"], kv[1]["untested"], kv[1]["total_fp"]))

    # Column widths
    gem_w = max(len(g) for g, _ in sorted_gems)
    gem_w = max(gem_w, 3)

    # Adapt columns based on whether we have registry data
    if has_registry:
        print(f"  {'Gem':<{gem_w}}  {'Reg':>4}  {'Corpus':>6}  {'Untest':>6}  {'Perf':>5}  {'Dvrg':>5}  "
              f"{'Total FP':>9}  {'Total FN':>9}  Status")
        print(f"  {'':->{gem_w}}  {'':->4}  {'':->6}  {'':->6}  {'':->5}  {'':->5}  "
              f"{'':->9}  {'':->9}  {'':->30}")
    else:
        print(f"  {'Gem':<{gem_w}}  {'Corpus':>6}  {'Perf':>5}  {'Dvrg':>5}  "
              f"{'Total FP':>9}  {'Total FN':>9}  Status")
        print(f"  {'':->{gem_w}}  {'':->6}  {'':->5}  {'':->5}  "
              f"{'':->9}  {'':->9}  {'':->30}")

    for gem, g in sorted_gems:
        if g["total_in_corpus"] == 0 and g["total_in_registry"] == 0:
            continue

        # Determine status
        if g["diverging"] == 0 and g["untested"] == 0:
            status = "100% conformance"
        elif g["diverging"] == 0 and g["untested"] > 0:
            status = f"0 FP/FN but {g['untested']} untested"
        elif g["total_fp"] == 0:
            status = f"FP-free! {g['diverging']} FN-only cops"
        else:
            status = f"{g['diverging']} to fix"
            if g["untested"] > 0:
                status += f", {g['untested']} untested"

        if has_registry:
            print(f"  {gem:<{gem_w}}  {g['total_in_registry']:>4}  {g['total_in_corpus']:>6}  {g['untested']:>6}  "
                  f"{g['perfect']:>5}  {g['diverging']:>5}  "
                  f"{fmt_count(g['total_fp']):>9}  {fmt_count(g['total_fn']):>9}  {status}")
        else:
            print(f"  {gem:<{gem_w}}  {g['total_in_corpus']:>6}  {g['perfect']:>5}  {g['diverging']:>5}  "
                  f"{fmt_count(g['total_fp']):>9}  {fmt_count(g['total_fn']):>9}  {status}")

    print()

    # Legend
    if has_registry:
        print("  Reg=registry cops  Corpus=triggered on 500 repos  Untest=never triggered  Perf=0 FP+FN  Dvrg=FP or FN >0")
        print()

    # Summary stats
    total_diverging = sum(g["diverging"] for g in gems.values())
    total_perfect = sum(g["perfect"] for g in gems.values())
    total_untested = sum(g["untested"] for g in gems.values())
    gems_at_100 = sum(1 for g in gems.values()
                      if g["diverging"] == 0 and g["untested"] == 0
                      and (g["total_in_corpus"] > 0 or g["total_in_registry"] > 0))
    total_gems = sum(1 for g in gems.values()
                     if g["total_in_corpus"] > 0 or g["total_in_registry"] > 0)
    print(f"Overall: {gems_at_100}/{total_gems} gems at 100% conformance, "
          f"{total_perfect} verified perfect, {total_diverging} diverging, {total_untested} untested")


def print_gem_detail(gem_name: str, gems: dict[str, dict], run_date: str):
    """Print deep-dive for a specific gem."""
    if gem_name not in gems:
        print(f"Unknown gem: {gem_name}", file=sys.stderr)
        print(f"Available gems: {', '.join(sorted(GEM_DEPARTMENTS.keys()))}", file=sys.stderr)
        sys.exit(1)

    g = gems[gem_name]
    cops = g["cops"]

    if not cops:
        print(f"No corpus data for {gem_name} (0 cops found in corpus results)")
        return

    # Categorize cops
    perfect = sorted([c for c in cops if c["fp"] == 0 and c["fn"] == 0],
                     key=lambda c: c["cop"])
    fp_only = sorted([c for c in cops if c["fp"] > 0 and c["fn"] == 0],
                     key=lambda c: c["fp"], reverse=True)
    fn_only = sorted([c for c in cops if c["fp"] == 0 and c["fn"] > 0],
                     key=lambda c: c["fn"], reverse=True)
    both = sorted([c for c in cops if c["fp"] > 0 and c["fn"] > 0],
                  key=lambda c: c["fp"], reverse=True)

    print(f"{gem_name} — Conformance Deep Dive ({run_date})")
    print(f"Departments: {', '.join(GEM_DEPARTMENTS[gem_name])}")
    reg = g["total_in_registry"]
    corpus = g["total_in_corpus"]
    untested = g["untested"]
    if reg > 0:
        print(f"{reg} cops in registry, {corpus} in corpus, {untested} untested (never triggered)")
    else:
        print(f"{corpus} cops in corpus")
    print(f"{g['perfect']} verified perfect, {g['diverging']} diverging "
          f"({g['fp_only']} FP-only, {g['fn_only']} FN-only, {g['both']} both)")
    print()

    # Perfect cops (compact list)
    if perfect:
        names = [c["cop"].split("/")[1] for c in perfect]
        print(f"Perfect ({len(perfect)}):")
        # Wrap at ~100 chars
        line = "  "
        for i, name in enumerate(names):
            addition = name + (", " if i < len(names) - 1 else "")
            if len(line) + len(addition) > 100:
                print(line)
                line = "  " + addition
            else:
                line += addition
        if line.strip():
            print(line)
        print()

    # FP-only cops (fix these first!)
    if fp_only:
        print(f"FP-only ({len(fp_only)} — fix these first!):")
        cop_w = max(len(c["cop"]) for c in fp_only)
        for i, c in enumerate(fp_only, 1):
            match_pct = f"{c['match_rate']:.1%}" if c["matches"] > 0 else "N/A"
            print(f"  #{i:<3} {c['cop']:<{cop_w}}  FP={fmt_count(c['fp']):>7}  "
                  f"matches={fmt_count(c['matches']):>7}  ({match_pct})")
        print()

    # Both FP+FN cops
    if both:
        print(f"Both FP+FN ({len(both)} — fix FPs first):")
        cop_w = max(len(c["cop"]) for c in both)
        for i, c in enumerate(both, 1):
            match_pct = f"{c['match_rate']:.1%}" if (c["matches"] + c["fn"]) > 0 else "N/A"
            print(f"  #{i:<3} {c['cop']:<{cop_w}}  FP={fmt_count(c['fp']):>7}  "
                  f"FN={fmt_count(c['fn']):>7}  matches={fmt_count(c['matches']):>7}  ({match_pct})")
        print()

    # FN-only cops
    if fn_only:
        print(f"FN-only ({len(fn_only)} — lower priority, missing detections):")
        cop_w = max(len(c["cop"]) for c in fn_only)
        for i, c in enumerate(fn_only, 1):
            match_pct = f"{c['match_rate']:.1%}" if (c["matches"] + c["fn"]) > 0 else "N/A"
            print(f"  #{i:<3} {c['cop']:<{cop_w}}  FN={fmt_count(c['fn']):>7}  "
                  f"matches={fmt_count(c['matches']):>7}  ({match_pct})")
        print()

    # Untested cops (in registry but never triggered on corpus)
    if g["untested_cops"]:
        print(f"Untested ({g['untested']} — in registry but never triggered on 500 repos):")
        for cop in g["untested_cops"]:
            print(f"  {cop}")
        print()

    # Strategy recommendation
    if g["diverging"] > 0 or g["untested"] > 0:
        print("Strategy:")
        step = 0
        if fp_only:
            step += 1
            print(f"  {step}. Fix {len(fp_only)} FP-only cops to eliminate all false alarms from these cops")
        if both:
            step += 1
            print(f"  {step}. Fix FP side of {len(both)} both-FP+FN cops")
        if fn_only:
            step += 1
            print(f"  {step}. Fix {len(fn_only)} FN-only cops for full 100% conformance")
        if g["diverging"] > 0:
            fp_cops = len(fp_only) + len(both)
            print(f"  Total FP-producing cops: {fp_cops} ({fmt_count(g['total_fp'])} false positives)")
            print(f"  Total FN-producing cops: {len(fn_only) + len(both)} ({fmt_count(g['total_fn'])} false negatives)")
        if g["untested"] > 0:
            print()
            can_claim = g["diverging"] == 0
            if can_claim:
                print(f"  Note: All corpus-tested cops are perfect, but {g['untested']} cops never triggered.")
            else:
                print(f"  Note: {g['untested']} cops have no corpus data — cannot claim full 100% until they're exercised.")
            print(f"  These cops may be correct but are unverified against real-world code.")
    else:
        print("This gem is at 100% corpus conformance! All cops tested and verified.")


def main():
    parser = argparse.ArgumentParser(description="Gem conformance progress report")
    parser.add_argument("--input", type=Path,
                        help="Path to corpus-results.json (default: download from CI)")
    parser.add_argument("--summary", action="store_true",
                        help="Show overview scoreboard of all gems")
    parser.add_argument("--gem", type=str,
                        help="Deep-dive into a specific gem (e.g., rubocop-performance)")
    args = parser.parse_args()

    if not args.summary and not args.gem:
        print("Specify --summary or --gem <name>", file=sys.stderr)
        print(f"Available gems: {', '.join(sorted(GEM_DEPARTMENTS.keys()))}", file=sys.stderr)
        sys.exit(1)

    # Load corpus results
    if args.input:
        input_path = args.input
    else:
        input_path = download_latest_corpus_results()

    data = json.loads(input_path.read_text())
    summary = data["summary"]
    by_cop = data["by_cop"]
    run_date = data.get("run_date", "unknown")[:10]

    # Get registry cops for untested detection (requires cargo build)
    print("Loading cop registry...", file=sys.stderr)
    registry_cops = get_registry_cops()
    has_registry = len(registry_cops) > 0
    if not has_registry:
        print("Warning: running without registry data — untested cops won't be shown", file=sys.stderr)

    gems = build_gem_stats(by_cop, registry_cops if has_registry else None)

    if args.summary:
        print_summary(gems, run_date, summary, has_registry)

    if args.gem:
        if args.summary:
            print()
            print("=" * 80)
            print()
        print_gem_detail(args.gem, gems, run_date)


if __name__ == "__main__":
    main()
