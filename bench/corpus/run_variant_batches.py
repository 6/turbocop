#!/usr/bin/env python3
"""Run style-variant batch configs on a single corpus repo.

Called from the corpus oracle workflow for each repo when style-variant
testing is enabled. Runs both nitrocop and rubocop with each variant
batch config and writes results to the output directories.

Usage (from workflow):
    python3 bench/corpus/run_variant_batches.py \\
        --repo-dir /path/to/repo \\
        --repo-id my_repo__abc123 \\
        --binary bin/nitrocop \\
        --batches-dir bench/corpus/variant_batches \\
        --results-dir results \\
        --timeout 600

Library usage:
    from run_variant_batches import run_variant_batches
    results = run_variant_batches(
        repo_dir="/path/to/repo",
        repo_id="my_repo__abc123",
        binary="bin/nitrocop",
        batches_dir="bench/corpus/variant_batches",
        results_dir="results",
    )
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

CORPUS_DIR = Path(__file__).resolve().parent


def discover_batches(batches_dir: str | Path) -> list[Path]:
    """Find all variant_batch_*.yml files in the given directory."""
    d = Path(batches_dir)
    if not d.exists():
        return []
    return sorted(d.glob("variant_batch_*.yml"))


def build_env(repo_dir: str) -> dict[str, str]:
    """Build environment matching the corpus oracle."""
    env = os.environ.copy()
    env["BUNDLE_GEMFILE"] = str(CORPUS_DIR / "Gemfile")
    env["BUNDLE_PATH"] = str(CORPUS_DIR / "vendor" / "bundle")
    env["GIT_CEILING_DIRECTORIES"] = str(Path(repo_dir).absolute().parent)
    return env


def _repo_manifest_index(repo_id: str, manifest: str | Path) -> int | None:
    """Return the 0-based index of *repo_id* in the manifest, or None."""
    with open(manifest) as f:
        for i, line in enumerate(f):
            line = line.strip()
            if not line:
                continue
            if json.loads(line).get("id") == repo_id:
                return i
    return None


def run_variant_batches(
    *,
    repo_dir: str,
    repo_id: str,
    binary: str,
    batches_dir: str,
    results_dir: str,
    timeout: int = 600,
    manifest: str | None = None,
    max_variant_repos: int | None = None,
) -> list[dict]:
    """Run all variant batch configs on a repo.

    If *manifest* and *max_variant_repos* are both provided, variants are
    only run when the repo's position in the manifest is < max_variant_repos.
    This keeps total runtime manageable by limiting variants to the top N
    repos (manifest is sorted by stars).

    Returns list of {batch_name, nitrocop_ok, rubocop_ok} dicts.
    """
    if manifest and max_variant_repos is not None:
        idx = _repo_manifest_index(repo_id, manifest)
        if idx is None or idx >= max_variant_repos:
            print(
                f"Skipping variants for {repo_id} (manifest index {idx}, limit {max_variant_repos})",
                file=sys.stderr,
            )
            return []

    batches = discover_batches(batches_dir)
    if not batches:
        return []

    abs_dest = str(Path(repo_dir).absolute())
    env = build_env(abs_dest)
    results = []

    for batch_config in batches:
        batch_name = batch_config.stem  # e.g. "variant_batch_1"
        nc_dir = Path(results_dir) / f"variant-nitrocop-{batch_name}"
        rc_dir = Path(results_dir) / f"variant-rubocop-{batch_name}"
        nc_dir.mkdir(parents=True, exist_ok=True)
        rc_dir.mkdir(parents=True, exist_ok=True)

        nc_json = nc_dir / f"{repo_id}.json"
        nc_err = nc_dir / f"{repo_id}.err"
        rc_json = rc_dir / f"{repo_id}.json"
        rc_err = rc_dir / f"{repo_id}.err"

        # nitrocop
        nc_ok = _run_tool(
            cmd=[
                binary, "--preview", "--format", "json", "--no-cache",
                "--config", str(batch_config), abs_dest,
            ],
            env=env, timeout=timeout,
            stdout_path=nc_json, stderr_path=nc_err,
            label=f"variant-nitrocop ({batch_name}): {repo_id}",
        )

        # rubocop
        rescue_file = CORPUS_DIR / "rescue_parser_crashes.rb"
        rc_ok = _run_tool(
            cmd=[
                "bundle", "exec", "rubocop",
                "--require", str(rescue_file),
                "--config", str(batch_config),
                "--format", "json", "--force-exclusion", "--cache", "false",
                abs_dest,
            ],
            env=env, timeout=timeout,
            stdout_path=rc_json, stderr_path=rc_err,
            label=f"variant-rubocop ({batch_name}): {repo_id}",
        )

        results.append({
            "batch_name": batch_name,
            "nitrocop_ok": nc_ok,
            "rubocop_ok": rc_ok,
        })

    return results


def _run_tool(
    *,
    cmd: list[str],
    env: dict[str, str],
    timeout: int,
    stdout_path: Path,
    stderr_path: Path,
    label: str,
) -> bool:
    """Run a tool, capturing stdout/stderr to files. Returns True on success."""
    print(f"=== {label} ===", file=sys.stderr)
    try:
        with open(stdout_path, "w") as out, open(stderr_path, "w") as err:
            result = subprocess.run(
                cmd, stdout=out, stderr=err, timeout=timeout, env=env,
            )
        return result.returncode in (0, 1)
    except subprocess.TimeoutExpired:
        print(f"::warning::{label} timed out after {timeout}s", file=sys.stderr)
        return False
    except FileNotFoundError as e:
        print(f"::warning::{label} command not found: {e}", file=sys.stderr)
        return False


def merge_variant_results(result_files: list[Path]) -> dict:
    """Merge multiple style-variant-*.json files into a single summary.

    Each input file is a diff_results.py output for one variant batch.
    Returns a combined dict with a 'batches' key.
    """
    merged: dict = {"batches": []}
    for f in sorted(result_files):
        try:
            data = json.loads(f.read_text())
        except (json.JSONDecodeError, OSError):
            continue
        batch_name = f.stem.replace("style-variant-", "")
        entry = {"name": batch_name}
        if "summary" in data:
            entry.update(data["summary"])
        if "by_cop" in data:
            entry["by_cop"] = data["by_cop"]
        merged["batches"].append(entry)
    return merged


def main():
    parser = argparse.ArgumentParser(
        description="Run style-variant batch configs on a corpus repo")
    parser.add_argument("--repo-dir", required=True, help="Path to corpus repo")
    parser.add_argument("--repo-id", required=True, help="Repo identifier")
    parser.add_argument("--binary", required=True, help="Path to nitrocop binary")
    parser.add_argument("--batches-dir", default="bench/corpus/variant_batches",
                        help="Directory containing variant_batch_*.yml files")
    parser.add_argument("--results-dir", default="results",
                        help="Base directory for result output")
    parser.add_argument("--timeout", type=int, default=600,
                        help="Timeout per tool per batch (default: 600s)")
    parser.add_argument("--manifest",
                        help="Path to manifest.jsonl for repo index lookup")
    parser.add_argument("--max-variant-repos", type=int,
                        help="Only run variants on repos in the first N manifest entries")
    args = parser.parse_args()

    results = run_variant_batches(
        repo_dir=args.repo_dir,
        repo_id=args.repo_id,
        binary=args.binary,
        batches_dir=args.batches_dir,
        results_dir=args.results_dir,
        timeout=args.timeout,
        manifest=args.manifest,
        max_variant_repos=args.max_variant_repos,
    )

    for r in results:
        nc_status = "ok" if r["nitrocop_ok"] else "FAIL"
        rc_status = "ok" if r["rubocop_ok"] else "FAIL"
        print(f"  {r['batch_name']}: nitrocop={nc_status} rubocop={rc_status}",
              file=sys.stderr)

    failures = sum(1 for r in results if not r["nitrocop_ok"] or not r["rubocop_ok"])
    sys.exit(1 if failures > 0 else 0)


if __name__ == "__main__":
    main()
