#!/usr/bin/env python3
"""Generate cop → repo mappings for targeted corpus investigation.

Reads corpus-results.json (auto-downloaded from CI) and produces:
1. Per-cop: which repos have FP/FN, with example file:line locations
2. For targeted cloning: the minimal set of repos needed to investigate a cop

Usage:
    python3 scripts/corpus_repo_map.py                          # summary of all diverging cops
    python3 scripts/corpus_repo_map.py Metrics/BlockLength      # repos for one cop
    python3 scripts/corpus_repo_map.py --clone Metrics/BlockLength  # clone just those repos
    python3 scripts/corpus_repo_map.py --all > /tmp/cop-repo-map.json  # full JSON mapping
"""

import json
import os
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
CORPUS_CACHE = Path("/tmp/nitrocop-corpus-cache")
MANIFEST = PROJECT_ROOT / "bench" / "corpus" / "manifest.jsonl"
CORPUS_DIR = PROJECT_ROOT / "vendor" / "corpus"


def find_corpus_results() -> dict | None:
    """Find the latest corpus-results.json, downloading from CI if needed."""
    # Check cache first
    if CORPUS_CACHE.exists():
        files = sorted(CORPUS_CACHE.glob("corpus-results-[0-9]*.json"), reverse=True)
        if files:
            with open(files[0]) as f:
                return json.load(f)

    # Try downloading via investigate_cop.py's download logic
    try:
        subprocess.run(
            [sys.executable, str(PROJECT_ROOT / "scripts" / "investigate_cop.py"),
             "Style/FrozenStringLiteralComment", "--repos-only"],
            capture_output=True, text=True, timeout=60
        )
        # Re-check cache after download
        if CORPUS_CACHE.exists():
            files = sorted(CORPUS_CACHE.glob("corpus-results-[0-9]*.json"), reverse=True)
            if files:
                with open(files[0]) as f:
                    return json.load(f)
    except Exception:
        pass

    return None


def parse_example(ex) -> tuple[str, str]:
    """Parse example into (repo_id, location).

    Handles both old string format and new dict format."""
    loc = ex.get("loc", "") if isinstance(ex, dict) else ex
    parts = loc.split(": ", 1)
    if len(parts) == 2:
        return parts[0], parts[1]
    return loc, ""


def build_cop_repo_map(data: dict) -> dict:
    """Build a mapping of cop → {fp_repos: {repo: [locations]}, fn_repos: {repo: [locations]}}."""
    result = {}
    for cop_entry in data.get("by_cop", []):
        cop = cop_entry["cop"]
        if cop_entry.get("fp", 0) == 0 and cop_entry.get("fn", 0) == 0:
            continue

        fp_repos = defaultdict(list)
        fn_repos = defaultdict(list)

        for ex in cop_entry.get("fp_examples", []):
            repo_id, loc = parse_example(ex)
            fp_repos[repo_id].append(loc)

        for ex in cop_entry.get("fn_examples", []):
            repo_id, loc = parse_example(ex)
            fn_repos[repo_id].append(loc)

        result[cop] = {
            "fp": cop_entry.get("fp", 0),
            "fn": cop_entry.get("fn", 0),
            "matches": cop_entry.get("matches", 0),
            "fp_repos": dict(fp_repos),
            "fn_repos": dict(fn_repos),
        }

    return result


def load_manifest() -> dict[str, str]:
    """Load manifest.jsonl into {repo_id: repo_url}."""
    urls = {}
    if MANIFEST.exists():
        with open(MANIFEST) as f:
            for line in f:
                r = json.loads(line.strip())
                urls[r["id"]] = r["repo_url"]
    return urls


def print_cop_summary(cop_map: dict):
    """Print summary table of all diverging cops."""
    rows = []
    for cop, info in sorted(cop_map.items()):
        rows.append((cop, info["fp"], info["fn"], info["matches"],
                      len(info["fp_repos"]), len(info["fn_repos"])))

    rows.sort(key=lambda r: r[1] + r[2], reverse=True)

    print(f"{'Cop':<45} {'FP':>5} {'FN':>6} {'Match':>7} {'FP repos':>8} {'FN repos':>8}")
    print("-" * 82)
    for cop, fp, fn, matches, fp_r, fn_r in rows[:50]:
        print(f"{cop:<45} {fp:>5} {fn:>6} {matches:>7} {fp_r:>8} {fn_r:>8}")
    if len(rows) > 50:
        print(f"  ... and {len(rows) - 50} more")


def print_cop_detail(cop_map: dict, cop_name: str):
    """Print detailed repo breakdown for one cop."""
    info = cop_map.get(cop_name)
    if not info:
        print(f"{cop_name}: no divergence in corpus data (0 FP, 0 FN)")
        return

    print(f"{cop_name}")
    print(f"  Matches: {info['matches']}  FP: {info['fp']}  FN: {info['fn']}")

    if info["fp_repos"]:
        print(f"\n  FP repos ({len(info['fp_repos'])}):")
        for repo, locs in sorted(info["fp_repos"].items(), key=lambda x: -len(x[1])):
            print(f"    {repo} ({len(locs)} FP)")
            for loc in locs[:5]:
                print(f"      {loc}")
            if len(locs) > 5:
                print(f"      ... +{len(locs) - 5} more")

    if info["fn_repos"]:
        print(f"\n  FN repos ({len(info['fn_repos'])}):")
        for repo, locs in sorted(info["fn_repos"].items(), key=lambda x: -len(x[1])):
            print(f"    {repo} ({len(locs)} FN)")
            for loc in locs[:5]:
                print(f"      {loc}")
            if len(locs) > 5:
                print(f"      ... +{len(locs) - 5} more")

    all_repos = set(info["fp_repos"]) | set(info["fn_repos"])
    print(f"\n  Unique repos to clone: {len(all_repos)}")


def clone_repos_for_cop(cop_map: dict, cop_name: str):
    """Clone only the repos needed to investigate a cop."""
    info = cop_map.get(cop_name)
    if not info:
        print(f"{cop_name}: no divergence, nothing to clone")
        return

    all_repos = set(info["fp_repos"]) | set(info["fn_repos"])
    manifest = load_manifest()

    print(f"Cloning {len(all_repos)} repos for {cop_name}...")
    CORPUS_DIR.mkdir(parents=True, exist_ok=True)

    for repo_id in sorted(all_repos):
        dest = CORPUS_DIR / repo_id
        if dest.exists():
            print(f"  SKIP  {repo_id} (already cloned)")
            continue

        url = manifest.get(repo_id)
        if not url:
            print(f"  MISS  {repo_id} (not in manifest)")
            continue

        # Find SHA from manifest
        sha = None
        with open(MANIFEST) as f:
            for line in f:
                r = json.loads(line.strip())
                if r["id"] == repo_id:
                    sha = r["sha"]
                    break

        print(f"  CLONE {repo_id}...", end="", flush=True)
        try:
            subprocess.run(
                ["git", "clone", "--depth", "50", "--no-recurse-submodules",
                 "--single-branch", "-q", url, str(dest)],
                timeout=180, capture_output=True,
                env={**os.environ, "GIT_TERMINAL_PROMPT": "0", "GIT_ASKPASS": "/bin/false"}
            )
            if sha:
                subprocess.run(
                    ["git", "-C", str(dest), "checkout", "-q", sha],
                    timeout=60, capture_output=True
                )
            print(" OK")
        except subprocess.TimeoutExpired:
            print(" TIMEOUT")
        except Exception as e:
            print(f" FAIL ({e})")


def main():
    args = sys.argv[1:]
    do_clone = "--clone" in args
    do_all_json = "--all" in args
    args = [a for a in args if not a.startswith("--")]

    data = find_corpus_results()
    if not data:
        print("ERROR: No corpus-results.json found. Run investigate_cop.py first.", file=sys.stderr)
        sys.exit(1)

    cop_map = build_cop_repo_map(data)

    if do_all_json:
        json.dump(cop_map, sys.stdout, indent=2)
        return

    if args:
        cop_name = args[0]
        print_cop_detail(cop_map, cop_name)
        if do_clone:
            print()
            clone_repos_for_cop(cop_map, cop_name)
    else:
        print_cop_summary(cop_map)


if __name__ == "__main__":
    main()
