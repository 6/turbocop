#!/usr/bin/env python3
"""Stress-related corpus tooling.

Subcommands:
- `generate-config` writes flipped-style RuboCop configs
- `merge-report` folds stress results into corpus reports

Usage:
    python3 scripts/corpus_stress.py generate-config
    python3 scripts/corpus_stress.py merge-report --corpus corpus-results.json --stress stress-results.json
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

VENDOR_CONFIGS = [
    ("vendor/rubocop/config/default.yml", None),
    ("vendor/rubocop-rails/config/default.yml", "rubocop-rails"),
    ("vendor/rubocop-performance/config/default.yml", "rubocop-performance"),
    ("vendor/rubocop-rspec/config/default.yml", "rubocop-rspec"),
    ("vendor/rubocop-rspec_rails/config/default.yml", "rubocop-rspec_rails"),
    ("vendor/rubocop-factory_bot/config/default.yml", "rubocop-factory_bot"),
]

# The baseline config that the corpus oracle already uses
BASELINE_PATH = "bench/corpus/baseline_rubocop.yml"


def parse_enforced_styles(config_path: str) -> list[dict]:
    """Parse a vendor default.yml to find all Enforced* keys and their alternatives.

    Returns list of dicts: {cop, key, default, alternatives}
    Uses simple line-based parsing to avoid PyYAML dependency.
    """
    results = []
    path = Path(config_path)
    if not path.exists():
        return results

    current_cop = None
    enforced_key = None
    enforced_val = None
    supported_vals: list[str] = []
    in_supported = False

    for line in path.read_text().splitlines():
        # Top-level cop key (e.g., "Style/StringLiterals:")
        m = re.match(r"^([A-Z]\w+/\w+):", line)
        if m:
            # Flush previous
            if current_cop and enforced_key and enforced_val and supported_vals:
                alts = [v for v in supported_vals if v != enforced_val]
                if alts:
                    results.append({
                        "cop": current_cop,
                        "key": enforced_key,
                        "default": enforced_val,
                        "alternatives": alts,
                    })
            current_cop = m.group(1)
            enforced_key = None
            enforced_val = None
            supported_vals = []
            in_supported = False
            continue

        if not current_cop:
            continue

        # Another top-level key (AllCops:, etc.) — flush
        if re.match(r"^\S", line) and not line.startswith("#"):
            if enforced_key and enforced_val and supported_vals:
                alts = [v for v in supported_vals if v != enforced_val]
                if alts:
                    results.append({
                        "cop": current_cop,
                        "key": enforced_key,
                        "default": enforced_val,
                        "alternatives": alts,
                    })
            current_cop = None
            enforced_key = None
            enforced_val = None
            in_supported = False
            continue

        stripped = line.strip()

        # Enforced* key
        m = re.match(r"^  (Enforced\w+):\s*(.+)", line)
        if m:
            # Flush previous enforced key if any
            if enforced_key and enforced_val and supported_vals:
                alts = [v for v in supported_vals if v != enforced_val]
                if alts:
                    results.append({
                        "cop": current_cop,
                        "key": enforced_key,
                        "default": enforced_val,
                        "alternatives": alts,
                    })
            enforced_key = m.group(1)
            enforced_val = m.group(2).strip().strip("'\"")
            supported_vals = []
            in_supported = False
            continue

        # Supported* key (array start)
        m = re.match(r"^  (Supported\w+):", line)
        if m:
            in_supported = True
            supported_vals = []
            continue

        # Array item under Supported*
        if in_supported and stripped.startswith("- "):
            val = stripped[2:].strip().strip("'\"")
            supported_vals.append(val)
            continue

        # End of array
        if in_supported and not stripped.startswith("-") and not stripped.startswith("#") and stripped:
            in_supported = False

    # Flush last cop
    if current_cop and enforced_key and enforced_val and supported_vals:
        alts = [v for v in supported_vals if v != enforced_val]
        if alts:
            results.append({
                "cop": current_cop,
                "key": enforced_key,
                "default": enforced_val,
                "alternatives": alts,
            })

    return results


def generate_flipped_config(output_path: Path, dry_run: bool = False) -> int:
    """Generate a config with all EnforcedStyles flipped to non-default values."""
    all_styles = []
    for config_path, _plugin in VENDOR_CONFIGS:
        all_styles.extend(parse_enforced_styles(config_path))

    # Group by cop (some cops have multiple Enforced* keys)
    by_cop: dict[str, list[dict]] = {}
    for s in all_styles:
        by_cop.setdefault(s["cop"], []).append(s)

    lines = [
        "# Auto-generated stress-test config: all EnforcedStyles flipped to non-default.",
        "# This surfaces bugs where a cop only works with the default style.",
        "# Generated by: python3 scripts/corpus_stress.py generate-config",
        f"# Covers {len(all_styles)} EnforcedStyle keys across {len(by_cop)} cops.",
        "#",
        "# Usage: layer this on top of baseline_rubocop.yml in corpus runs.",
        "#   rubocop --config bench/corpus/flipped_styles.yml ...",
        "",
        "inherit_from: baseline_rubocop.yml",
        "",
    ]

    for cop in sorted(by_cop.keys()):
        styles = by_cop[cop]
        lines.append(f"{cop}:")
        for s in styles:
            # Pick the first alternative (most different from default)
            flipped = s["alternatives"][0]
            lines.append(f"  {s['key']}: {flipped}  # default: {s['default']}")
        lines.append("")

    content = "\n".join(lines) + "\n"

    if dry_run:
        print(content)
        print(f"# Would write {len(by_cop)} cops, {len(all_styles)} style keys", file=sys.stderr)
    else:
        output_path.write_text(content)
        print(f"Wrote {output_path} ({len(by_cop)} cops, {len(all_styles)} style keys)", file=sys.stderr)

    return len(all_styles)

def load_by_cop(data: dict) -> dict[str, dict]:
    return {
        cop["cop"]: {
            "matches": cop.get("matches", 0),
            "fp": cop.get("fp", 0),
            "fn": cop.get("fn", 0),
        }
        for cop in data.get("by_cop", [])
    }


def find_stress_failures(corpus_cops: dict, stress_cops: dict) -> list[dict]:
    failures = []
    for cop, baseline in corpus_cops.items():
        if baseline["fp"] == 0 and baseline["fn"] == 0:
            stress = stress_cops.get(cop, {})
            stress_fp = stress.get("fp", 0)
            stress_fn = stress.get("fn", 0)
            if stress_fp > 0 or stress_fn > 0:
                failures.append({
                    "cop": cop,
                    "stress_fp": stress_fp,
                    "stress_fn": stress_fn,
                    "stress_matches": stress.get("matches", 0),
                })
    failures.sort(key=lambda failure: -(failure["stress_fp"] + failure["stress_fn"]))
    return failures


def group_by_dept(failures: list[dict]) -> dict[str, list[dict]]:
    grouped: dict[str, list[dict]] = {}
    for failure in failures:
        grouped.setdefault(failure["cop"].split("/")[0], []).append(failure)
    return grouped


def print_summary(failures: list[dict], stress_repos: int) -> None:
    print(
        f"Stress test ({stress_repos} repos, flipped EnforcedStyles): "
        f"{len(failures)} cops break under non-default styles"
    )
    if not failures:
        return

    by_dept = group_by_dept(failures)
    print()
    print(f"{'Department':<20s} {'Failures':>8s} {'Total FP':>8s} {'Total FN':>8s}")
    print(f"{'-'*20} {'-'*8} {'-'*8} {'-'*8}")
    for dept in sorted(by_dept):
        cops = by_dept[dept]
        print(
            f"{dept:<20s} {len(cops):>8d} "
            f"{sum(c['stress_fp'] for c in cops):>8d} {sum(c['stress_fn'] for c in cops):>8d}"
        )
    print()
    print("Top 20 cops:")
    for failure in failures[:20]:
        print(f"  {failure['cop']:<50s} FP={failure['stress_fp']:>5d}  FN={failure['stress_fn']:>5d}")


def merge_json(corpus_data: dict, stress_data: dict, failures: list[dict], output: Path) -> None:
    stress_by_cop = {failure["cop"]: failure for failure in failures}
    for cop_entry in corpus_data.get("by_cop", []):
        if cop_entry["cop"] in stress_by_cop:
            failure = stress_by_cop[cop_entry["cop"]]
            cop_entry["stress_fp"] = failure["stress_fp"]
            cop_entry["stress_fn"] = failure["stress_fn"]

    corpus_data["stress_summary"] = {
        "repos_tested": stress_data.get("summary", {}).get("total_repos", 0),
        "config": "flipped_styles.yml",
        "failures": len(failures),
        "total_stress_fp": sum(failure["stress_fp"] for failure in failures),
        "total_stress_fn": sum(failure["stress_fn"] for failure in failures),
    }

    output.write_text(json.dumps(corpus_data, indent=None, separators=(",", ":")))
    print(f"Merged stress data into {output} ({len(failures)} failures)", file=sys.stderr)


def patch_summary_table(md_text: str, failures: list[dict], stress_repos: int) -> str:
    row = f"| Stress failures (flipped styles, {stress_repos} repos) | {len(failures)} |"
    lines = md_text.split("\n")
    insert_idx = None
    in_summary = False
    for index, line in enumerate(lines):
        if "| Metric |" in line:
            in_summary = True
        elif in_summary and line.strip() == "":
            insert_idx = index
            break
    if insert_idx is not None:
        lines.insert(insert_idx, row)
    return "\n".join(lines)


def append_stress_section(md_text: str, failures: list[dict], stress_repos: int) -> str:
    lines = [
        "",
        "## Stress Test (Flipped EnforcedStyles)",
        "",
        f"> {stress_repos} repos tested with all `EnforcedStyle` options set to non-default values.",
        f"> {len(failures)} cops that are baseline-perfect break under flipped styles.",
        "",
    ]
    if not failures:
        lines.append("All baseline-perfect cops also pass under flipped styles.")
        return md_text + "\n".join(lines) + "\n"

    by_dept = group_by_dept(failures)
    lines.append("| Department | Stress Failures | Stress FP | Stress FN |")
    lines.append("|:-----------|----------------:|----------:|----------:|")
    for dept in sorted(by_dept):
        cops = by_dept[dept]
        lines.append(
            f"| {dept} | {len(cops)} | {sum(c['stress_fp'] for c in cops):,} | {sum(c['stress_fn'] for c in cops):,} |"
        )
    lines.append("")
    lines.append("### Top Diverging Cops (Flipped Styles)")
    lines.append("")
    lines.append("| Cop | Stress FP | Stress FN |")
    lines.append("|:----|----------:|----------:|")
    for failure in failures[:30]:
        lines.append(f"| {failure['cop']} | {failure['stress_fp']:,} | {failure['stress_fn']:,} |")
    if len(failures) > 30:
        lines.append(f"| *... and {len(failures) - 30} more* | | |")
    lines.append("")
    return md_text + "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description="Stress-related corpus tooling")
    subparsers = parser.add_subparsers(dest="command", required=True)

    generate_parser = subparsers.add_parser("generate-config", help="Generate flipped-style configs")
    generate_parser.add_argument("--output-dir", type=str, default="bench/corpus", help="Output directory")
    generate_parser.add_argument("--dry-run", action="store_true", help="Preview without writing")

    merge_parser = subparsers.add_parser("merge-report", help="Merge stress results into corpus reports")
    merge_parser.add_argument("--corpus", required=True, type=Path, help="Main corpus-results.json")
    merge_parser.add_argument("--stress", type=Path, help="Stress-test stress-results.json")
    merge_parser.add_argument("--md", type=Path, help="Corpus markdown report to patch")
    merge_parser.add_argument("--output", type=Path, help="Write merged JSON here")
    merge_parser.add_argument("--summary", action="store_true", help="Print summary to stdout only")

    args = parser.parse_args()

    if args.command == "generate-config":
        out = Path(args.output_dir)
        print("Parsing vendor configs...", file=sys.stderr)
        styles = generate_flipped_config(out / "flipped_styles.yml", args.dry_run)
        print("\nSummary:", file=sys.stderr)
        print(f"  flipped_styles.yml: {styles} EnforcedStyle keys flipped", file=sys.stderr)
        return

    corpus_data = json.loads(args.corpus.read_text())
    corpus_cops = load_by_cop(corpus_data)
    if not args.stress or not args.stress.exists():
        if args.summary:
            print("No stress results available.")
        return

    stress_data = json.loads(args.stress.read_text())
    failures = find_stress_failures(corpus_cops, load_by_cop(stress_data))
    stress_repos = stress_data.get("summary", {}).get("total_repos", 0)

    if args.summary:
        print_summary(failures, stress_repos)
        return

    merge_json(corpus_data, stress_data, failures, args.output or args.corpus)
    if args.md and args.md.exists():
        md_text = args.md.read_text()
        md_text = patch_summary_table(md_text, failures, stress_repos)
        md_text = append_stress_section(md_text, failures, stress_repos)
        args.md.write_text(md_text)
        print(f"Patched {args.md} with stress test section", file=sys.stderr)


if __name__ == "__main__":
    main()
