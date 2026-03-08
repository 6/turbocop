#!/usr/bin/env python3
"""Update README.md sections from corpus oracle results.

Reads corpus-results.json and manifest.jsonl to update the generated Cops
breakdown plus the conformance percentages, offense counts, and top-15 repo
table in README.md.

Usage:
    python3 bench/corpus/update_readme.py \
        --input corpus-results.json \
        --manifest bench/corpus/manifest.jsonl \
        --readme README.md

    # Dry run (print changes to stderr, don't write)
    python3 bench/corpus/update_readme.py --input corpus-results.json --dry-run
"""

import argparse
import json
import math
import re
import sys
from pathlib import Path

COPS_SECTION_START = "<!-- corpus-cops:start -->"
COPS_SECTION_END = "<!-- corpus-cops:end -->"

GEMS = [
    {
        "key": "rubocop",
        "url": "https://github.com/rubocop/rubocop",
        "departments": [
            "Layout",
            "Lint",
            "Style",
            "Metrics",
            "Naming",
            "Security",
            "Bundler",
            "Gemspec",
            "Migration",
        ],
    },
    {
        "key": "rubocop-rails",
        "url": "https://github.com/rubocop/rubocop-rails",
        "departments": ["Rails"],
    },
    {
        "key": "rubocop-performance",
        "url": "https://github.com/rubocop/rubocop-performance",
        "departments": ["Performance"],
    },
    {
        "key": "rubocop-rspec",
        "url": "https://github.com/rubocop/rubocop-rspec",
        "departments": ["RSpec"],
    },
    {
        "key": "rubocop-rspec_rails",
        "url": "https://github.com/rubocop/rubocop-rspec_rails",
        "departments": ["RSpecRails"],
    },
    {
        "key": "rubocop-factory_bot",
        "url": "https://github.com/rubocop/rubocop-factory_bot",
        "departments": ["FactoryBot"],
    },
]


def load_manifest_stars(path: Path) -> dict[str, tuple[str, int]]:
    """Load manifest and extract repo_url + star count from notes.

    Returns dict mapping repo ID prefix (owner__name) to (repo_url, stars).
    """
    repos = {}
    with open(path) as f:
        for line in f:
            entry = json.loads(line.strip())
            repo_id = entry["id"]
            repo_url = entry["repo_url"]
            notes = entry.get("notes", "")

            # Parse star count from notes: "..., 51454 stars" or "auto-discovered, 51454 stars"
            m = re.search(r"(\d+)\s+stars", notes)
            stars = int(m.group(1)) if m else 0

            # Key by owner__name prefix (strip the SHA suffix)
            parts = repo_id.split("__")
            prefix = "__".join(parts[:2])
            repos[prefix] = (repo_url, stars)

    return repos


def format_files(n: int) -> str:
    """Format file count: 163000 -> '163k'."""
    return f"{n // 1000}k"


def format_count_summary(n: int) -> str:
    """Format count for summary: 4989169 -> '5.0M', 72659 -> '72.7K'."""
    if n >= 1_000_000:
        return f"{n / 1_000_000:.1f}M"
    elif n >= 1_000:
        return f"{n / 1_000:.1f}K"
    return str(n)


def format_match_rate(rate: float) -> str:
    """Format match rate floored to 0.1%: 0.9999 -> '99.9%', never rounds up to 100%."""
    return f"{math.floor(rate * 1000) / 10:.1f}%"


def build_department_stats(data: dict) -> dict[str, dict]:
    """Build per-department cop counts for the generated README Cops section."""
    derived: dict[str, dict] = {}
    for cop in data.get("by_cop", []):
        dept = cop["cop"].split("/")[0]
        stats = derived.setdefault(dept, {
            "department": dept,
            "cops": 0,
            "seen_cops": 0,
            "perfect_cops": 0,
            "diverging_cops": 0,
            "no_data_cops": 0,
            "matches": 0,
            "fp": 0,
            "fn": 0,
        })
        matches = cop.get("matches", 0)
        fp = cop.get("fp", 0)
        fn = cop.get("fn", 0)
        seen = matches + fp + fn > 0
        diverging = fp + fn > 0
        stats["cops"] += 1
        stats["matches"] += matches
        stats["fp"] += fp
        stats["fn"] += fn
        if seen:
            stats["seen_cops"] += 1
        if diverging:
            stats["diverging_cops"] += 1
        elif seen:
            stats["perfect_cops"] += 1
        else:
            stats["no_data_cops"] += 1

    stats_by_department: dict[str, dict] = {}
    for entry in data.get("by_department", []):
        dept = entry["department"]
        derived_entry = derived.get(dept, {})
        stats_by_department[dept] = {
            "department": dept,
            "cops": entry.get("cops", derived_entry.get("cops", 0)),
            "seen_cops": entry.get("seen_cops", entry.get("exercised_cops", derived_entry.get("seen_cops", 0))),
            "perfect_cops": entry.get("perfect_cops", derived_entry.get("perfect_cops", 0)),
            "diverging_cops": entry.get("diverging_cops", derived_entry.get("diverging_cops", 0)),
            "no_data_cops": entry.get("no_data_cops", entry.get("inactive_cops", derived_entry.get("no_data_cops", 0))),
            "matches": entry.get("matches", derived_entry.get("matches", 0)),
            "fp": entry.get("fp", derived_entry.get("fp", 0)),
            "fn": entry.get("fn", derived_entry.get("fn", 0)),
        }

    for dept, entry in derived.items():
        stats_by_department.setdefault(dept, entry)

    for gem in GEMS:
        for dept in gem["departments"]:
            stats_by_department.setdefault(dept, {
                "department": dept,
                "cops": 0,
                "seen_cops": 0,
                "perfect_cops": 0,
                "diverging_cops": 0,
                "no_data_cops": 0,
                "matches": 0,
                "fp": 0,
                "fn": 0,
            })

    return stats_by_department


def build_cops_section(data: dict) -> str:
    """Build the generated README Cops section."""
    summary = data.get("summary", {})
    baseline = data.get("baseline", {})
    by_department = build_department_stats(data)

    total_cops = summary.get("registered_cops", sum(d["cops"] for d in by_department.values()))
    perfect_cops = summary.get("perfect_cops", sum(d["perfect_cops"] for d in by_department.values()))
    diverging_cops = summary.get("diverging_cops", sum(d["diverging_cops"] for d in by_department.values()))
    no_data_cops = summary.get("no_data_cops", summary.get("inactive_cops", sum(d["no_data_cops"] for d in by_department.values())))

    lines = []
    lines.append(f"nitrocop supports {total_cops:,} cops from {len(GEMS)} RuboCop gems.")
    lines.append("")
    lines.append(
        f"Current corpus status: {perfect_cops:,} cops match RuboCop exactly on the corpus, "
        f"{diverging_cops:,} diverge, and {no_data_cops:,} have no corpus data."
    )
    lines.append("")
    lines.append("No corpus data means the cop never appeared in the corpus, so it has not been compared yet.")
    lines.append("")

    for gem in GEMS:
        rows = [by_department[dept] for dept in gem["departments"]]
        total = sum(r["cops"] for r in rows)
        perfect = sum(r["perfect_cops"] for r in rows)
        diverging = sum(r["diverging_cops"] for r in rows)
        no_data = sum(r["no_data_cops"] for r in rows)
        version = baseline.get(gem["key"], "?")
        lines.append(f"**[{gem['key']}]({gem['url']})** `{version}` ({total:,} cops)")
        lines.append("")
        lines.append("| Department | Total cops | Exact match | Diverging | No corpus data |")
        lines.append("|------------|-----------:|------------:|----------:|---------------:|")
        for row in rows:
            lines.append(
                f"| {row['department']} | {row['cops']:,} | "
                f"{row['perfect_cops']:,} | {row['diverging_cops']:,} | {row['no_data_cops']:,} |"
            )
        if len(rows) > 1:
            lines.append(
                f"| **Total** | **{total:,}** | **{perfect:,}** | "
                f"**{diverging:,}** | **{no_data:,}** |"
            )
            lines.append("")
        else:
            lines.append("")

    return "\n".join(lines).rstrip()


def replace_marked_section(text: str, start_marker: str, end_marker: str, body: str) -> str:
    """Replace the section between two explicit markers."""
    start = text.find(start_marker)
    end = text.find(end_marker)
    if start == -1 or end == -1 or end < start:
        raise ValueError(
            f"README is missing generated section markers: {start_marker} ... {end_marker}"
        )

    start += len(start_marker)
    return text[:start] + "\n" + body + "\n" + text[end:]


def build_top15_table(by_repo: list, manifest: dict[str, tuple[str, int]]) -> str:
    """Build the top-15 repos markdown table with FP/FN columns."""
    # Match corpus results to manifest entries and attach stars
    enriched = []
    for repo in by_repo:
        if repo["status"] != "ok":
            continue
        repo_id = repo["repo"]
        prefix = "__".join(repo_id.split("__")[:2])
        if prefix not in manifest:
            continue
        repo_url, stars = manifest[prefix]
        if stars == 0:
            continue
        # Extract short name from URL: https://github.com/rails/rails -> rails
        short_name = repo_url.rstrip("/").split("/")[-1]
        total_offenses = repo["matches"] + repo["fn"]
        files = repo.get("files_inspected", 0)
        enriched.append({
            "name": short_name,
            "url": repo_url,
            "stars": stars,
            "files": files,
            "fp": repo["fp"],
            "fn": repo["fn"],
            "offenses": total_offenses,
            "match_rate": repo["match_rate"],
        })

    # Filter to repos with meaningful offense counts (exclude trivial repos)
    enriched = [r for r in enriched if r["offenses"] >= 1000]

    # Sort by stars descending (most recognizable repos), take top 15
    enriched.sort(key=lambda x: x["stars"], reverse=True)
    top15 = enriched[:15]

    lines = []
    lines.append("| Repo | .rb files | RuboCop offenses | nitrocop extra (FP) | nitrocop missed (FN) | Agreement |")
    lines.append("|------|----------:|-----------------:|--------------------:|---------------------:|----------:|")
    for r in top15:
        name_link = f"[{r['name']}]({r['url']})"
        lines.append(f"| {name_link} | {r['files']:,} | {r['offenses']:,} | {r['fp']:,} | {r['fn']:,} | {format_match_rate(r['match_rate'])} |")

    return "\n".join(lines)


def build_summary_table(summary: dict) -> str:
    """Build the FP/FN summary table."""
    matches = summary["matches"]
    fp = summary["fp"]
    fn = summary["fn"]
    total = matches + fp + fn

    lines = []
    lines.append("|                        |    Count |  Rate |")
    lines.append("|:-----------------------|--------: |------:|")
    lines.append(f"| Agreed                 | {format_count_summary(matches):>8} | {matches/total:.1%} |")
    lines.append(f"| nitrocop extra (FP)    | {format_count_summary(fp):>8} | {fp/total:.1%} |")
    lines.append(f"| nitrocop missed (FN)   | {format_count_summary(fn):>8} | {fn/total:.1%} |")
    return "\n".join(lines)


def update_readme(readme_text: str, data: dict, manifest: dict[str, tuple[str, int]]) -> str:
    """Replace conformance data in README text."""
    summary = data["summary"]
    by_repo = data["by_repo"]
    total_repos = summary["total_repos"]
    matches = summary["matches"]
    fp = summary["fp"]
    fn = summary["fn"]
    total = matches + fp + fn
    conformance_rate = matches / total if total > 0 else 0.0
    files = summary.get("total_files_inspected", 0)

    rate_str = format_match_rate(conformance_rate)
    files_str = format_files(files) if files > 0 else None

    # 0. Generated Cops section between explicit markers
    readme_text = replace_marked_section(
        readme_text,
        COPS_SECTION_START,
        COPS_SECTION_END,
        build_cops_section(data),
    )

    # 1. Features bullet: **XX.X% conformance**
    readme_text = re.sub(
        r"\*\*[\d.]+% conformance\*\*",
        f"**{rate_str} conformance**",
        readme_text,
    )

    # 2. Repo count: update all "N open-source repos" occurrences
    readme_text = re.sub(
        r"[\d,]+ open-source repos",
        f"{total_repos:,} open-source repos",
        readme_text,
    )

    # 3. File count in corpus description: (XXXk Ruby files)
    if files_str:
        readme_text = re.sub(
            r"\(\d+k Ruby files\)",
            f"({files_str} Ruby files)",
            readme_text,
        )

    # 4. Summary table: Agreed / nitrocop extra (FP) / nitrocop missed (FN)
    new_summary = build_summary_table(summary)
    readme_text = re.sub(
        r"\|[^\n]*Count[^\n]*\n\|[^\n]*-+[^\n]*\n(?:\|[^\n]*\n){2,3}",
        new_summary + "\n",
        readme_text,
    )

    # 7. Replace the top-15 table (header + separator + data rows)
    new_table = build_top15_table(by_repo, manifest)
    readme_text = re.sub(
        r"\| Repo \| (?:Stars|Files|\.rb files|FP|Extra|Offenses|RuboCop|nitrocop) [^\n]*\n\|[-| :]+\n(?:\| .+\n)*",
        new_table + "\n",
        readme_text,
    )

    return readme_text


def main():
    parser = argparse.ArgumentParser(description="Update README.md conformance section")
    parser.add_argument("--input", required=True, type=Path, help="Path to corpus-results.json")
    parser.add_argument("--manifest", type=Path, default=Path("bench/corpus/manifest.jsonl"),
                        help="Path to manifest.jsonl")
    parser.add_argument("--readme", type=Path, default=Path("README.md"),
                        help="Path to README.md")
    parser.add_argument("--dry-run", action="store_true", help="Print diff to stderr without writing")
    args = parser.parse_args()

    data = json.loads(args.input.read_text())
    manifest = load_manifest_stars(args.manifest)

    readme_text = args.readme.read_text()
    updated = update_readme(readme_text, data, manifest)

    if updated == readme_text:
        print("No changes needed", file=sys.stderr)
        return

    if args.dry_run:
        # Show what changed
        old_lines = readme_text.splitlines()
        new_lines = updated.splitlines()
        for i, (old, new) in enumerate(zip(old_lines, new_lines)):
            if old != new:
                print(f"L{i+1} - {old}", file=sys.stderr)
                print(f"L{i+1} + {new}", file=sys.stderr)
        print(f"\nDry run — {args.readme} not modified", file=sys.stderr)
    else:
        args.readme.write_text(updated)
        print(f"Updated {args.readme}", file=sys.stderr)

    summary = data["summary"]
    rate = format_match_rate(summary["overall_match_rate"])
    print(f"Conformance: {rate} across {summary['total_repos']} repos", file=sys.stderr)


if __name__ == "__main__":
    main()
