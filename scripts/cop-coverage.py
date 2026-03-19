#!/usr/bin/env python3
from __future__ import annotations
"""Per-cop corpus coverage report.

Shows every registered cop ranked by real-world occurrence count and unique
repo spread, with a confidence tier. Use this to identify cops that lack
real-world validation from the corpus.

Usage:
    python3 scripts/cop-coverage.py --input results.json                    # single corpus
    python3 scripts/cop-coverage.py --input results.json --synthetic s.json # + synthetic
    python3 scripts/cop-coverage.py --input results.json --extended e.json  # + extended corpus
    python3 scripts/cop-coverage.py --zero-only                             # only zero-hit cops
    python3 scripts/cop-coverage.py --department Style                      # filter by department
    python3 scripts/cop-coverage.py --format csv                            # CSV output
"""

import argparse
import csv
import json
import sys
from pathlib import Path

# Add scripts/ to path for corpus_download
sys.path.insert(0, str(Path(__file__).resolve().parent))


def load_json(path: str | None) -> dict | None:
    if not path:
        return None
    p = Path(path)
    if not p.exists():
        return None
    return json.loads(p.read_text())


def load_corpus_results(input_path: str | None) -> dict:
    """Load corpus-results.json from a local file or download from CI."""
    if input_path:
        return json.loads(Path(input_path).read_text())

    from corpus_download import download_corpus_results
    path, run_id, _ = download_corpus_results()
    print(f"Using corpus results from CI run {run_id}", file=sys.stderr)
    return json.loads(path.read_text())


def compute_coverage(data: dict) -> dict[str, dict]:
    """Compute per-cop coverage from corpus-results.json.

    Returns dict mapping cop name -> {occurrences, unique_repos, matches, fp, fn, total_repos}.
    """
    by_cop = data.get("by_cop", [])
    total_repos = data.get("summary", {}).get("total_repos", 0)

    has_enriched = by_cop and "rubocop_total" in by_cop[0]

    if has_enriched:
        return {
            c["cop"]: {
                "occurrences": c.get("rubocop_total", c["matches"] + c["fn"]),
                "unique_repos": c.get("unique_repos", 0),
                "total_repos": total_repos,
                "matches": c["matches"],
                "fp": c.get("fp", 0),
                "fn": c.get("fn", 0),
            }
            for c in by_cop
        }

    # Fallback: compute from by_repo_cop
    by_repo_cop = data.get("by_repo_cop", {})
    cop_repo_counts: dict[str, int] = {}
    for _repo_id, cops in by_repo_cop.items():
        for cop_name, stats in cops.items():
            if stats.get("matches", 0) + stats.get("fn", 0) > 0:
                cop_repo_counts[cop_name] = cop_repo_counts.get(cop_name, 0) + 1

    return {
        c["cop"]: {
            "occurrences": c["matches"] + c["fn"],
            "unique_repos": cop_repo_counts.get(c["cop"], 0),
            "total_repos": total_repos,
            "matches": c["matches"],
            "fp": c.get("fp", 0),
            "fn": c.get("fn", 0),
        }
        for c in by_cop
    }


def compute_synthetic(data: dict) -> dict[str, dict]:
    """Extract per-cop data from synthetic-results.json."""
    return {
        c["cop"]: {
            "occurrences": c["matches"] + c.get("fn", 0),
            "matches": c["matches"],
            "fp": c.get("fp", 0),
            "fn": c.get("fn", 0),
        }
        for c in data.get("by_cop", [])
        if c["matches"] + c.get("fn", 0) + c.get("fp", 0) > 0
    }


def confidence_tier(occurrences: int, unique_repos: int) -> str:
    if occurrences == 0:
        return "None"
    if occurrences >= 100 and unique_repos >= 10:
        return "High"
    if occurrences >= 10 or unique_repos >= 3:
        return "Medium"
    return "Low"


def main():
    parser = argparse.ArgumentParser(description="Per-cop corpus coverage report")
    parser.add_argument("--input", type=str, help="Path to corpus-results.json (standard corpus)")
    parser.add_argument("--extended", type=str, help="Path to extended corpus-results.json")
    parser.add_argument("--synthetic", type=str, help="Path to synthetic-results.json")
    parser.add_argument("--format", choices=["table", "csv"], default="table", help="Output format")
    parser.add_argument("--department", type=str, help="Filter to a specific department")
    parser.add_argument("--zero-only", action="store_true", help="Show only zero-hit cops")
    parser.add_argument("--summary", action="store_true", help="Show tier summary only")
    args = parser.parse_args()

    data = load_corpus_results(args.input)
    std_cov = compute_coverage(data)
    ext_data = load_json(args.extended)
    ext_cov = compute_coverage(ext_data) if ext_data else {}
    synth_data = load_json(args.synthetic)
    synth_cov = compute_synthetic(synth_data) if synth_data else {}

    has_extended = bool(ext_cov)
    has_synthetic = bool(synth_cov)

    # Merge all cop names
    all_cops = sorted(set(std_cov) | set(ext_cov) | set(synth_cov))

    std_repos = data.get("summary", {}).get("total_repos", 0)
    ext_repos = ext_data.get("summary", {}).get("total_repos", 0) if ext_data else 0

    # Build merged rows
    rows = []
    for cop in all_cops:
        s = std_cov.get(cop, {})
        e = ext_cov.get(cop, {})
        y = synth_cov.get(cop, {})
        rows.append({
            "cop": cop,
            "std_occ": s.get("occurrences", 0),
            "std_repos": s.get("unique_repos", 0),
            "ext_occ": e.get("occurrences", 0),
            "ext_repos": e.get("unique_repos", 0),
            "synth_occ": y.get("occurrences", 0),
            "confidence": confidence_tier(s.get("occurrences", 0), s.get("unique_repos", 0)),
        })

    # Sort by standard occurrences descending
    rows.sort(key=lambda x: (-x["std_occ"], -x["ext_occ"], x["cop"]))

    # Apply filters
    if args.department:
        dept = args.department.rstrip("/")
        rows = [r for r in rows if r["cop"].startswith(dept + "/")]
    if args.zero_only:
        rows = [r for r in rows if r["std_occ"] == 0]

    # Compute tier counts (unfiltered)
    tier_counts = {"High": 0, "Medium": 0, "Low": 0, "None": 0}
    for cop in all_cops:
        s = std_cov.get(cop, {})
        tier = confidence_tier(s.get("occurrences", 0), s.get("unique_repos", 0))
        tier_counts[tier] += 1

    if args.summary:
        total_cops = sum(tier_counts.values())
        print(f"Corpus coverage summary ({std_repos} repos, {total_cops} cops):")
        print(f"  High   (>=100 occurrences, >=10 repos): {tier_counts['High']}")
        print(f"  Medium (10-99 occurrences or 3-9 repos): {tier_counts['Medium']}")
        print(f"  Low    (1-9 occurrences, 1-2 repos):    {tier_counts['Low']}")
        print(f"  None   (0 occurrences):                 {tier_counts['None']}")
        return

    if args.format == "csv":
        writer = csv.writer(sys.stdout)
        header = ["Rank", "Cop", "Std Occurrences", "Std Repos"]
        if has_extended:
            header += ["Ext Occurrences", "Ext Repos"]
        if has_synthetic:
            header += ["Synth Occurrences"]
        header.append("Confidence")
        writer.writerow(header)
        for i, r in enumerate(rows, 1):
            row = [i, r["cop"], r["std_occ"], r["std_repos"]]
            if has_extended:
                row += [r["ext_occ"], r["ext_repos"]]
            if has_synthetic:
                row += [r["synth_occ"]]
            row.append(r["confidence"])
            writer.writerow(row)
        return

    # Markdown table
    sources = [f"standard corpus ({std_repos:,} repos)"]
    if has_extended:
        sources.append(f"extended corpus ({ext_repos:,} repos)")
    if has_synthetic:
        sources.append("synthetic bench")
    print(f"# Per-Cop Corpus Coverage")
    print()
    print(f"> Auto-generated by the corpus oracle workflow.")
    print(f"> Sources: {', '.join(sources)}")
    print()
    print(f"**Confidence tiers (standard corpus):** {tier_counts['High']} High, "
          f"{tier_counts['Medium']} Medium, {tier_counts['Low']} Low, {tier_counts['None']} None")
    print()
    print(f"- **High**: >=100 occurrences AND >=10 repos")
    print(f"- **Medium**: 10-99 occurrences OR 3-9 repos")
    print(f"- **Low**: 1-9 occurrences AND 1-2 repos")
    print(f"- **None**: 0 occurrences in corpus")
    print()

    # Build header
    header = "| Rank | Cop | Std Occ | Std Repos |"
    sep = "|-----:|-----|--------:|----------:|"
    if has_extended:
        header += " Ext Occ | Ext Repos |"
        sep += "--------:|----------:|"
    if has_synthetic:
        header += " Synth |"
        sep += "------:|"
    header += " Confidence |"
    sep += ":-----------|"

    print(header)
    print(sep)
    for i, r in enumerate(rows, 1):
        line = f"| {i} | {r['cop']} | {r['std_occ']:,} | {r['std_repos']} |"
        if has_extended:
            line += f" {r['ext_occ']:,} | {r['ext_repos']} |"
        if has_synthetic:
            line += f" {r['synth_occ']:,} |" if r["synth_occ"] > 0 else " — |"
        line += f" {r['confidence']} |"
        print(line)


if __name__ == "__main__":
    main()
