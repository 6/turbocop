#!/usr/bin/env python3
"""Tests for workflow_git.py."""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
from pathlib import Path
from unittest.mock import patch

SCRIPT = Path(__file__).parents[3] / "scripts" / "workflows" / "workflow_git.py"
sys.path.insert(0, str(SCRIPT.parent))

import workflow_git


def git(repo: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        cwd=str(repo),
        text=True,
        capture_output=True,
        check=check,
    )


def make_repo() -> Path:
    repo = Path(tempfile.mkdtemp())
    git(repo, "init")
    git(repo, "remote", "add", "origin", "https://github.com/example/repo.git")
    return repo


def test_configure_identity_only():
    repo = make_repo()
    subprocess.run(
        [sys.executable, str(SCRIPT), "configure", "--repo-root", str(repo)],
        check=True,
        text=True,
        capture_output=True,
    )
    assert git(repo, "config", "user.name").stdout.strip() == workflow_git.BOT_NAME
    assert git(repo, "config", "user.email").stdout.strip() == workflow_git.BOT_EMAIL
    assert git(repo, "remote", "get-url", "origin").stdout.strip() == "https://github.com/example/repo.git"


def test_configure_remote_and_unset_extraheader():
    repo = make_repo()
    git(repo, "config", "--local", workflow_git.EXTRAHEADER_KEY, "AUTH old")
    env = os.environ.copy()
    env["GH_TOKEN"] = "ghs_test_token"

    subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            "configure",
            "--repo-root",
            str(repo),
            "--repo",
            "6/nitrocop",
            "--unset-extraheader",
        ],
        check=True,
        text=True,
        capture_output=True,
        env=env,
    )

    assert git(repo, "config", "user.name").stdout.strip() == workflow_git.BOT_NAME
    assert git(repo, "config", "user.email").stdout.strip() == workflow_git.BOT_EMAIL
    assert (
        git(repo, "remote", "get-url", "origin").stdout.strip()
        == "https://x-access-token:ghs_test_token@github.com/6/nitrocop.git"
    )
    extraheader = git(repo, "config", "--local", "--get-all", workflow_git.EXTRAHEADER_KEY, check=False)
    assert extraheader.returncode != 0


def test_promote_branch_head_rewrites_ref_to_signed_commit():
    calls = []

    def fake_run_gh(args: list[str]) -> str:
        calls.append(args)
        path = args[0]
        if path.endswith("/git/ref/heads/test-branch"):
            return '{"object":{"sha":"unsigned123"}}'
        if path.endswith("/git/commits/unsigned123"):
            return '{"tree":{"sha":"tree456"},"parents":[{"sha":"parent789"}]}'
        if path.endswith("/git/commits"):
            return '{"sha":"signed999"}'
        if path.endswith("/git/refs/heads/test-branch"):
            return ""
        raise AssertionError(f"Unexpected gh api call: {args}")

    with patch.object(workflow_git, "run_gh", side_effect=fake_run_gh):
        result = workflow_git.promote("owner/repo", "test-branch", "Test message")

    assert result == {
        "unsigned_sha": "unsigned123",
        "signed_sha": "signed999",
        "tree_sha": "tree456",
        "parent_sha": "parent789",
    }
    assert calls[0] == ["repos/owner/repo/git/ref/heads/test-branch"]
    assert calls[1] == ["repos/owner/repo/git/commits/unsigned123"]
    assert calls[2][:5] == [
        "repos/owner/repo/git/commits",
        "-f",
        "message=Test message",
        "-f",
        "tree=tree456",
    ]
    assert "parents[]=parent789" in calls[2]
    assert calls[3] == [
        "repos/owner/repo/git/refs/heads/test-branch",
        "-X",
        "PATCH",
        "-f",
        "sha=signed999",
        "-F",
        "force=true",
    ]


if __name__ == "__main__":
    test_configure_identity_only()
    test_configure_remote_and_unset_extraheader()
    test_promote_branch_head_rewrites_ref_to_signed_commit()
    print("All tests passed.")
