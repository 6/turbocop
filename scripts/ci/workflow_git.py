#!/usr/bin/env python3
"""Shared git helpers for workflow-authored branches and verified commits."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
from pathlib import Path


BOT_NAME = "6[bot]"
BOT_EMAIL = "129682364+6[bot]@users.noreply.github.com"
EXTRAHEADER_KEY = "http.https://github.com/.extraheader"


def run_git(repo_root: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        cwd=str(repo_root),
        text=True,
        capture_output=True,
        check=check,
    )


def run_gh(args: list[str]) -> str:
    result = subprocess.run(
        ["gh", "api", *args],
        capture_output=True,
        text=True,
        check=True,
    )
    return result.stdout


def configure_git(
    repo_root: Path,
    *,
    repo: str | None,
    token_env: str | None,
    unset_extraheader: bool,
) -> None:
    run_git(repo_root, "config", "user.name", BOT_NAME)
    run_git(repo_root, "config", "user.email", BOT_EMAIL)

    if unset_extraheader:
        run_git(repo_root, "config", "--local", "--unset-all", EXTRAHEADER_KEY, check=False)

    if repo and token_env:
        token = os.environ.get(token_env, "")
        if not token:
            raise SystemExit(f"{token_env} is required when --repo is set")
        remote = f"https://x-access-token:{token}@github.com/{repo}.git"
        run_git(repo_root, "remote", "set-url", "origin", remote)


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
    parser = argparse.ArgumentParser(description="Shared workflow git helpers")
    subparsers = parser.add_subparsers(dest="command", required=True)

    configure_parser = subparsers.add_parser("configure", help="Configure git bot identity and origin")
    configure_parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    configure_parser.add_argument("--repo", help="owner/repo for authenticated origin URL")
    configure_parser.add_argument(
        "--token-env",
        default="GH_TOKEN",
        help="Environment variable holding the GitHub token for origin auth",
    )
    configure_parser.add_argument(
        "--unset-extraheader",
        action="store_true",
        help="Remove checkout-injected GitHub auth header before setting origin URL",
    )

    promote_parser = subparsers.add_parser("promote", help="Promote branch head to a verified commit")
    promote_parser.add_argument("--repo", required=True, help="owner/repo")
    promote_parser.add_argument("--branch", required=True, help="branch name")
    promote_parser.add_argument("--message", required=True, help="final commit message")

    args = parser.parse_args()

    if args.command == "configure":
        configure_git(
            args.repo_root.resolve(),
            repo=args.repo,
            token_env=args.token_env if args.repo else None,
            unset_extraheader=args.unset_extraheader,
        )
        return 0

    if args.command == "promote":
        result = promote(args.repo, args.branch, args.message)
        for key, value in result.items():
            print(f"{key}={value}")
        return 0

    return 1


if __name__ == "__main__":
    raise SystemExit(main())
