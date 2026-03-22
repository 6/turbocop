#!/usr/bin/env python3
"""Replace a pushed branch head with a GitHub-signed commit of the same tree.

This lets workflows do local staging/build steps, push the result to a branch,
then immediately rewrite that branch to a verified `6[bot]` commit using the
GitHub App token.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys


def run_gh(args: list[str]) -> str:
    result = subprocess.run(
        ["gh", "api", *args],
        capture_output=True,
        text=True,
        check=True,
    )
    return result.stdout


def promote(repo: str, branch: str, message: str) -> dict[str, str]:
    ref = json.loads(run_gh([f"repos/{repo}/git/ref/heads/{branch}"]))
    unsigned_sha = ref["object"]["sha"]

    commit = json.loads(run_gh([f"repos/{repo}/git/commits/{unsigned_sha}"]))
    tree_sha = commit["tree"]["sha"]
    parent_shas = [parent["sha"] for parent in commit.get("parents", [])]

    create_args = [
        f"repos/{repo}/git/commits",
        "-f",
        f"message={message}",
        "-f",
        f"tree={tree_sha}",
    ]
    for parent_sha in parent_shas:
        create_args.extend(["-f", f"parents[]={parent_sha}"])

    signed = json.loads(run_gh(create_args))
    signed_sha = signed["sha"]

    run_gh([
        f"repos/{repo}/git/refs/heads/{branch}",
        "-X",
        "PATCH",
        "-f",
        f"sha={signed_sha}",
        "-F",
        "force=true",
    ])

    result = {
        "unsigned_sha": unsigned_sha,
        "signed_sha": signed_sha,
        "tree_sha": tree_sha,
    }
    if parent_shas:
        result["parent_sha"] = parent_shas[0]
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description="Promote a branch head to a verified GitHub commit")
    parser.add_argument("--repo", required=True, help="owner/repo")
    parser.add_argument("--branch", required=True, help="branch name")
    parser.add_argument("--message", required=True, help="final commit message")
    args = parser.parse_args()

    result = promote(args.repo, args.branch, args.message)
    for key, value in result.items():
        print(f"{key}={value}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
