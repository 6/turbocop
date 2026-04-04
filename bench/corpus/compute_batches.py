#!/usr/bin/env python3
"""Compute corpus oracle matrix batches and per-job repo lists.

Splits the manifest into round-robin batches for parallel CI execution.
Large repos (listed in large_repos.json) get their own dedicated matrix
jobs so they don't block smaller repos in the same batch.

Usage:
    # Generate the matrix JSON for GitHub Actions
    python3 bench/corpus/compute_batches.py matrix \
        --manifest bench/corpus/manifest.jsonl \
        --batch-size 30

    # Compute the repo list for a specific batch job
    python3 bench/corpus/compute_batches.py repos \
        --manifest bench/corpus/manifest.jsonl \
        --batch-id 5 --num-batches 187 --output batch_repos.json

    # Compute the repo list for a large-repo batch
    python3 bench/corpus/compute_batches.py repos \
        --manifest bench/corpus/manifest.jsonl \
        --batch-id "large-jruby__jruby__0303464" --is-large \
        --output batch_repos.json
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

CORPUS_DIR = Path(__file__).resolve().parent
LARGE_REPOS_PATH = CORPUS_DIR / "large_repos.json"


def load_manifest(path: str) -> list[dict]:
    repos = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line:
                repos.append(json.loads(line))
    return repos


def load_large_repo_ids(path: str | Path = LARGE_REPOS_PATH) -> set[str]:
    try:
        data = json.loads(Path(path).read_text())
        return set(data.get("repo_ids", []))
    except (FileNotFoundError, json.JSONDecodeError):
        return set()


def compute_matrix(
    repos: list[dict],
    batch_size: int = 30,
    large_repo_ids: set[str] | None = None,
) -> dict:
    """Compute the GitHub Actions matrix JSON.

    Returns {"include": [...]} with one entry per batch.
    Large repos get their own batch with is_large=true.
    """
    if large_repo_ids is None:
        large_repo_ids = set()

    large_repos = [r for r in repos if r["id"] in large_repo_ids]
    normal_repos = [r for r in repos if r["id"] not in large_repo_ids]

    num_batches = max(1, (len(normal_repos) + batch_size - 1) // batch_size) if normal_repos else 0

    batches = []
    for batch_idx in range(num_batches):
        # Compute stable hash from the repos that would be in this batch
        chunk = [normal_repos[i] for i in range(len(normal_repos)) if i % num_batches == batch_idx]
        if not chunk:
            continue
        batch_repo_ids = "|".join(sorted(r["id"] for r in chunk))
        batch_hash = hashlib.md5(batch_repo_ids.encode()).hexdigest()[:12]
        batches.append({
            "batch_id": str(batch_idx),
            "batch_hash": batch_hash,
            "num_batches": str(num_batches),
            "is_large": "false",
        })

    for repo in large_repos:
        batch_hash = hashlib.md5(repo["id"].encode()).hexdigest()[:12]
        batches.append({
            "batch_id": f"large-{repo['id']}",
            "batch_hash": batch_hash,
            "num_batches": str(num_batches),
            "is_large": "true",
        })

    return {
        "include": batches,
        "_stats": {
            "total_repos": len(repos),
            "normal_repos": len(normal_repos),
            "large_repos": len(large_repos),
            "normal_batches": num_batches,
            "total_batches": len(batches),
        },
    }


def compute_batch_repos(
    repos: list[dict],
    batch_id: str,
    num_batches: int,
    is_large: bool = False,
    large_repo_ids: set[str] | None = None,
) -> list[dict]:
    """Compute the repo list for a specific batch job.

    Returns list of {id, repo_url, sha} dicts.
    """
    if large_repo_ids is None:
        large_repo_ids = set()

    if is_large:
        # Large-repo batch: batch_id is "large-<repo_id>"
        repo_id = batch_id.replace("large-", "")
        return [
            {"id": r["id"], "repo_url": r["repo_url"], "sha": r["sha"]}
            for r in repos if r["id"] == repo_id
        ]

    # Normal batch: round-robin, excluding large repos
    normal_repos = [r for r in repos if r["id"] not in large_repo_ids]
    idx = int(batch_id)
    chunk = [normal_repos[i] for i in range(len(normal_repos)) if i % num_batches == idx]
    return [{"id": r["id"], "repo_url": r["repo_url"], "sha": r["sha"]} for r in chunk]


def main():
    parser = argparse.ArgumentParser(description="Compute corpus oracle batches")
    sub = parser.add_subparsers(dest="command", required=True)

    mat = sub.add_parser("matrix", help="Generate matrix JSON")
    mat.add_argument("--manifest", required=True)
    mat.add_argument("--batch-size", type=int, default=30)
    mat.add_argument("--repo-filter", default="all")
    mat.add_argument("--large-repos", default=str(LARGE_REPOS_PATH))

    rep = sub.add_parser("repos", help="Compute batch repo list")
    rep.add_argument("--manifest", required=True)
    rep.add_argument("--batch-id", required=True)
    rep.add_argument("--num-batches", type=int, required=True)
    rep.add_argument("--is-large", action="store_true")
    rep.add_argument("--repo-filter", default="all")
    rep.add_argument("--large-repos", default=str(LARGE_REPOS_PATH))
    rep.add_argument("--output", required=True)

    args = parser.parse_args()

    repos = load_manifest(args.manifest)
    if args.repo_filter != "all":
        repos = [r for r in repos if r["id"] == args.repo_filter]
        if not repos:
            print(f"ERROR: no repo matching '{args.repo_filter}' in manifest", file=sys.stderr)
            sys.exit(1)

    large_ids = load_large_repo_ids(args.large_repos)

    if args.command == "matrix":
        result = compute_matrix(repos, args.batch_size, large_ids)
        stats = result.pop("_stats")
        print(
            f"{stats['total_repos']} repos: {stats['normal_repos']} in "
            f"{stats['normal_batches']} batches + {stats['large_repos']} solo "
            f"large-repo jobs (batch_size={args.batch_size})",
            file=sys.stderr,
        )
        if len(result["include"]) > 256:
            print(
                f"ERROR: {len(result['include'])} batches exceeds GitHub Actions "
                f"matrix limit of 256.",
                file=sys.stderr,
            )
            sys.exit(1)
        print(json.dumps(result))

    elif args.command == "repos":
        chunk = compute_batch_repos(
            repos, args.batch_id, args.num_batches,
            is_large=args.is_large, large_repo_ids=large_ids,
        )
        with open(args.output, "w") as f:
            json.dump(chunk, f)
        print(f"Batch {args.batch_id}: {len(chunk)} repos", file=sys.stderr)


if __name__ == "__main__":
    main()
