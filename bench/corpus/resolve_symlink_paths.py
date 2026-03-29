#!/usr/bin/env python3
"""Resolve symlink paths in nitrocop/RuboCop JSON output files.

Both tools may report offenses through symlink paths (e.g., docs/gem/foo.gemspec
when the canonical path is gem/foo.gemspec). This creates phantom FP/FN in the
corpus oracle diff because the same physical file gets different path strings.

This script resolves all file paths in the JSON output to their realpath,
ensuring consistent paths regardless of how the file walker discovered the file.
It also deduplicates offenses that appear at the same (resolved_path, line, cop).

Usage:
    python3 resolve_symlink_paths.py results/nitrocop/repo.json results/rubocop/repo.json

Must be run while the repo is still on disk (before rm -rf).
"""

import json
import os
import sys


def resolve_nitrocop_json(path: str) -> None:
    """Resolve symlink paths in nitrocop JSON output."""
    try:
        with open(path) as f:
            data = json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        return

    offenses = data.get("offenses", [])
    seen = set()
    deduped = []
    for o in offenses:
        filepath = o.get("path", "")
        if filepath and os.path.exists(filepath):
            resolved = os.path.realpath(filepath)
            o["path"] = resolved
        # Use column in dedup key to preserve genuine multi-offenses at
        # different columns (e.g. two bad param names on one line) while
        # still collapsing symlink duplicates (same physical location).
        key = (o.get("path", ""), o.get("line", 0), o.get("cop_name", ""),
               o.get("column", 0))
        if key not in seen:
            seen.add(key)
            deduped.append(o)

    data["offenses"] = deduped
    with open(path, "w") as f:
        json.dump(data, f)


def resolve_rubocop_json(path: str) -> None:
    """Resolve symlink paths in RuboCop JSON output and deduplicate files."""
    try:
        with open(path) as f:
            data = json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        return

    files = data.get("files", [])
    # Group files by resolved path to merge offenses from symlink duplicates
    by_resolved: dict[str, dict] = {}
    for f in files:
        filepath = f.get("path", "")
        if filepath and os.path.exists(filepath):
            resolved = os.path.realpath(filepath)
        else:
            resolved = filepath

        if resolved in by_resolved:
            # Merge offenses, deduplicating by (line, cop, column) to preserve
            # genuine multi-offenses at different columns on the same line.
            existing = by_resolved[resolved]
            existing_keys = {
                (o.get("location", {}).get("line", 0), o.get("cop_name", ""),
                 o.get("location", {}).get("start_column", 0))
                for o in existing.get("offenses", [])
            }
            for o in f.get("offenses", []):
                key = (o.get("location", {}).get("line", 0), o.get("cop_name", ""),
                       o.get("location", {}).get("start_column", 0))
                if key not in existing_keys:
                    existing["offenses"].append(o)
                    existing_keys.add(key)
        else:
            f["path"] = resolved
            by_resolved[resolved] = f

    data["files"] = list(by_resolved.values())
    with open(path, "w") as f:
        json.dump(data, f)


def main():
    for path in sys.argv[1:]:
        if not os.path.exists(path):
            continue
        # Detect format by peeking at the JSON
        try:
            with open(path) as f:
                data = json.load(f)
        except (json.JSONDecodeError, FileNotFoundError):
            continue

        if "offenses" in data:
            resolve_nitrocop_json(path)
        elif "files" in data:
            resolve_rubocop_json(path)


if __name__ == "__main__":
    main()
