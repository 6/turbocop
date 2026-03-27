#!/usr/bin/env python3
"""Wait for checks.yml on main to be green before proceeding.

Handles [skip ci] commits gracefully: if HEAD has no checks run and the
most recent checks run (for an older SHA) isn't pending, proceeds with
a warning instead of blocking.

Usage:
    python3 scripts/workflows/wait_healthy_main.py --repo OWNER/REPO [--max-wait 600] [--interval 30]
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time


def get_head_sha() -> str:
    result = subprocess.run(
        ["git", "rev-parse", "HEAD"], capture_output=True, text=True, check=True
    )
    return result.stdout.strip()


def get_latest_checks_run(repo: str) -> dict | None:
    result = subprocess.run(
        [
            "gh", "run", "list",
            "--workflow=checks.yml", "--branch=main",
            "--repo", repo, "--limit", "1",
            "--json", "headSha,conclusion,status",
        ],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        return None
    runs = json.loads(result.stdout)
    return runs[0] if runs else None


def main():
    parser = argparse.ArgumentParser(description="Wait for healthy main checks")
    parser.add_argument("--repo", required=True, help="GitHub repository (owner/repo)")
    parser.add_argument("--max-wait", type=int, default=900, help="Max wait seconds (default: 900)")
    parser.add_argument("--interval", type=int, default=30, help="Poll interval seconds (default: 30)")
    args = parser.parse_args()

    head_sha = get_head_sha()
    elapsed = 0

    while elapsed < args.max_wait:
        run = get_latest_checks_run(args.repo)

        if run is None:
            print("::notice::No checks.yml runs found — proceeding")
            return

        run_sha = run.get("headSha", "")
        conclusion = run.get("conclusion") or run.get("status") or "unknown"

        if conclusion == "success":
            print(f"::notice::Main Checks is green ({run_sha[:7]}) — proceeding")
            return

        # HEAD is a [skip ci] commit — checks.yml didn't run for it.
        # Don't wait for an older run that's already terminal.
        if run_sha != head_sha and conclusion not in ("in_progress", "queued"):
            print(
                f"::warning::Latest checks ({run_sha[:7]}) were {conclusion} "
                f"but HEAD ({head_sha[:7]}) has no checks — proceeding"
            )
            return

        print(
            f"Main Checks status: {conclusion} ({run_sha[:7]}) "
            f"— waiting {args.interval}s ({elapsed}s/{args.max_wait}s)"
        )
        time.sleep(args.interval)
        elapsed += args.interval

    print(f"::error::Main Checks did not go green within {args.max_wait}s (last: {conclusion})")
    sys.exit(1)


if __name__ == "__main__":
    main()
