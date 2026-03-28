#!/usr/bin/env python3
"""Tests for remote repair publish request generation."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parents[3]
SCRIPT = ROOT / "scripts" / "workflows" / "repair_publish.py"


def _run(*args: str, cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(SCRIPT), *args],
        cwd=str(cwd),
        text=True,
        capture_output=True,
        check=True,
    )


def _init_repo(path: Path) -> tuple[str, str]:
    subprocess.run(["git", "init", "-b", "main"], cwd=path, text=True, capture_output=True, check=True)
    subprocess.run(["git", "config", "user.name", "Test User"], cwd=path, text=True, capture_output=True, check=True)
    subprocess.run(
        ["git", "config", "user.email", "test@example.com"],
        cwd=path,
        text=True,
        capture_output=True,
        check=True,
    )

    (path / "base.txt").write_text("base\n")
    subprocess.run(["git", "add", "base.txt"], cwd=path, text=True, capture_output=True, check=True)
    subprocess.run(["git", "commit", "-m", "base"], cwd=path, text=True, capture_output=True, check=True)
    base_sha = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=path,
        text=True,
        capture_output=True,
        check=True,
    ).stdout.strip()

    (path / "added.txt").write_text("added\n")
    subprocess.run(["git", "add", "added.txt"], cwd=path, text=True, capture_output=True, check=True)
    subprocess.run(["git", "commit", "-m", "head"], cwd=path, text=True, capture_output=True, check=True)
    head_sha = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=path,
        text=True,
        capture_output=True,
        check=True,
    ).stdout.strip()
    return base_sha, head_sha


def test_skip_request_includes_pr_and_issue_operations(tmp_path: Path) -> None:
    output_dir = tmp_path / "out"

    _run(
        "skip-request",
        "--output-dir",
        str(output_dir),
        "--pr-number",
        "12",
        "--linked-issue-number",
        "34",
        "--heading",
        "Automatic PR repair stopped",
        "--reason",
        "Retry limit hit",
        "--checks-run-id",
        "99",
        "--checks-url",
        "https://example.invalid/checks/99",
        "--backend-label",
        "Claude",
        "--route",
        "single-cop",
        "--run-id",
        "123",
        "--run-url",
        "https://example.invalid/runs/123",
        "--target-sha",
        "abc123",
        "--target-ref",
        "refs/heads/fix/example",
        "--needs-human",
        cwd=ROOT,
    )

    request = json.loads((output_dir / "request.json").read_text())
    assert [operation["type"] for operation in request["operations"]] == [
        "comment_pr",
        "comment_issue",
        "edit_issue_labels",
    ]
    assert "Automatic PR repair stopped" in (output_dir / "pr-comment.md").read_text()
    assert "linked PR #12" in (output_dir / "issue-comment.md").read_text()


def test_result_request_pushable_writes_patch_and_signed_sha_placeholder(tmp_path: Path) -> None:
    repo_root = tmp_path / "repo"
    output_dir = tmp_path / "out"
    repo_root.mkdir()
    base_sha, head_sha = _init_repo(repo_root)

    _run(
        "result-request",
        "--output-dir",
        str(output_dir),
        "--repo-root",
        str(repo_root),
        "--result",
        "pushable",
        "--pr-number",
        "12",
        "--checks-run-id",
        "99",
        "--checks-url",
        "https://example.invalid/checks/99",
        "--backend-label",
        "Claude",
        "--model-label",
        "claude-sonnet",
        "--backend-name",
        "claude-normal",
        "--run-id",
        "123",
        "--run-url",
        "https://example.invalid/runs/123",
        "--base-sha",
        base_sha,
        "--target-sha",
        head_sha,
        "--target-ref",
        "refs/heads/fix/example",
        cwd=ROOT,
    )

    request = json.loads((output_dir / "request.json").read_text())
    assert [operation["type"] for operation in request["operations"]] == ["push_patch", "comment_pr"]
    patch_file = output_dir / request["operations"][0]["patch_file"]
    assert patch_file.is_file()
    assert "diff --git" in patch_file.read_text()
    assert "{{SIGNED_SHA}}" in (output_dir / "pr-comment.md").read_text()


def test_result_request_nonpushable_updates_linked_issue(tmp_path: Path) -> None:
    repo_root = tmp_path / "repo"
    output_dir = tmp_path / "out"
    repo_root.mkdir()
    base_sha, _head_sha = _init_repo(repo_root)
    verify_log = tmp_path / "verify.log"
    verify_log.write_text("first line\nlast line\n")

    _run(
        "result-request",
        "--output-dir",
        str(output_dir),
        "--repo-root",
        str(repo_root),
        "--result",
        "verify_failed",
        "--pr-number",
        "12",
        "--linked-issue-number",
        "34",
        "--checks-run-id",
        "99",
        "--checks-url",
        "https://example.invalid/checks/99",
        "--backend-label",
        "Claude",
        "--model-label",
        "claude-sonnet",
        "--backend-name",
        "claude-normal",
        "--run-id",
        "123",
        "--run-url",
        "https://example.invalid/runs/123",
        "--reason",
        "Verification failed",
        "--verify-log",
        str(verify_log),
        "--base-sha",
        base_sha,
        "--target-sha",
        "abc123",
        "--target-ref",
        "refs/heads/fix/example",
        cwd=ROOT,
    )

    request = json.loads((output_dir / "request.json").read_text())
    assert [operation["type"] for operation in request["operations"]] == [
        "comment_pr",
        "comment_issue",
        "edit_issue_labels",
    ]
    body = (output_dir / "pr-comment.md").read_text()
    assert "Auto-repair Failed Verification" in body
    assert "Verification tail" in body
