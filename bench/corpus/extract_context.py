#!/usr/bin/env python3
"""Extract source context around offense locations before repos are deleted.

Reads nitrocop and rubocop JSON results, then reads the source files to capture
lines around each offense. Output is a JSON file with context snippets that
diff_results.py can merge into corpus-results.json.

Usage (in CI, before rm -rf of repo):
    python3 bench/corpus/extract_context.py \
        --nitrocop-json results/nitrocop/REPO_ID.json \
        --rubocop-json results/rubocop/REPO_ID.json \
        --repo-dir repos/REPO_ID \
        --output context/REPO_ID.json \
        --context-lines 7
"""

import argparse
import json
import sys
from pathlib import Path


def strip_repo_prefix(filepath: str) -> str:
    """Strip the repos/<id>/ prefix to get a path relative to the repo root."""
    parts = filepath.replace("\\", "/").split("/")
    for i, part in enumerate(parts):
        if part == "repos" and i + 1 < len(parts):
            return "/".join(parts[i + 2:])
    return filepath


def extract_offenses_from_nitrocop(path: Path) -> list[tuple[str, int, str]]:
    """Extract (filepath, line, cop) from nitrocop JSON."""
    try:
        data = json.loads(path.read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        return []
    result = []
    for o in data.get("offenses", []):
        filepath = strip_repo_prefix(o.get("path", ""))
        line = o.get("line", 0)
        cop = o.get("cop_name", "")
        if filepath and cop and line > 0:
            result.append((filepath, line, cop))
    return result


def extract_offenses_from_rubocop(path: Path) -> list[tuple[str, int, str]]:
    """Extract (filepath, line, cop) from rubocop JSON."""
    try:
        data = json.loads(path.read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        return []
    result = []
    for f in data.get("files", []):
        filepath = strip_repo_prefix(f.get("path", ""))
        for o in f.get("offenses", []):
            line = o.get("location", {}).get("line", 0)
            cop = o.get("cop_name", "")
            if filepath and cop and line > 0:
                result.append((filepath, line, cop))
    return result


def read_context(repo_dir: Path, filepath: str, line: int, context_lines: int) -> list[str] | None:
    """Read context_lines before and after the given line from a source file.

    Returns list of formatted lines like '  42: source_code_here', or None if
    the file can't be read."""
    source_path = repo_dir / filepath
    try:
        lines = source_path.read_text(errors="replace").splitlines()
    except (FileNotFoundError, OSError):
        return None

    start = max(0, line - 1 - context_lines)
    end = min(len(lines), line + context_lines)

    result = []
    for i in range(start, end):
        marker = ">>> " if i == line - 1 else "    "
        # Truncate very long lines to keep JSON size reasonable
        text = lines[i]
        if len(text) > 200:
            text = text[:200] + "..."
        result.append(f"{marker}{i + 1:>5}: {text}")
    return result


def main():
    parser = argparse.ArgumentParser(description="Extract source context around offenses")
    parser.add_argument("--nitrocop-json", type=Path, required=True)
    parser.add_argument("--rubocop-json", type=Path, required=True)
    parser.add_argument("--repo-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--context-lines", type=int, default=7,
                        help="Lines of context before/after offense (default: 7)")
    parser.add_argument("--max-per-cop", type=int, default=20,
                        help="Max examples with context per cop (default: 20)")
    args = parser.parse_args()

    # Collect all unique (filepath, line) pairs from both tools
    tc_offenses = extract_offenses_from_nitrocop(args.nitrocop_json)
    rc_offenses = extract_offenses_from_rubocop(args.rubocop_json)

    # Find diverging offenses (FP and FN only — not matches).
    # Context is most valuable for divergences, not agreements.
    tc_set = {(filepath, line, cop) for filepath, line, cop in tc_offenses}
    rc_set = {(filepath, line, cop) for filepath, line, cop in rc_offenses}
    fp_offenses = tc_set - rc_set  # nitrocop-only
    fn_offenses = rc_set - tc_set  # rubocop-only
    diverging = list(fp_offenses) + list(fn_offenses)

    # Limit context extraction per cop to keep JSON size manageable
    cop_counts: dict[str, int] = {}
    locations: dict[tuple[str, int], list[str]] = {}
    for filepath, line, cop in diverging:
        count = cop_counts.get(cop, 0)
        if count >= args.max_per_cop:
            continue
        cop_counts[cop] = count + 1
        key = (filepath, line)
        if key not in locations:
            locations[key] = []
        if cop not in locations[key]:
            locations[key].append(cop)

    # Extract context for each location
    # Output format: { "filepath:line": { "context": [...lines...] } }
    context_data = {}
    for (filepath, line), cops in sorted(locations.items()):
        ctx = read_context(args.repo_dir, filepath, line, args.context_lines)
        if ctx is not None:
            key = f"{filepath}:{line}"
            context_data[key] = {"context": ctx}

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(context_data) + "\n")

    print(f"Extracted context for {len(context_data)} locations from {args.repo_dir.name}",
          file=sys.stderr)


if __name__ == "__main__":
    main()
