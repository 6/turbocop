#!/usr/bin/env python3
"""Diff nitrocop vs RuboCop JSON results and produce a corpus report.

Usage:
    python3 bench/corpus/diff_results.py \
        --nitrocop-dir results/nitrocop \
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


def read_err_snippet(json_path: Path, tool: str) -> str:
    """Read the .err file next to a .json file and return the first meaningful error line.
    Returns empty string if no .err file or no meaningful content."""
    err_path = json_path.with_suffix(".err")
    try:
        text = err_path.read_text(errors="replace").strip()
    except FileNotFoundError:
        return ""
    if not text:
        return ""
    # Return first non-trivial line (skip blanks, "N files inspected" summaries,
    # and rubocop progress dots)
    for line in text.splitlines():
        line = line.strip()
        if not line:
            continue
        # Skip rubocop progress indicators (lines of just dots/letters like "..CC.W.")
        if all(c in ".CWEF" for c in line):
            continue
        # Skip summary lines like "1234 files inspected, ..."
        if "files inspected" in line or "offenses detected" in line:
            continue
        # Truncate long lines
        if len(line) > 200:
            line = line[:200] + "..."
        return line
    return ""


def parse_nitrocop_json(path: Path) -> set | None:
    """Parse nitrocop JSON output. Format: {"offenses": [...]}
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


def parse_rubocop_json(path: Path) -> tuple[set, set, int, int] | None:
    """Parse RuboCop JSON output. Format: {"files": [{"path": ..., "offenses": [...]}]}
    Returns (offenses, inspected_files, target_file_count, inspected_file_count)
    or None if the file is missing/empty/unparseable.
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
    zero_offense_files = set()
    for f in data.get("files", []):
        filepath = strip_repo_prefix(f.get("path", ""))
        if filepath:
            inspected_files.add(filepath)
            if not f.get("offenses"):
                zero_offense_files.add(filepath)
        for o in f.get("offenses", []):
            line = o.get("location", {}).get("line", 0)
            cop = o.get("cop_name", "")
            if filepath and cop:
                offenses.add((filepath, line, cop))

    # Detect parser crashes: when inspected_file_count < target_file_count,
    # RuboCop's parser crashed mid-batch and dropped files. Files that appear
    # in the output with 0 offenses may have been listed but not actually
    # analyzed (the crash causes them to be emitted with empty offense lists).
    # Exclude these suspicious zero-offense files from the inspected set so
    # nitrocop's correct offenses on them don't count as false positives.
    summary = data.get("summary", {})
    target = summary.get("target_file_count", 0)
    inspected = summary.get("inspected_file_count", 0)
    if target > 0 and inspected < target and zero_offense_files:
        inspected_files -= zero_offense_files

    return offenses, inspected_files, target, inspected


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
    parser.add_argument("--nitrocop-dir", required=True, type=Path)
    parser.add_argument("--rubocop-dir", required=True, type=Path)
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--output-json", required=True, type=Path)
    parser.add_argument("--output-md", required=True, type=Path)
    parser.add_argument("--cop-list", type=Path, help="File with one cop name per line (filter RuboCop to these)")
    args = parser.parse_args()

    manifest = load_manifest(args.manifest)
    manifest_ids = {r["id"] for r in manifest}

    # Load cop filter (only compare offenses from cops nitrocop knows about)
    covered_cops = None
    if args.cop_list and args.cop_list.exists():
        covered_cops = {line.strip() for line in args.cop_list.read_text().splitlines() if line.strip()}
        print(f"Filtering to {len(covered_cops)} covered cops", file=sys.stderr)

    # Collect all repo IDs that have results
    tc_files = {f.stem: f for f in args.nitrocop_dir.glob("*.json")} if args.nitrocop_dir.exists() else {}
    rc_files = {f.stem: f for f in args.rubocop_dir.glob("*.json")} if args.rubocop_dir.exists() else {}
    all_ids = sorted(set(tc_files.keys()) | set(rc_files.keys()))

    multi_repo = len(all_ids) > 1

    # Per-repo results
    repo_results = []
    by_cop_matches = defaultdict(int)
    by_cop_fp = defaultdict(int)  # nitrocop-only
    by_cop_fn = defaultdict(int)  # rubocop-only
    by_cop_fp_examples = defaultdict(list)  # (filepath, line) per cop
    by_cop_fn_examples = defaultdict(list)
    by_repo_cop = defaultdict(lambda: defaultdict(lambda: {"fp": 0, "fn": 0}))
    total_matches = 0
    total_fp = 0
    total_fn = 0
    repos_perfect = 0
    repos_error = 0
    total_files = 0
    total_files_dropped = 0
    warning_repos = []  # repos with partial RuboCop crashes (file drops)

    for repo_id in all_ids:
        tc_path = tc_files.get(repo_id)
        rc_path = rc_files.get(repo_id)

        if not tc_path or not rc_path:
            side = "nitrocop" if not tc_path else "rubocop"
            repo_results.append({
                "repo": repo_id,
                "status": "missing_results",
                "error_message": f"No {side} JSON output file",
                "match_rate": 0,
                "matches": 0,
                "fp": 0,
                "fn": 0,
            })
            repos_error += 1
            continue

        tc_offenses = parse_nitrocop_json(tc_path)
        rc_result = parse_rubocop_json(rc_path)

        # Detect crashed/empty output — don't compare against phantom zero offenses
        if tc_offenses is None or rc_result is None:
            side = "nitrocop" if tc_offenses is None else "rubocop"
            err_path = tc_path if tc_offenses is None else rc_path
            err_msg = read_err_snippet(err_path, side)
            repo_results.append({
                "repo": repo_id,
                "status": f"crashed_{side}",
                "error_message": err_msg,
                "match_rate": 0,
                "matches": 0,
                "fp": 0,
                "fn": 0,
            })
            repos_error += 1
            continue

        rc_offenses, rc_inspected_files, rc_target, rc_inspected = rc_result
        total_files += len(rc_inspected_files)

        # Filter to covered cops only (drop offenses from cops nitrocop doesn't implement)
        if covered_cops is not None:
            tc_offenses = {o for o in tc_offenses if o[2] in covered_cops}
            rc_offenses = {o for o in rc_offenses if o[2] in covered_cops}

        # Only compare files RuboCop actually inspected. RuboCop silently drops
        # files when its parser crashes mid-batch, producing phantom FPs for every
        # nitrocop offense on those dropped files.
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
        fp = tc_offenses - rc_offenses  # nitrocop-only (false positives)
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
            if multi_repo:
                by_repo_cop[repo_id][cop]["fp"] += 1
        for filepath, line, cop in fn:
            by_cop_fn[cop] += 1
            loc = f"{repo_id}: {filepath}:{line}" if multi_repo else f"{filepath}:{line}"
            by_cop_fn_examples[cop].append(loc)
            if multi_repo:
                by_repo_cop[repo_id][cop]["fn"] += 1

        result = {
            "repo": repo_id,
            "status": "ok",
            "match_rate": round(match_rate, 4),
            "matches": n_matches,
            "fp": n_fp,
            "fn": n_fn,
            "nitrocop_total": len(tc_offenses),
            "rubocop_total": len(rc_offenses),
            "files_inspected": len(rc_inspected_files),
        }

        # Track partial RuboCop crashes (parser errors that drop files)
        files_dropped = rc_target - rc_inspected if rc_target > rc_inspected > 0 else 0
        if files_dropped > 0:
            total_files_dropped += files_dropped
            err_msg = read_err_snippet(rc_path, "rubocop")
            result["rubocop_files_dropped"] = files_dropped
            result["rubocop_target_files"] = rc_target
            result["rubocop_inspected_files"] = rc_inspected
            if err_msg:
                result["rubocop_error"] = err_msg
            warning_repos.append(result)

        repo_results.append(result)

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
            "fp_examples": by_cop_fp_examples.get(cop, []),
            "fn_examples": by_cop_fn_examples.get(cop, []),
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
            "repos_with_rubocop_warnings": len(warning_repos),
            "total_offenses_compared": oracle_total,
            "matches": total_matches,
            "fp": total_fp,
            "fn": total_fn,
            "overall_match_rate": round(overall_rate, 4),
            "total_files_inspected": total_files,
            "rubocop_files_dropped": total_files_dropped,
        },
        "by_cop": by_cop,  # all cops (gen_tiers.py needs the full list)
        "by_repo": repo_results,
        "by_repo_cop": {repo: dict(cops) for repo, cops in by_repo_cop.items()},
    }
    args.output_json.write_text(json.dumps(json_output, indent=2) + "\n")

    # ── Write Markdown ──
    md = []
    md.append(f"# Corpus Oracle Results")
    md.append("")
    md.append("> Auto-generated by the [corpus oracle workflow](../.github/workflows/corpus-oracle.yml).")
    md.append(f"> Last updated: {datetime.now(timezone.utc).strftime('%Y-%m-%d')}")
    md.append("")
    md.append("Compares nitrocop against RuboCop on 500 open-source Ruby repos (167k files).")
    md.append("Every offense is compared by file path, line number, and cop name.")
    md.append("")

    md.append("## Summary")
    md.append("")
    md.append("| Metric | Value |")
    md.append("|--------|------:|")
    md.append(f"| Repos | {len(all_ids)} |")
    md.append(f"| Repos with 100% match | {repos_perfect} |")
    md.append(f"| Files inspected | {total_files:,} |")
    md.append(f"| Offenses compared | {oracle_total:,} |")
    md.append(f"| Matches (both agree) | {total_matches:,} |")
    md.append(f"| FP (nitrocop extra) | {total_fp:,} |")
    md.append(f"| FN (nitrocop missing) | {total_fn:,} |")
    md.append(f"| **Match rate** | **{overall_rate:.1%}** |")
    if repos_error > 0 or warning_repos:
        md.append(f"| Repos with errors | {repos_error} |")
    if warning_repos:
        md.append(f"| Repos with RuboCop parser crashes | {len(warning_repos)} |")
        md.append(f"| RuboCop files dropped (parser crash) | {total_files_dropped:,} |")
    md.append("")

    # ── RuboCop warnings (parser crashes, errors) ──
    warn_and_err = warning_repos + err_repos
    if warn_and_err:
        md.append("## RuboCop Warnings")
        md.append("")
        md.append(f"{len(warn_and_err)} repos had RuboCop issues (parser crashes or errors).")
        md.append("")
        md.append("| Repo | Issue | Files Dropped | Error |")
        md.append("|------|-------|--------------|-------|")
        for r in warning_repos:
            dropped = r.get("rubocop_files_dropped", 0)
            err = r.get("rubocop_error", "")
            err_cell = f"`{err}`" if err else ""
            md.append(f"| {r['repo']} | parser crash | {dropped:,} | {err_cell} |")
        for r in err_repos:
            err = r.get("error_message", "")
            err_cell = f"`{err}`" if err else ""
            md.append(f"| {r['repo']} | {r['status']} | all | {err_cell} |")
        md.append("")

    # ── Diverging cops with <details> for examples ──
    diverging = [c for c in by_cop if c["fp"] + c["fn"] > 0]
    perfect_cops = [c for c in by_cop if c["fp"] + c["fn"] == 0 and c["matches"] > 0]
    if diverging:
        md.append("## Diverging Cops")
        md.append("")
        md.append(f"{len(diverging)} cops have divergence. {len(perfect_cops)} cops match perfectly.")
        md.append("")
        md.append("| Cop | Matches | FP | FN | Match % |")
        md.append("|-----|--------:|---:|---:|--------:|")
        for c in diverging:
            total = c["matches"] + c["fn"]
            pct = f"{c['match_rate']:.1%}" if total > 0 else "N/A"
            md.append(f"| {c['cop']} | {c['matches']:,} | {c['fp']:,} | {c['fn']:,} | {pct} |")
        md.append("")

        # Expandable details per cop (show up to 20 examples in markdown)
        MD_EXAMPLE_LIMIT = 20
        for c in diverging:
            fp_list = c.get("fp_examples", [])
            fn_list = c.get("fn_examples", [])
            if not fp_list and not fn_list:
                continue
            total = c["matches"] + c["fn"]
            pct = f"{c['match_rate']:.1%}" if total > 0 else "N/A"
            md.append(f"<details>")
            md.append(f"<summary><strong>{c['cop']}</strong> — {c['matches']:,} matches, {c['fp']:,} FP, {c['fn']:,} FN ({pct})</summary>")
            md.append("")
            if fp_list:
                md.append("**False positives** (nitrocop reports, RuboCop does not):")
                md.append("")
                for ex in fp_list[:MD_EXAMPLE_LIMIT]:
                    md.append(f"- `{ex}`")
                if len(fp_list) > MD_EXAMPLE_LIMIT:
                    md.append(f"- ... and {len(fp_list) - MD_EXAMPLE_LIMIT:,} more (see corpus-results.json for full list)")
                md.append("")
            if fn_list:
                md.append("**False negatives** (RuboCop reports, nitrocop does not):")
                md.append("")
                for ex in fn_list[:MD_EXAMPLE_LIMIT]:
                    md.append(f"- `{ex}`")
                if len(fn_list) > MD_EXAMPLE_LIMIT:
                    md.append(f"- ... and {len(fn_list) - MD_EXAMPLE_LIMIT:,} more (see corpus-results.json for full list)")
                md.append("")
            md.append("</details>")
            md.append("")

    # ── Per-repo table ──
    ok_repos = [r for r in repo_results if r["status"] == "ok"]
    err_repos = [r for r in repo_results if r["status"] != "ok"]

    md.append("## Per-Repo Results")
    md.append("")
    md.append(f"{len(ok_repos)} repos completed successfully, {len(err_repos)} had errors.")
    md.append("")
    md.append("| Repo | Files | Match Rate | Matches | FP | FN |")
    md.append("|------|------:|----------:|--------:|---:|---:|")
    for r in sorted(ok_repos, key=lambda x: x.get("match_rate", 0)):
        files = r.get("files_inspected", 0)
        rate = f"{r['match_rate']:.1%}"
        md.append(f"| {r['repo']} | {files:,} | {rate} | {r['matches']:,} | {r['fp']:,} | {r['fn']:,} |")
    md.append("")

    if err_repos:
        md.append("<details>")
        md.append(f"<summary>Repos with errors ({len(err_repos)})</summary>")
        md.append("")
        md.append("| Repo | Status | Error |")
        md.append("|------|--------|-------|")
        for r in err_repos:
            err = r.get("error_message", "")
            err_cell = f"`{err}`" if err else ""
            md.append(f"| {r['repo']} | {r['status']} | {err_cell} |")
        md.append("")
        md.append("</details>")
        md.append("")

    # ── Perfect cops ──
    if perfect_cops:
        md.append("<details>")
        md.append(f"<summary>Perfect cops ({len(perfect_cops)} cops with 100% match rate)</summary>")
        md.append("")
        md.append("| Cop | Matches |")
        md.append("|-----|--------:|")
        for c in sorted(perfect_cops, key=lambda x: x["matches"], reverse=True):
            md.append(f"| {c['cop']} | {c['matches']:,} |")
        md.append("")
        md.append("</details>")
        md.append("")

    args.output_md.write_text("\n".join(md) + "\n")

    # Print summary to stderr
    print(f"\nCorpus: {len(all_ids)} repos, {repos_perfect} perfect, {repos_error} errors", file=sys.stderr)
    print(f"Offenses: {oracle_total:,} compared, {total_matches:,} match, {total_fp:,} FP, {total_fn:,} FN", file=sys.stderr)
    print(f"Overall match rate: {overall_rate:.1%}", file=sys.stderr)
    if warning_repos:
        print(f"RuboCop parser crashes: {len(warning_repos)} repos, {total_files_dropped:,} files dropped", file=sys.stderr)
        for r in warning_repos:
            err = r.get("rubocop_error", "unknown")
            print(f"  - {r['repo']}: {r.get('rubocop_files_dropped', 0)} files dropped ({err})", file=sys.stderr)

    # Exit 0 always for now — CI gating can be added later via --strict flag
    sys.exit(0)


if __name__ == "__main__":
    main()
