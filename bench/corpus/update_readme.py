#!/usr/bin/env python3
"""Update README.md conformance section from corpus oracle results.

Reads corpus-results.json and manifest.jsonl to update the conformance
percentages, offense counts, and top-15 repo table in README.md.

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
import re
import sys
from pathlib import Path


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


def format_offenses_summary(n: int) -> str:
    """Format offense count for summary line: 4989169 -> '5.0M'."""
    return f"{n / 1_000_000:.1f}M"


def format_match_rate(rate: float) -> str:
    """Format match rate: 0.938 -> '93.8%'."""
    return f"{rate:.1%}"


def build_top15_table(by_repo: list, manifest: dict[str, tuple[str, int]]) -> str:
    """Build the top-15 repos markdown table."""
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
            "offenses": total_offenses,
            "match_rate": repo["match_rate"],
        })

    # Filter to repos with meaningful offense counts (exclude trivial repos)
    enriched = [r for r in enriched if r["offenses"] >= 1000]

    # Sort by stars descending (most recognizable repos), take top 15
    enriched.sort(key=lambda x: x["stars"], reverse=True)
    top15 = enriched[:15]

    # Show Files column if per-repo file data is available
    has_files = any(r["files"] > 0 for r in top15)

    lines = []
    if has_files:
        lines.append("| Repo | Files | Offenses | Conformance % |")
        lines.append("|------|------:|---------:|--------------:|")
        for r in top15:
            name_link = f"[{r['name']}]({r['url']})"
            lines.append(f"| {name_link} | {r['files']:,} | {r['offenses']:,} | {format_match_rate(r['match_rate'])} |")
    else:
        lines.append("| Repo | Offenses | Conformance % |")
        lines.append("|------|---------:|--------------:|")
        for r in top15:
            name_link = f"[{r['name']}]({r['url']})"
            lines.append(f"| {name_link} | {r['offenses']:,} | {format_match_rate(r['match_rate'])} |")

    return "\n".join(lines)


def update_readme(readme_text: str, summary: dict, by_repo: list,
                  manifest: dict[str, tuple[str, int]]) -> str:
    """Replace conformance data in README text."""
    total_repos = summary["total_repos"]
    match_rate = summary["overall_match_rate"]
    offenses = summary["total_offenses_compared"]
    files = summary.get("total_files_inspected", 0)

    rate_str = format_match_rate(match_rate)
    offenses_str = format_offenses_summary(offenses)
    files_str = format_files(files) if files > 0 else None

    # 1. Features bullet: **XX.X% conformance**
    readme_text = re.sub(
        r"\*\*[\d.]+% conformance\*\*",
        f"**{rate_str} conformance**",
        readme_text,
    )

    # 2. Corpus description line: **500 open-source repos** (XXXk Ruby files)
    if files_str:
        readme_text = re.sub(
            r"\*\*\d+ open-source repos\*\* \(\d+k Ruby files\)",
            f"**{total_repos} open-source repos** ({files_str} Ruby files)",
            readme_text,
        )
    else:
        readme_text = re.sub(
            r"\*\*\d+ open-source repos\*\*",
            f"**{total_repos} open-source repos**",
            readme_text,
        )

    # 3. Overall line: **Overall: XX.X% match rate** across X.XM offenses compared.
    readme_text = re.sub(
        r"\*\*Overall: [\d.]+% match rate\*\* across [\d.]+M offenses compared\.",
        f"**Overall: {rate_str} match rate** across {offenses_str} offenses compared.",
        readme_text,
    )

    # 4. Replace the top-15 table (header + separator + data rows)
    new_table = build_top15_table(by_repo, manifest)
    readme_text = re.sub(
        r"\| Repo \| (?:Stars|Files|Offenses) [^\n]*\n\|[-| :]+\n(?:\| .+\n)*",
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
    summary = data["summary"]
    by_repo = data["by_repo"]

    manifest = load_manifest_stars(args.manifest)

    readme_text = args.readme.read_text()
    updated = update_readme(readme_text, summary, by_repo, manifest)

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

    rate = format_match_rate(summary["overall_match_rate"])
    print(f"Conformance: {rate} across {summary['total_repos']} repos", file=sys.stderr)


if __name__ == "__main__":
    main()
