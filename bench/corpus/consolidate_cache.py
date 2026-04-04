#!/usr/bin/env python3
"""Consolidate per-batch rubocop result files into SHA-indexed cache directories.

Used by the corpus oracle workflow to merge per-batch artifacts into a single
cache that can be restored by future runs, keyed by (repo_id, sha) for baseline
results and (repo_id, sha, batch_name) for variant results.

Usage:
    # Consolidate baseline rubocop results
    python3 bench/corpus/consolidate_cache.py baseline \
        --manifest bench/corpus/manifest.jsonl \
        --source all-results/results/rubocop \
        --dest rubocop-cache

    # Consolidate variant rubocop results
    python3 bench/corpus/consolidate_cache.py variants \
        --manifest bench/corpus/manifest.jsonl \
        --source all-results/results \
        --dest variant-rubocop-cache
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import sys


def load_manifest_shas(manifest_path: str) -> dict[str, str]:
    """Load {repo_id: sha} mapping from manifest.jsonl."""
    result: dict[str, str] = {}
    with open(manifest_path) as f:
        for line in f:
            line = line.strip()
            if line:
                r = json.loads(line)
                result[r["id"]] = r["sha"]
    return result


def consolidate_baseline(
    manifest_path: str,
    source_dir: str,
    dest_dir: str,
) -> int:
    """Consolidate baseline rubocop results into {repo_id}_{sha}.json cache files.

    Returns number of files consolidated.
    """
    manifest = load_manifest_shas(manifest_path)
    os.makedirs(dest_dir, exist_ok=True)

    if not os.path.isdir(source_dir):
        print("No rubocop results to consolidate", file=sys.stderr)
        return 0

    count = 0
    for fname in os.listdir(source_dir):
        if not fname.endswith(".json"):
            continue
        repo_id = fname[:-5]
        sha = manifest.get(repo_id)
        if sha:
            shutil.copy2(
                os.path.join(source_dir, fname),
                os.path.join(dest_dir, f"{repo_id}_{sha}.json"),
            )
            count += 1
    return count


def consolidate_variants(
    manifest_path: str,
    source_dir: str,
    dest_dir: str,
) -> int:
    """Consolidate variant rubocop results into {repo_id}_{sha}_{batch_name}.json cache files.

    Scans source_dir for directories matching variant-rubocop-variant_batch_*.

    Returns number of files consolidated.
    """
    manifest = load_manifest_shas(manifest_path)
    os.makedirs(dest_dir, exist_ok=True)

    count = 0
    try:
        entries = os.listdir(source_dir)
    except FileNotFoundError:
        print("No variant results to consolidate", file=sys.stderr)
        return 0

    for batch_dir in sorted(
        d for d in entries if d.startswith("variant-rubocop-variant_batch_")
    ):
        batch_name = batch_dir.replace("variant-rubocop-", "")
        src = os.path.join(source_dir, batch_dir)
        for fname in os.listdir(src):
            if not fname.endswith(".json"):
                continue
            repo_id = fname[:-5]
            sha = manifest.get(repo_id)
            if sha:
                shutil.copy2(
                    os.path.join(src, fname),
                    os.path.join(dest_dir, f"{repo_id}_{sha}_{batch_name}.json"),
                )
                count += 1
    return count


def main():
    parser = argparse.ArgumentParser(description="Consolidate rubocop cache")
    sub = parser.add_subparsers(dest="command", required=True)

    base = sub.add_parser("baseline", help="Consolidate baseline rubocop results")
    base.add_argument("--manifest", required=True)
    base.add_argument("--source", required=True)
    base.add_argument("--dest", required=True)

    var = sub.add_parser("variants", help="Consolidate variant rubocop results")
    var.add_argument("--manifest", required=True)
    var.add_argument("--source", required=True)
    var.add_argument("--dest", required=True)

    args = parser.parse_args()

    if args.command == "baseline":
        count = consolidate_baseline(args.manifest, args.source, args.dest)
        print(f"Consolidated {count} rubocop results")
    elif args.command == "variants":
        count = consolidate_variants(args.manifest, args.source, args.dest)
        print(f"Consolidated {count} variant rubocop results")


if __name__ == "__main__":
    main()
