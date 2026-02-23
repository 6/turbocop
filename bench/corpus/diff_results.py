#!/usr/bin/env python3
"""Diff turbocop vs RuboCop JSON results and produce a corpus report.

Usage:
    python3 bench/corpus/diff_results.py \
        --turbocop-dir results/turbocop \
        --rubocop-dir results/rubocop \
        --manifest bench/corpus/manifest.jsonl \
        --output-json corpus-results.json \
        --output-md corpus-results.md
"""

import argparse
import json
import os
import sys
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path


def strip_repo_prefix(filepath: str) -> str:
    """Strip the repos/<id>/ prefix to get a path relative to the repo root."""
    # Paths may look like: repos/mastodon__mastodon__c1f398a/app/models/user.rb
    # or /full/path/repos/mastodon__mastodon__c1f398a/app/models/user.rb
    # We want: app/models/user.rb
    parts = filepath.replace("\\", "/").split("/")
    # Find "repos" in the path and skip it + the repo id
    for i, part in enumerate(parts):
        if part == "repos" and i + 1 < len(parts):
            return "/".join(parts[i + 2:])
    return filepath


def parse_turbocop_json(path: Path) -> set | None:
    """Parse turbocop JSON output. Format: {"offenses": [...]}
    Returns None if the file is missing, empty, or unparseable (crash)."""
    try:
        text = path.read_text()
    except FileNotFoundError:
        return None
    if not text.strip():
        return None
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        return None

    offenses = set()
    for o in data.get("offenses", []):
        filepath = strip_repo_prefix(o.get("path", ""))
        line = o.get("line", 0)
        cop = o.get("cop_name", "")
        if filepath and cop:
            offenses.add((filepath, line, cop))
    return offenses


def parse_rubocop_json(path: Path) -> tuple[set, set] | None:
    """Parse RuboCop JSON output. Format: {"files": [{"path": ..., "offenses": [...]}]}
    Returns (offenses, inspected_files) or None if the file is missing/empty/unparseable.
    inspected_files is the set of relative file paths that RuboCop actually reported on.
    This is needed because RuboCop silently drops files when its parser crashes mid-batch."""
    try:
        text = path.read_text()
    except FileNotFoundError:
        return None
    if not text.strip():
        return None
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        return None

    offenses = set()
    inspected_files = set()
    for f in data.get("files", []):
        filepath = strip_repo_prefix(f.get("path", ""))
        if filepath:
            inspected_files.add(filepath)
        for o in f.get("offenses", []):
            line = o.get("location", {}).get("line", 0)
            cop = o.get("cop_name", "")
            if filepath and cop:
                offenses.add((filepath, line, cop))
    return offenses, inspected_files


def load_manifest(path: Path) -> list:
    """Load JSONL manifest."""
    repos = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line:
                repos.append(json.loads(line))
    return repos


def main():
    parser = argparse.ArgumentParser(description="Diff corpus oracle results")
    parser.add_argument("--turbocop-dir", required=True, type=Path)
    parser.add_argument("--rubocop-dir", required=True, type=Path)
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--output-json", required=True, type=Path)
    parser.add_argument("--output-md", required=True, type=Path)
    parser.add_argument("--cop-list", type=Path, help="File with one cop name per line (filter RuboCop to these)")
    args = parser.parse_args()

    manifest = load_manifest(args.manifest)
    manifest_ids = {r["id"] for r in manifest}

    # Load cop filter (only compare offenses from cops turbocop knows about)
    covered_cops = None
    if args.cop_list and args.cop_list.exists():
        covered_cops = {line.strip() for line in args.cop_list.read_text().splitlines() if line.strip()}
        print(f"Filtering to {len(covered_cops)} covered cops", file=sys.stderr)

    # Collect all repo IDs that have results
    tc_files = {f.stem: f for f in args.turbocop_dir.glob("*.json")} if args.turbocop_dir.exists() else {}
    rc_files = {f.stem: f for f in args.rubocop_dir.glob("*.json")} if args.rubocop_dir.exists() else {}
    all_ids = sorted(set(tc_files.keys()) | set(rc_files.keys()))

    multi_repo = len(all_ids) > 1

    # Per-repo results
    repo_results = []
    by_cop_matches = defaultdict(int)
    by_cop_fp = defaultdict(int)  # turbocop-only
    by_cop_fn = defaultdict(int)  # rubocop-only
    by_cop_fp_examples = defaultdict(list)  # (filepath, line) per cop
    by_cop_fn_examples = defaultdict(list)
    total_matches = 0
    total_fp = 0
    total_fn = 0
    repos_perfect = 0
    repos_error = 0

    for repo_id in all_ids:
        tc_path = tc_files.get(repo_id)
        rc_path = rc_files.get(repo_id)

        if not tc_path or not rc_path:
            repo_results.append({
                "repo": repo_id,
                "status": "missing_results",
                "match_rate": 0,
                "matches": 0,
                "fp": 0,
                "fn": 0,
            })
            repos_error += 1
            continue

        tc_offenses = parse_turbocop_json(tc_path)
        rc_result = parse_rubocop_json(rc_path)

        # Detect crashed/empty output — don't compare against phantom zero offenses
        if tc_offenses is None or rc_result is None:
            side = "turbocop" if tc_offenses is None else "rubocop"
            repo_results.append({
                "repo": repo_id,
                "status": f"crashed_{side}",
                "match_rate": 0,
                "matches": 0,
                "fp": 0,
                "fn": 0,
            })
            repos_error += 1
            continue

        rc_offenses, rc_inspected_files = rc_result

        # Filter to covered cops only (drop offenses from cops turbocop doesn't implement)
        if covered_cops is not None:
            tc_offenses = {o for o in tc_offenses if o[2] in covered_cops}
            rc_offenses = {o for o in rc_offenses if o[2] in covered_cops}

        # Only compare files RuboCop actually inspected. RuboCop silently drops
        # files when its parser crashes mid-batch, producing phantom FPs for every
        # turbocop offense on those dropped files.
        #
        # Root cause: Prism::Translation::Parser crashes on files with invalid
        # multibyte regex escapes (e.g., /\x9F/ in jruby's test_regexp.rb).
        # The unrescued RegexpError kills the worker, and all subsequent files in
        # that batch are silently omitted from the JSON output. Only 2-3 files
        # actually crash, but ~1000 are lost as collateral.
        #
        # Alternatives considered:
        # - Exclude crashing files in baseline_rubocop.yml: too repo-specific,
        #   and new crashing files could appear in future corpus additions.
        # - Re-run RuboCop file-by-file on dropped files: would recover ~1000
        #   files but adds significant CI complexity and runtime. Worth doing
        #   if we need coverage on those files for tier decisions.
        # - Fix upstream in parser gem / Prism translation layer: the crash is
        #   in Parser::Builders::Default#static_regexp which doesn't rescue
        #   RegexpError. Not something we control.
        if rc_inspected_files:
            tc_offenses = {o for o in tc_offenses if o[0] in rc_inspected_files}

        matches = tc_offenses & rc_offenses
        fp = tc_offenses - rc_offenses  # turbocop-only (false positives)
        fn = rc_offenses - tc_offenses  # rubocop-only (false negatives)

        n_matches = len(matches)
        n_fp = len(fp)
        n_fn = len(fn)
        total = n_matches + n_fn  # rubocop is the oracle
        match_rate = n_matches / total if total > 0 else 1.0

        total_matches += n_matches
        total_fp += n_fp
        total_fn += n_fn

        if n_fp == 0 and n_fn == 0:
            repos_perfect += 1

        # Per-cop aggregation
        for _, _, cop in matches:
            by_cop_matches[cop] += 1
        for filepath, line, cop in fp:
            by_cop_fp[cop] += 1
            loc = f"{repo_id}: {filepath}:{line}" if multi_repo else f"{filepath}:{line}"
            by_cop_fp_examples[cop].append(loc)
        for filepath, line, cop in fn:
            by_cop_fn[cop] += 1
            loc = f"{repo_id}: {filepath}:{line}" if multi_repo else f"{filepath}:{line}"
            by_cop_fn_examples[cop].append(loc)

        repo_results.append({
            "repo": repo_id,
            "status": "ok",
            "match_rate": round(match_rate, 4),
            "matches": n_matches,
            "fp": n_fp,
            "fn": n_fn,
            "turbocop_total": len(tc_offenses),
            "rubocop_total": len(rc_offenses),
        })

    # Build per-cop table (sorted by divergence descending)
    all_cops = sorted(set(by_cop_matches) | set(by_cop_fp) | set(by_cop_fn))
    by_cop = []
    for cop in all_cops:
        m = by_cop_matches.get(cop, 0)
        fp = by_cop_fp.get(cop, 0)
        fn = by_cop_fn.get(cop, 0)
        total = m + fn
        rate = m / total if total > 0 else 1.0
        by_cop.append({
            "cop": cop,
            "matches": m,
            "fp": fp,
            "fn": fn,
            "match_rate": round(rate, 4),
            "fp_examples": by_cop_fp_examples.get(cop, [])[:3],
            "fn_examples": by_cop_fn_examples.get(cop, [])[:3],
        })
    by_cop.sort(key=lambda x: x["fp"] + x["fn"], reverse=True)

    # Overall stats
    oracle_total = total_matches + total_fn
    overall_rate = total_matches / oracle_total if oracle_total > 0 else 1.0

    # ── Write JSON ──
    json_output = {
        "schema": 1,
        "run_date": datetime.now(timezone.utc).isoformat(),
        "baseline": {
            "rubocop": "1.84.2",
            "rubocop-rails": "2.34.3",
            "rubocop-performance": "1.26.1",
            "rubocop-rspec": "3.9.0",
            "rubocop-rspec_rails": "2.32.0",
            "rubocop-factory_bot": "2.28.0",
        },
        "summary": {
            "total_repos": len(all_ids),
            "repos_perfect": repos_perfect,
            "repos_error": repos_error,
            "total_offenses_compared": oracle_total,
            "matches": total_matches,
            "fp": total_fp,
            "fn": total_fn,
            "overall_match_rate": round(overall_rate, 4),
        },
        "by_cop": by_cop,  # all cops (gen_tiers.py needs the full list)
        "by_repo": repo_results,
    }
    args.output_json.write_text(json.dumps(json_output, indent=2) + "\n")

    # ── Write Markdown ──
    md = []
    md.append(f"# Corpus Oracle Results — {datetime.now(timezone.utc).strftime('%Y-%m-%d')}")
    md.append("")
    md.append(f"**Corpus**: {len(all_ids)} repos ({repos_perfect} perfect match)")
    md.append(f"**Baseline**: rubocop 1.84.2, rubocop-rails 2.34.3, rubocop-performance 1.26.1, rubocop-rspec 3.9.0")
    md.append("")
    md.append("## Overall")
    md.append("")
    md.append("| Metric | Count |")
    md.append("|--------|------:|")
    md.append(f"| Total repos | {len(all_ids)} |")
    md.append(f"| Repos with 100% match | {repos_perfect} |")
    md.append(f"| Repos with errors | {repos_error} |")
    md.append(f"| Total offenses compared | {oracle_total:,} |")
    md.append(f"| Matches | {total_matches:,} |")
    md.append(f"| FP (turbocop only) | {total_fp:,} |")
    md.append(f"| FN (rubocop only) | {total_fn:,} |")
    md.append(f"| Overall match rate | {overall_rate:.1%} |")
    md.append("")

    # Top diverging cops (only those with divergence)
    diverging = [c for c in by_cop if c["fp"] + c["fn"] > 0]
    if diverging:
        md.append("## Top Diverging Cops")
        md.append("")
        md.append("| Cop | Matches | FP | FN | Match % | FP examples | FN examples |")
        md.append("|-----|--------:|---:|---:|--------:|-------------|-------------|")
        for c in diverging[:30]:
            total = c["matches"] + c["fn"]
            pct = f"{c['match_rate']:.1%}" if total > 0 else "N/A"
            fp_ex = ", ".join(c.get("fp_examples", []))
            fn_ex = ", ".join(c.get("fn_examples", []))
            md.append(f"| {c['cop']} | {c['matches']} | {c['fp']} | {c['fn']} | {pct} | {fp_ex} | {fn_ex} |")
        md.append("")

    # Per-repo summary
    md.append("## Per-Repo Summary")
    md.append("")
    md.append("| Repo | Status | Match Rate | Matches | FP | FN |")
    md.append("|------|--------|----------:|--------:|---:|---:|")
    for r in sorted(repo_results, key=lambda x: x.get("match_rate", 0)):
        rate = f"{r['match_rate']:.1%}" if r["status"] == "ok" else "—"
        md.append(f"| {r['repo']} | {r['status']} | {rate} | {r['matches']} | {r['fp']} | {r['fn']} |")
    md.append("")

    args.output_md.write_text("\n".join(md) + "\n")

    # Print summary to stderr
    print(f"\nCorpus: {len(all_ids)} repos, {repos_perfect} perfect, {repos_error} errors", file=sys.stderr)
    print(f"Offenses: {oracle_total:,} compared, {total_matches:,} match, {total_fp:,} FP, {total_fn:,} FN", file=sys.stderr)
    print(f"Overall match rate: {overall_rate:.1%}", file=sys.stderr)

    # Exit 0 always for now — CI gating can be added later via --strict flag
    sys.exit(0)


if __name__ == "__main__":
    main()
