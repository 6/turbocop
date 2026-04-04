#!/usr/bin/env python3
"""Update README.md sections from corpus oracle results.

Reads corpus-results.json to update the generated Cops breakdown plus the
headline conformance percentage in README.md.

Usage:
    python3 bench/corpus/update_readme.py \
        --input corpus-results.json \
        --readme README.md

    # Include synthetic corpus results (fills in cops with no corpus data)
    python3 bench/corpus/update_readme.py \
        --input corpus-results.json \
        --synthetic bench/synthetic/synthetic-results.json

    # Dry run (print changes to stderr, don't write)
    python3 bench/corpus/update_readme.py --input corpus-results.json --dry-run
"""

from __future__ import annotations

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
    {
        "key": "rubocop-rake",
        "url": "https://github.com/rubocop/rubocop-rake",
        "departments": ["Rake"],
    },
]

def load_synthetic_results(path: Path) -> dict[str, dict]:
    """Load synthetic results and return a dict keyed by cop name.

    Returns dict mapping cop name to its synthetic entry (matches, fp, fn, etc).
    """
    data = json.loads(path.read_text())
    return {entry["cop"]: entry for entry in data.get("by_cop", [])}


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


def format_exact_match_pct(exact: int, total: int) -> str:
    """Format cop exact-match coverage across total cops."""
    if total <= 0:
        return "N/A"
    pct = format_match_rate(exact / total)
    return f"✓ {pct}" if exact == total else pct


def format_offense_match_pct(matches: int, fp: int, fn: int) -> str:
    """Format match rate across all compared issue reports."""
    total = matches + fp + fn
    if total <= 0:
        return "N/A"
    rate = matches / total
    pct = format_match_rate(rate)
    return f"✓ {pct}" if fp == 0 and fn == 0 else pct


def build_department_stats(data: dict, synthetic: dict[str, dict] | None = None) -> dict[str, dict]:
    """Build per-department cop counts for the generated README Cops section.

    When synthetic results are provided, cops with no corpus data are reclassified
    using their synthetic status (exact match or diverging).
    """
    derived: dict[str, dict] = {}
    for cop in data.get("by_cop", []):
        cop_name = cop["cop"]
        dept = cop_name.split("/")[0]
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
        stress_fp = cop.get("stress_fp", 0)
        stress_fn = cop.get("stress_fn", 0)
        seen = matches + fp + fn > 0
        diverging = (fp + fn > 0) or (stress_fp + stress_fn > 0)
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
        elif synthetic and cop_name in synthetic:
            # Cop had no corpus data but was exercised synthetically
            syn = synthetic[cop_name]
            if syn.get("perfect_match"):
                stats["perfect_cops"] += 1
            elif syn.get("diverging"):
                stats["diverging_cops"] += 1
            else:
                stats["no_data_cops"] += 1
        else:
            stats["no_data_cops"] += 1

    stats_by_department: dict[str, dict] = {}
    for entry in data.get("by_department", []):
        dept = entry["department"]
        derived_entry = derived.get(dept, {})
        has_derived = dept in derived
        stats_by_department[dept] = {
            "department": dept,
            "cops": entry.get("cops", derived_entry.get("cops", 0)),
            "seen_cops": entry.get("seen_cops", entry.get("exercised_cops", derived_entry.get("seen_cops", 0))),
            "perfect_cops": derived_entry.get("perfect_cops", 0) if has_derived else entry.get("perfect_cops", 0),
            "diverging_cops": derived_entry.get("diverging_cops", 0) if has_derived else entry.get("diverging_cops", 0),
            "no_data_cops": derived_entry.get("no_data_cops", 0) if has_derived else entry.get("inactive_cops", 0),
            "matches": entry.get("matches", derived_entry.get("matches", 0)),
            "fp": entry.get("fp", derived_entry.get("fp", 0)),
            "fn": entry.get("fn", derived_entry.get("fn", 0)),
            "variant_perfect_cops": entry.get("variant_perfect_cops"),
            "variant_diverging_cops": entry.get("variant_diverging_cops"),
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



def build_cops_section(data: dict, synthetic: dict[str, dict] | None = None) -> str:
    """Build the generated README Cops section."""
    summary = data.get("summary", {})
    baseline = data.get("baseline", {})
    by_department = build_department_stats(data, synthetic)

    total_cops = summary.get("registered_cops", sum(d["cops"] for d in by_department.values()))
    perfect_cops = sum(d["perfect_cops"] for d in by_department.values())
    diverging_cops = sum(d["diverging_cops"] for d in by_department.values())
    no_data_cops = sum(d["no_data_cops"] for d in by_department.values())
    has_variant_data = any(d.get("variant_perfect_cops") is not None for d in by_department.values())
    total_matches = summary.get("matches", 0)
    total_fp = summary.get("fp", 0)
    total_fn = summary.get("fn", 0)
    total_compared = total_matches + total_fp + total_fn
    total_repos = summary.get("total_repos", 0)
    total_files = summary.get("total_files_inspected", 0)
    files_str = format_files(total_files) if total_files > 0 else None

    lines = []
    lines.append(f"nitrocop supports {total_cops:,} cops from {len(GEMS)} RuboCop gems.")
    lines.append("")
    if total_repos > 0:
        corpus_line = "Compared with RuboCop on "
        corpus_line += f"[**{total_repos:,} open-source repos**](docs/corpus.md)"
        if files_str:
            corpus_line += f" ({files_str} Ruby files)"
        corpus_line += "."
        lines.append(corpus_line)
        lines.append("")
    variant_rate = summary.get("variant_overall_match_rate")
    if total_compared > 0:
        default_pct = format_offense_match_pct(total_matches, total_fp, total_fn)
        summary_line = (
            f"{default_pct} offense match rate "
            f"({format_count_summary(total_matches)} of {format_count_summary(total_compared)} offenses)"
        )
        if variant_rate is not None:
            summary_line += f", {format_match_rate(variant_rate)} across all style variants"
        summary_line += "."
        lines.append(summary_line)
        lines.append("")
        # Cop-level stats on a separate line for clarity
        summary_line = (
            f"{perfect_cops:,} of {total_cops:,} cops matched exactly (default config)"
        )
    else:
        summary_line = f"{perfect_cops:,} of {total_cops:,} cops matched exactly (default config)"
    if diverging_cops > 0:
        summary_line += f"; {diverging_cops:,} differed"
    if no_data_cops > 0:
        summary_line += f"; {no_data_cops:,} had no corpus data"
    summary_line += "."
    lines.append(summary_line)
    lines.append("")

    for gem in GEMS:
        rows = [by_department[dept] for dept in gem["departments"]]
        total = sum(r["cops"] for r in rows)
        perfect = sum(r["perfect_cops"] for r in rows)
        diverging = sum(r["diverging_cops"] for r in rows)
        no_data = sum(r["no_data_cops"] for r in rows)
        has_variant = any(r.get("variant_perfect_cops") is not None for r in rows)
        version = baseline.get(gem["key"], "?")
        lines.append(f"**[{gem['key']}]({gem['url']})** `{version}` ({total:,} cops)")
        lines.append("")

        # Build table header based on available columns
        hdr = "| Department | Cops | Exact match (default) | Differed |"
        sep = "|------------|-----:|----------------------:|---------:|"
        if no_data > 0:
            hdr += " No corpus data |"
            sep += "---------------:|"
        hdr += " Exact match % (default) |"
        sep += "------------------------:|"
        if has_variant:
            hdr += " Exact match % (all variants) |"
            sep += "-----------------------------:|"
        lines.append(hdr)
        lines.append(sep)

        for row in rows:
            cells = (
                f"| {row['department']} | {row['cops']:,} | "
                f"{row['perfect_cops']:,} | {row['diverging_cops']:,} |"
            )
            if no_data > 0:
                cells += f" {row['no_data_cops']:,} |"
            cells += f" {format_exact_match_pct(row['perfect_cops'], row['cops'])} |"
            if has_variant:
                vpc = row.get("variant_perfect_cops")
                if vpc is not None:
                    cells += f" {format_exact_match_pct(vpc, row['cops'])} |"
                else:
                    cells += " N/A |"
            lines.append(cells)

        if len(rows) > 1:
            cells = (
                f"| **Total** | **{total:,}** | **{perfect:,}** | "
                f"**{diverging:,}** |"
            )
            if no_data > 0:
                cells += f" **{no_data:,}** |"
            cells += f" **{format_exact_match_pct(perfect, total)}** |"
            if has_variant:
                gem_variant_perfect = sum(r.get("variant_perfect_cops", 0) for r in rows)
                cells += f" **{format_exact_match_pct(gem_variant_perfect, total)}** |"
            lines.append(cells)
        lines.append("")

    if has_variant_data:
        lines.append(
            "**Default** = tested with the default RuboCop config. "
            "**All variants** = tested with every supported `EnforcedStyle` value for each cop."
        )
    else:
        lines.append(
            "\"Exact match\" means nitrocop produced no extra issues and missed no issues "
            "for that cop anywhere in the corpus."
        )
    if no_data_cops > 0:
        lines.append("No corpus data means the cop never appeared in the corpus, so it has not been compared yet.")
    lines.append("See [docs/corpus.md](docs/corpus.md) for the full corpus breakdown.")

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


def update_readme(readme_text: str, data: dict, synthetic: dict[str, dict] | None = None) -> str:
    """Replace conformance data in README text."""
    summary = data["summary"]
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
        build_cops_section(data, synthetic),
    )

    # 1. Features bullet: **N cops** and **XX.X% conformance**
    total_cops = summary.get("registered_cops", 0)
    if total_cops > 0:
        readme_text = re.sub(
            r"\*\*[\d,]+ cops\*\*",
            f"**{total_cops:,} cops**",
            readme_text,
        )
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

    return readme_text


def main():
    parser = argparse.ArgumentParser(description="Update README.md corpus summary")
    parser.add_argument("--input", required=True, type=Path, help="Path to corpus-results.json")
    parser.add_argument("--manifest", type=Path, default=Path("bench/corpus/manifest.jsonl"),
                        help="Path to manifest.jsonl (deprecated; ignored)")
    parser.add_argument("--readme", type=Path, default=Path("README.md"),
                        help="Path to README.md")
    parser.add_argument("--synthetic", type=Path, default=None,
                        help="Path to synthetic-results.json (fills in cops with no corpus data)")
    parser.add_argument("--dry-run", action="store_true", help="Print diff to stderr without writing")
    args = parser.parse_args()

    data = json.loads(args.input.read_text())
    synthetic = load_synthetic_results(args.synthetic) if args.synthetic else None

    readme_text = args.readme.read_text()
    updated = update_readme(readme_text, data, synthetic)

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
