#!/usr/bin/env python3
"""Corpus oracle runner — subcommands for corpus-oracle.yml workflow steps.

Subcommands:
    compute-matrix        Build the GitHub Actions batch matrix
    compute-batch-repos   Pick repos for a specific batch
    consolidate-cache     Merge per-batch rubocop results into SHA-indexed cache
    delete-crash-files    Delete known crash-prone files from a repo
    resolve-symlinks      Resolve symlink paths in nitrocop/RuboCop JSON output
    extract-context       Extract source context around offense locations
    validate-rubocop      Validate rubocop JSON contains only in-repo paths

Usage:
    python3 bench/corpus/oracle_runner.py compute-matrix --batch-size 30
    python3 bench/corpus/oracle_runner.py compute-batch-repos --batch-id 0 --batch-size 30
    python3 bench/corpus/oracle_runner.py consolidate-cache --results-dir results/rubocop --cache-dir rubocop-cache
    python3 bench/corpus/oracle_runner.py delete-crash-files --repo-id jruby__jruby__0303464 --repo-dir /tmp/repo
    python3 bench/corpus/oracle_runner.py resolve-symlinks results/nitrocop/repo.json results/rubocop/repo.json
    python3 bench/corpus/oracle_runner.py extract-context --nitrocop-json nc.json --rubocop-json rc.json --repo-dir /tmp/repo --output ctx.json
    python3 bench/corpus/oracle_runner.py validate-rubocop results/rubocop/repo.json /tmp/repo
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import sys
from pathlib import Path

CORPUS_DIR = Path(__file__).resolve().parent
MANIFEST_PATH = CORPUS_DIR / "manifest.jsonl"

# Repos with files that crash the Prism parser. rescue_parser_crashes.rb
# prevents crashes from killing the run, but crashy files produce 0-offense
# entries that create phantom FN. Deleting them before running eliminates
# the noise at the source.
CRASH_FILES: dict[str, list[str]] = {
    "jruby__jruby__0303464": [
        "bench/compiler/bench_compilation.rb",
        "test/jruby/test_regexp.rb",
    ],
    "ruby__logger__00796ec": [
        "test/",
    ],
    "infochimps-labs__wukong__437eff1": [
        "old/",
    ],
}


def load_manifest(repo_filter: str = "all") -> list[dict]:
    repos = []
    with open(MANIFEST_PATH) as f:
        for line in f:
            line = line.strip()
            if line:
                repos.append(json.loads(line))
    if repo_filter != "all":
        repos = [r for r in repos if r["id"] == repo_filter]
        if not repos:
            print(f"ERROR: no repo matching '{repo_filter}' in manifest", file=sys.stderr)
            sys.exit(1)
    return repos


def round_robin_batches(repos: list[dict], batch_size: int) -> list[list[dict]]:
    num_batches = (len(repos) + batch_size - 1) // batch_size
    batches: list[list[dict]] = [[] for _ in range(num_batches)]
    for i, repo in enumerate(repos):
        batches[i % num_batches].append(repo)
    return batches


def cmd_compute_matrix(args: argparse.Namespace) -> int:
    repos = load_manifest(args.repo_filter)
    rr_batches = round_robin_batches(repos, args.batch_size)

    matrix_entries = []
    for batch_idx, chunk in enumerate(rr_batches):
        if not chunk:
            continue
        batch_repo_ids = "|".join(sorted(r["id"] for r in chunk))
        batch_hash = hashlib.md5(batch_repo_ids.encode()).hexdigest()[:12]
        matrix_entries.append({
            "batch_id": str(batch_idx),
            "batch_hash": batch_hash,
            "num_batches": str(len(rr_batches)),
        })

    print(f"{len(repos)} repos in {len(matrix_entries)} batches (batch_size={args.batch_size})", file=sys.stderr)
    if len(matrix_entries) > 256:
        print(f"ERROR: {len(matrix_entries)} batches exceeds GitHub Actions matrix limit of 256. Increase --batch-size.", file=sys.stderr)
        return 1

    matrix = {"include": matrix_entries}
    output_file = os.environ.get("GITHUB_OUTPUT")
    if output_file:
        with open(output_file, "a") as out:
            out.write(f"matrix={json.dumps(matrix)}\n")
    else:
        print(json.dumps(matrix, indent=2))
    return 0


def cmd_compute_batch_repos(args: argparse.Namespace) -> int:
    repos = load_manifest(args.repo_filter)
    rr_batches = round_robin_batches(repos, args.batch_size)

    chunk = rr_batches[args.batch_id] if args.batch_id < len(rr_batches) else []
    output = [{"id": r["id"], "repo_url": r["repo_url"], "sha": r["sha"]} for r in chunk]

    with open(args.output, "w") as f:
        json.dump(output, f)
    print(f"Batch {args.batch_id}: {len(chunk)} repos")
    return 0


def cmd_consolidate_cache(args: argparse.Namespace) -> int:
    manifest: dict[str, str] = {}
    with open(MANIFEST_PATH) as f:
        for line in f:
            line = line.strip()
            if line:
                r = json.loads(line)
                manifest[r["id"]] = r["sha"]

    results_dir = Path(args.results_dir)
    cache_dir = Path(args.cache_dir)
    cache_dir.mkdir(parents=True, exist_ok=True)

    if not results_dir.is_dir():
        print("No rubocop results to consolidate")
        return 0

    count = 0
    for fname in results_dir.iterdir():
        if fname.suffix != ".json":
            continue
        repo_id = fname.stem
        sha = manifest.get(repo_id)
        if sha:
            shutil.copy2(str(fname), str(cache_dir / f"{repo_id}_{sha}.json"))
            count += 1
    print(f"Consolidated {count} rubocop results")
    return 0


def cmd_delete_crash_files(args: argparse.Namespace) -> int:
    paths = CRASH_FILES.get(args.repo_id, [])
    deleted = 0
    for rel in paths:
        target = Path(args.repo_dir) / rel
        if target.is_dir():
            shutil.rmtree(target, ignore_errors=True)
            deleted += 1
        elif target.exists():
            target.unlink()
            deleted += 1
    if deleted:
        print(f"Deleted {deleted} crash-prone path(s) from {args.repo_id}")
    return 0


# ── resolve-symlinks ────────────────────────────────────────────────


def _resolve_nitrocop_json(path: str) -> None:
    try:
        with open(path) as f:
            data = json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        return
    seen: set[tuple[str, int, str]] = set()
    deduped = []
    for o in data.get("offenses", []):
        filepath = o.get("path", "")
        if filepath and os.path.exists(filepath):
            o["path"] = os.path.realpath(filepath)
        key = (o.get("path", ""), o.get("line", 0), o.get("cop_name", ""))
        if key not in seen:
            seen.add(key)
            deduped.append(o)
    data["offenses"] = deduped
    with open(path, "w") as f:
        json.dump(data, f)


def _resolve_rubocop_json(path: str) -> None:
    try:
        with open(path) as f:
            data = json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        return
    by_resolved: dict[str, dict] = {}
    for file_entry in data.get("files", []):
        filepath = file_entry.get("path", "")
        resolved = os.path.realpath(filepath) if filepath and os.path.exists(filepath) else filepath
        if resolved in by_resolved:
            existing = by_resolved[resolved]
            existing_keys = {
                (o.get("location", {}).get("line", 0), o.get("cop_name", ""))
                for o in existing.get("offenses", [])
            }
            for o in file_entry.get("offenses", []):
                key = (o.get("location", {}).get("line", 0), o.get("cop_name", ""))
                if key not in existing_keys:
                    existing["offenses"].append(o)
                    existing_keys.add(key)
        else:
            file_entry["path"] = resolved
            by_resolved[resolved] = file_entry
    data["files"] = list(by_resolved.values())
    with open(path, "w") as f:
        json.dump(data, f)


def cmd_resolve_symlinks(args: argparse.Namespace) -> int:
    for path in args.paths:
        if not os.path.exists(path):
            continue
        try:
            with open(path) as f:
                data = json.load(f)
        except (json.JSONDecodeError, FileNotFoundError):
            continue
        if "offenses" in data:
            _resolve_nitrocop_json(path)
        elif "files" in data:
            _resolve_rubocop_json(path)
    return 0


# ── extract-context ─────────────────────────────────────────────────


def _strip_repo_prefix(filepath: str) -> str:
    parts = filepath.replace("\\", "/").split("/")
    for i, part in enumerate(parts):
        if part == "repos" and i + 1 < len(parts):
            return "/".join(parts[i + 2:])
    return filepath


def cmd_extract_context(args: argparse.Namespace) -> int:
    # Collect offenses from both tools
    tc_set: set[tuple[str, int, str]] = set()
    rc_set: set[tuple[str, int, str]] = set()

    try:
        nc_data = json.loads(args.nitrocop_json.read_text())
        for o in nc_data.get("offenses", []):
            fp = _strip_repo_prefix(o.get("path", ""))
            line = o.get("line", 0)
            cop = o.get("cop_name", "")
            if fp and cop and line > 0:
                tc_set.add((fp, line, cop))
    except (FileNotFoundError, json.JSONDecodeError):
        pass

    try:
        rc_data = json.loads(args.rubocop_json.read_text())
        for file_entry in rc_data.get("files", []):
            fp = _strip_repo_prefix(file_entry.get("path", ""))
            for o in file_entry.get("offenses", []):
                line = o.get("location", {}).get("line", 0)
                cop = o.get("cop_name", "")
                if fp and cop and line > 0:
                    rc_set.add((fp, line, cop))
    except (FileNotFoundError, json.JSONDecodeError):
        pass

    diverging = list((tc_set - rc_set) | (rc_set - tc_set))

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

    context_data: dict[str, dict] = {}
    for (filepath, line), _cops in sorted(locations.items()):
        source_path = args.repo_dir / filepath
        try:
            lines = source_path.read_text(errors="replace").splitlines()
        except (FileNotFoundError, OSError):
            continue
        start = max(0, line - 1 - args.context_lines)
        end = min(len(lines), line + args.context_lines)
        ctx = []
        for i in range(start, end):
            marker = ">>> " if i == line - 1 else "    "
            text = lines[i][:200] + "..." if len(lines[i]) > 200 else lines[i]
            ctx.append(f"{marker}{i + 1:>5}: {text}")
        context_data[f"{filepath}:{line}"] = {"context": ctx}

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(context_data) + "\n")
    print(f"Extracted context for {len(context_data)} locations from {args.repo_dir.name}", file=sys.stderr)
    return 0


# ── validate-rubocop ────────────────────────────────────────────────


def cmd_validate_rubocop(args: argparse.Namespace) -> int:
    prefix = args.repo_dir.rstrip("/") + "/"
    with open(args.result_json) as f:
        data = json.load(f)
    for fobj in data.get("files", []):
        path = fobj.get("path", "")
        if not path.startswith(prefix):
            print(f"POISONED: {path} not under {prefix}", file=sys.stderr)
            return 1
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Corpus oracle runner")
    subparsers = parser.add_subparsers(dest="command", required=True)

    # compute-matrix
    cm = subparsers.add_parser("compute-matrix", help="Build GitHub Actions batch matrix")
    cm.add_argument("--batch-size", type=int, default=30)
    cm.add_argument("--repo-filter", default=os.environ.get("REPO_FILTER", "all"))

    # compute-batch-repos
    cb = subparsers.add_parser("compute-batch-repos", help="Pick repos for a specific batch")
    cb.add_argument("--batch-id", type=int, required=True)
    cb.add_argument("--batch-size", type=int, default=30)
    cb.add_argument("--output", default="batch_repos.json")
    cb.add_argument("--repo-filter", default=os.environ.get("REPO_FILTER", "all"))

    # consolidate-cache
    cc = subparsers.add_parser("consolidate-cache", help="Merge per-batch rubocop results")
    cc.add_argument("--results-dir", required=True)
    cc.add_argument("--cache-dir", required=True)

    # delete-crash-files
    dc = subparsers.add_parser("delete-crash-files", help="Delete known crash-prone files")
    dc.add_argument("--repo-id", required=True)
    dc.add_argument("--repo-dir", required=True)

    # resolve-symlinks
    rs = subparsers.add_parser("resolve-symlinks", help="Resolve symlink paths in JSON output")
    rs.add_argument("paths", nargs="+", help="JSON output files to process")

    # extract-context
    ec = subparsers.add_parser("extract-context", help="Extract source context around offenses")
    ec.add_argument("--nitrocop-json", type=Path, required=True)
    ec.add_argument("--rubocop-json", type=Path, required=True)
    ec.add_argument("--repo-dir", type=Path, required=True)
    ec.add_argument("--output", type=Path, required=True)
    ec.add_argument("--context-lines", type=int, default=7)
    ec.add_argument("--max-per-cop", type=int, default=20)

    # validate-rubocop
    vr = subparsers.add_parser("validate-rubocop", help="Validate rubocop JSON paths")
    vr.add_argument("result_json", help="Path to rubocop result JSON")
    vr.add_argument("repo_dir", help="Expected repo directory prefix")

    args = parser.parse_args()
    commands = {
        "compute-matrix": cmd_compute_matrix,
        "compute-batch-repos": cmd_compute_batch_repos,
        "consolidate-cache": cmd_consolidate_cache,
        "delete-crash-files": cmd_delete_crash_files,
        "resolve-symlinks": cmd_resolve_symlinks,
        "extract-context": cmd_extract_context,
        "validate-rubocop": cmd_validate_rubocop,
    }
    return commands[args.command](args)


if __name__ == "__main__":
    raise SystemExit(main())
