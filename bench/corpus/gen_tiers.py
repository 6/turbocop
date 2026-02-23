#!/usr/bin/env python3
"""Generate resources/tiers.json from corpus oracle results.

Reads corpus-results.json (output of diff_results.py) and classifies each cop
as stable or preview. Default tier is preview — cops must prove 0 false
positives across the corpus to be promoted to stable. This means new cops
and cops with no corpus data default to preview (gated behind --preview).

Usage:
    python3 bench/corpus/gen_tiers.py \
        --input corpus-results.json \
        --output src/resources/tiers.json

    # Dry run (print to stdout, don't write)
    python3 bench/corpus/gen_tiers.py --input corpus-results.json --dry-run
"""

import argparse
import json
import sys
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description="Generate tiers.json from corpus results")
    parser.add_argument("--input", required=True, type=Path, help="Path to corpus-results.json")
    parser.add_argument("--output", type=Path, help="Path to write tiers.json (default: src/resources/tiers.json)")
    parser.add_argument("--dry-run", action="store_true", help="Print result to stdout without writing")
    args = parser.parse_args()

    data = json.loads(args.input.read_text())
    by_cop = data.get("by_cop", [])

    stable_cops = []
    preview_cops = []

    for entry in by_cop:
        cop = entry["cop"]
        fp = entry.get("fp", 0)
        if fp == 0:
            stable_cops.append(cop)
        else:
            preview_cops.append(cop)

    # Default is preview; only allowlist stable cops as overrides
    overrides = {cop: "stable" for cop in sorted(stable_cops)}

    tiers = {
        "schema": 1,
        "default_tier": "preview",
        "overrides": overrides,
    }

    output_str = json.dumps(tiers, indent=2) + "\n"

    print(f"Corpus: {len(by_cop)} cops analyzed", file=sys.stderr)
    print(f"Stable: {len(stable_cops)}, Preview: {len(preview_cops)}", file=sys.stderr)

    if stable_cops:
        print(f"\nStable cops ({len(stable_cops)}):", file=sys.stderr)
        for cop in sorted(stable_cops):
            entry = next(e for e in by_cop if e["cop"] == cop)
            print(f"  {cop} ({entry['matches']} matches, {entry['fn']} FN)", file=sys.stderr)

    if args.dry_run:
        print(output_str)
    else:
        out_path = args.output or Path("src/resources/tiers.json")
        out_path.write_text(output_str)
        print(f"\nWrote {out_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
