#!/usr/bin/env python3
"""Generate variant batch configs from vendor defaults for corpus oracle runs.

Groups all (cop, style) variants into 3 batches where each batch overrides
cops simultaneously. Batch 1 sets each cop to its 2nd supported style,
batch 2 to the 3rd, and batch 3 to the 4th. Most cops have 2-3 styles,
so batch 1 covers all 160 configurable cops and batches 2-3 cover
progressively fewer.

Of 248 total non-default style alternatives across 171 style params:
  - 113 params have 1 alternative  → tested in batch 1 only
  - 43 params have 2 alternatives  → tested in batches 1-2
  - 15 params have 3+ alternatives → tested in batches 1-3

Only 3 params have 4+ alternatives, requiring us to drop 4 styles to
fit in 3 batches. We keep the first overflow alternative (alt[2]) per
param, which in all 3 cases is the most useful style to test:

  Cop / param                                 | Keep (batch 3)       | Drop (0 GitHub users)
  --------------------------------------------|----------------------|-------------------------
  Layout/EmptyLinesAroundClassBody             | empty_lines_special  | beginning_only, ending_only
  Style/EndlessMethod                          | require_single_line  | require_always
  Style/HashSyntax.EnforcedShorthandSyntax     | consistent (4 users) | either_consistent

Decision rationale (April 2026 GitHub code search):
  - beginning_only, ending_only: 0 repos use these in .rubocop.yml
  - require_always: 0 repos; reviewers on rubocop#11064 called it "relatively limited"
  - either_consistent: 0 repos; consistent has 4 users (incl. Cookpad)
  - empty_lines_special: 1 repo, but highest divergence (228k) so best for catching bugs

This gives 244/248 (98.4%) coverage of all supported style values.

Usage:
    python3 bench/corpus/gen_variant_batches.py --output-dir bench/corpus/variant_batches/
    python3 bench/corpus/gen_variant_batches.py --dry-run
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT / "scripts"))
from corpus_stress import VENDOR_CONFIGS, parse_enforced_styles  # noqa: E402

# Absolute path to the baseline config. RuboCop resolves inherit_from
# relative to the config file's directory, so a relative path only works
# when configs are generated in bench/corpus/variant_batches/. Using the
# absolute path makes it work from any output directory (e.g. temp dirs
# created by check_cop.py).
BASELINE_PATH = str(Path(__file__).resolve().parent / "baseline_rubocop.yml")


MAX_BATCHES = 3


def generate_batches(output_dir: Path, *, dry_run: bool = False) -> int:
    """Generate variant_batch_{n}.yml files.

    Produces at most MAX_BATCHES batches. Alternatives beyond batch 3 are
    folded into the last batch so that every supported style value is
    covered without adding extra CI jobs.

    Returns number of batches created.
    """
    all_styles: list[dict] = []
    for config_path, _plugin in VENDOR_CONFIGS:
        full_path = str(PROJECT_ROOT / config_path)
        all_styles.extend(parse_enforced_styles(full_path))

    # Find maximum number of alternatives across all styles
    max_alts = max((len(s["alternatives"]) for s in all_styles), default=0)
    if max_alts == 0:
        print("No style alternatives found in vendor configs.", file=sys.stderr)
        return 0

    # Group by cop for multi-key cops
    by_cop: dict[str, list[dict]] = {}
    for s in all_styles:
        by_cop.setdefault(s["cop"], []).append(s)

    # Build batch assignments: each batch gets a list of (cop, key, value, default).
    # Alternatives 0..MAX_BATCHES-2 go to batches 1..MAX_BATCHES-1 as before.
    # Alternatives MAX_BATCHES-1 and beyond are all folded into the last batch.
    # A YAML key can only appear once per cop per batch, so when multiple
    # overflow alternatives map to the last batch we must pick one.
    batch_entries: list[list[tuple[str, str, str, str]]] = [[] for _ in range(MAX_BATCHES)]

    for cop in sorted(by_cop.keys()):
        for s in by_cop[cop]:
            for alt_idx, alt in enumerate(s["alternatives"]):
                # Map alternative index to batch index, capping at last batch
                batch_idx = min(alt_idx, MAX_BATCHES - 1)
                batch_entries[batch_idx].append((cop, s["key"], alt, s["default"]))

    # Deduplicate last batch: if a (cop, key) appears multiple times
    # (from folding), keep only the FIRST (lowest-index) alternative.
    # This is alt[MAX_BATCHES-1] — the first overflow. For all 3 cops
    # that overflow, alt[2] is the most useful style to test (see module
    # docstring for rationale). Later alternatives are dropped.
    last = batch_entries[MAX_BATCHES - 1]
    seen: dict[tuple[str, str], int] = {}
    for i, (cop, key, value, default) in enumerate(last):
        if (cop, key) not in seen:
            seen[(cop, key)] = i
    batch_entries[MAX_BATCHES - 1] = [last[i] for i in sorted(seen.values())]

    batches_written = 0
    for batch_idx, entries in enumerate(batch_entries):
        if not entries:
            continue

        # Group entries by cop for YAML output
        cop_entries: dict[str, list[tuple[str, str, str]]] = {}
        for cop, key, value, default in entries:
            cop_entries.setdefault(cop, []).append((key, value, default))

        lines = [
            f"# Auto-generated variant batch {batch_idx + 1}:",
            f"# {len(cop_entries)} cops, {len(entries)} style overrides.",
            "# Generated by: python3 bench/corpus/gen_variant_batches.py",
            "",
            f"inherit_from: {BASELINE_PATH}",
            "",
        ]

        for cop in sorted(cop_entries.keys()):
            lines.append(f"{cop}:")
            for key, value, default in cop_entries[cop]:
                lines.append(f"  {key}: {value}  # default: {default}")
            lines.append("")

        content = "\n".join(lines) + "\n"
        filename = f"variant_batch_{batch_idx + 1}.yml"

        if dry_run:
            print(f"--- {filename} ({len(cop_entries)} cops, {len(entries)} overrides) ---")
            for line in content.splitlines()[:20]:
                print(f"  {line}")
            if len(content.splitlines()) > 20:
                print(f"  ... ({len(content.splitlines()) - 20} more lines)")
            print()
        else:
            output_dir.mkdir(parents=True, exist_ok=True)
            (output_dir / filename).write_text(content)
            print(f"Wrote {output_dir / filename} "
                  f"({len(cop_entries)} cops, {len(entries)} overrides)",
                  file=sys.stderr)

        batches_written += 1

    return batches_written


def main():
    parser = argparse.ArgumentParser(
        description="Generate variant batch configs from vendor defaults")
    parser.add_argument("--output-dir", type=Path,
                        default=Path(__file__).resolve().parent / "variant_batches",
                        help="Output directory for batch configs")
    parser.add_argument("--dry-run", action="store_true",
                        help="Preview without writing files")
    args = parser.parse_args()

    n = generate_batches(args.output_dir, dry_run=args.dry_run)
    print(f"\n{n} variant batch(es) generated.", file=sys.stderr)


if __name__ == "__main__":
    main()
