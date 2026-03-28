#!/usr/bin/env python3
"""Tests for cop_fix_publish.py."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

SCRIPT = Path(__file__).parents[3] / "scripts" / "workflows" / "cop_fix_publish.py"


def _run(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(SCRIPT), *args],
        text=True,
        capture_output=True,
        check=True,
    )


def test_cleanup_request_with_pr_and_issue(tmp_path: Path) -> None:
    output_dir = tmp_path / "cleanup"
    _run(
        "cleanup-request",
        "--output-dir",
        str(output_dir),
        "--cop",
        "Style/NegatedWhile",
        "--pr",
        "https://github.com/6/nitrocop/pull/123",
        "--issue-number",
        "77",
        "--backend-label",
        "claude / normal",
        "--model-label",
        "Claude Sonnet",
        "--mode",
        "fix",
        "--run-url",
        "https://github.com/6/nitrocop/actions/runs/1",
    )

    request = json.loads((output_dir / "request.json").read_text())
    assert request["match_mode"] == "contained"
    assert request["operations"][0] == {
        "type": "close_pr",
        "pr": "https://github.com/6/nitrocop/pull/123",
        "comment": "Agent failed. See run: https://github.com/6/nitrocop/actions/runs/1",
        "delete_branch": True,
    }
    assert request["operations"][1]["type"] == "comment_issue"
    assert request["operations"][1]["issue_number"] == 77
    assert request["operations"][1]["body_file"] == "issue-comment.md"
    assert request["operations"][2] == {
        "type": "edit_issue_labels",
        "issue_number": 77,
        "remove_labels": ["state:pr-open", "state:dispatched"],
        "add_labels": ["state:backlog"],
        "ignore_failure": True,
    }
    body = (output_dir / "issue-comment.md").read_text()
    assert "Agent fix failed before producing a usable PR" in body
    assert "`Style/NegatedWhile`" in body


def test_claim_request_builds_remote_claim_flow(tmp_path: Path) -> None:
    output_dir = tmp_path / "claim"
    task_file = tmp_path / "task.md"
    task_file.write_text("## Task\n\nFix the cop.\n")

    _run(
        "claim-request",
        "--output-dir",
        str(output_dir),
        "--cop",
        "Style/NegatedWhile",
        "--mode",
        "fix",
        "--branch",
        "fix/style-negated_while-123",
        "--backend",
        "claude-normal",
        "--backend-label",
        "claude / normal",
        "--model-label",
        "Claude Sonnet",
        "--backend-reason",
        "auto-selected",
        "--run-url",
        "https://github.com/6/nitrocop/actions/runs/1",
        "--issue-number",
        "77",
        "--code-bugs",
        "5",
        "--tokens",
        "1200",
        "--task-file",
        str(task_file),
    )

    request = json.loads((output_dir / "request.json").read_text())
    assert request["match_mode"] == "current_head"
    operations = request["operations"]
    assert operations[0]["type"] == "ensure_labels"
    assert operations[1] == {
        "type": "create_branch",
        "branch": "fix/style-negated_while-123",
        "commit_message": "[bot] Fix Style/NegatedWhile: in progress",
        "promote_message": "[bot] Fix Style/NegatedWhile: in progress",
    }
    assert operations[2]["type"] == "create_pr"
    assert operations[2]["head"] == "fix/style-negated_while-123"
    assert operations[2]["draft"] is True
    assert operations[2]["labels"] == ["type:cop-fix", "model:claude-normal"]
    assert operations[3] == {
        "type": "edit_issue_labels",
        "issue_number": 77,
        "remove_labels": ["state:backlog", "state:dispatched", "state:blocked"],
        "add_labels": ["state:pr-open"],
        "ignore_failure": True,
    }
    assert operations[4] == {
        "type": "edit_pr",
        "pr": "{{PR_URL}}",
        "body_file": "task-body.md",
    }
    claim_body = (output_dir / "claim-body.md").read_text()
    task_body = (output_dir / "task-body.md").read_text()
    assert "Refs #77" in claim_body
    assert "<!-- nitrocop-cop-issue: number=77 cop=Style/NegatedWhile -->" in claim_body
    assert "## Task" in task_body
    assert "Code bugs:** 5" in task_body


def _init_repo(tmp_path: Path) -> tuple[Path, str]:
    repo = tmp_path / "repo"
    repo.mkdir()
    subprocess.run(["git", "init", "-b", "main"], cwd=repo, check=True, capture_output=True, text=True)
    subprocess.run(["git", "config", "user.name", "Test User"], cwd=repo, check=True, capture_output=True, text=True)
    subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=repo, check=True, capture_output=True, text=True)
    (repo / "README.md").write_text("base\n")
    subprocess.run(["git", "add", "README.md"], cwd=repo, check=True, capture_output=True, text=True)
    subprocess.run(["git", "commit", "-m", "base"], cwd=repo, check=True, capture_output=True, text=True)
    base_sha = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    return repo, base_sha


def test_cleanup_request_without_pr_only_comments_issue(tmp_path: Path) -> None:
    output_dir = tmp_path / "cleanup"
    _run(
        "cleanup-request",
        "--output-dir",
        str(output_dir),
        "--cop",
        "Style/NegatedWhile",
        "--issue-number",
        "77",
        "--backend-label",
        "auto",
        "--mode",
        "fix",
        "--run-url",
        "https://github.com/6/nitrocop/actions/runs/1",
    )

    request = json.loads((output_dir / "request.json").read_text())
    assert request["match_mode"] == "contained"
    assert request["operations"] == [
        {
            "type": "comment_issue",
            "issue_number": 77,
            "body_file": "issue-comment.md",
        }
    ]
    body = (output_dir / "issue-comment.md").read_text()
    assert "before it could create a draft PR" in body


def test_skip_fixed_request_comments_issue(tmp_path: Path) -> None:
    output_dir = tmp_path / "skip-fixed"
    _run(
        "skip-fixed-request",
        "--output-dir",
        str(output_dir),
        "--cop",
        "Style/NegatedWhile",
        "--issue-number",
        "77",
        "--backend-input",
        "auto",
        "--mode",
        "fix",
        "--run-url",
        "https://github.com/6/nitrocop/actions/runs/1",
    )

    request = json.loads((output_dir / "request.json").read_text())
    assert request == {
        "match_mode": "contained",
        "operations": [
            {
                "type": "comment_issue",
                "issue_number": 77,
                "body_file": "issue-comment.md",
            }
        ],
    }
    body = (output_dir / "issue-comment.md").read_text()
    assert "no reproducible code bugs" in body
    assert "`Style/NegatedWhile`" in body


def test_finalize_request_docs_only_pushes_patch_and_marks_issue_blocked(tmp_path: Path) -> None:
    repo, base_sha = _init_repo(tmp_path)
    output_dir = tmp_path / "finalize-docs"
    pr_body = tmp_path / "pr-body.md"
    pr_body.write_text("PR body\n")
    (repo / "README.md").write_text("base\ndocs only\n")
    subprocess.run(["git", "add", "README.md"], cwd=repo, check=True, capture_output=True, text=True)
    subprocess.run(["git", "commit", "-m", "docs"], cwd=repo, check=True, capture_output=True, text=True)

    _run(
        "finalize-request",
        "--output-dir",
        str(output_dir),
        "--repo-root",
        str(repo),
        "--result",
        "docs_only",
        "--cop",
        "Style/NegatedWhile",
        "--backend",
        "claude-normal",
        "--backend-label",
        "claude / normal",
        "--model-label",
        "Claude Sonnet",
        "--mode",
        "fix",
        "--issue-number",
        "77",
        "--run-url",
        "https://github.com/6/nitrocop/actions/runs/1",
        "--base-sha",
        base_sha,
        "--pr-url",
        "https://github.com/6/nitrocop/pull/123",
        "--pr-number",
        "123",
        "--pr-body-file",
        str(pr_body),
    )

    request = json.loads((output_dir / "request.json").read_text())
    assert request["match_mode"] == "current_head"
    assert [operation["type"] for operation in request["operations"]] == [
        "push_patch",
        "edit_pr",
        "ready_pr",
        "merge_pr",
        "comment_issue",
        "edit_issue_labels",
    ]
    assert (output_dir / "cop-fix.patch").read_text()
    assert request["operations"][2] == {
        "type": "ready_pr",
        "pr": "https://github.com/6/nitrocop/pull/123",
    }
    assert request["operations"][3] == {
        "type": "merge_pr",
        "pr": "https://github.com/6/nitrocop/pull/123",
        "auto": True,
        "squash": True,
        "delete_branch": True,
    }
    assert request["operations"][5] == {
        "type": "edit_issue_labels",
        "issue_number": 77,
        "remove_labels": ["state:pr-open", "state:dispatched", "state:backlog"],
        "add_labels": ["state:blocked"],
        "ignore_failure": True,
    }


def test_finalize_request_no_changes_closes_pr_and_resets_issue(tmp_path: Path) -> None:
    repo, base_sha = _init_repo(tmp_path)
    output_dir = tmp_path / "finalize-no-changes"
    pr_body = tmp_path / "pr-body.md"
    pr_body.write_text("PR body\n")

    _run(
        "finalize-request",
        "--output-dir",
        str(output_dir),
        "--repo-root",
        str(repo),
        "--result",
        "no_changes",
        "--cop",
        "Style/NegatedWhile",
        "--backend",
        "claude-normal",
        "--backend-label",
        "claude / normal",
        "--model-label",
        "Claude Sonnet",
        "--mode",
        "fix",
        "--issue-number",
        "77",
        "--run-url",
        "https://github.com/6/nitrocop/actions/runs/1",
        "--base-sha",
        base_sha,
        "--pr-url",
        "https://github.com/6/nitrocop/pull/123",
        "--pr-number",
        "123",
        "--pr-body-file",
        str(pr_body),
    )

    request = json.loads((output_dir / "request.json").read_text())
    assert request["operations"][0] == {
        "type": "close_pr",
        "pr": "https://github.com/6/nitrocop/pull/123",
        "comment": "Agent produced no changes.",
        "delete_branch": True,
    }
    assert request["operations"][1]["type"] == "comment_issue"
    assert request["operations"][2] == {
        "type": "edit_issue_labels",
        "issue_number": 77,
        "remove_labels": ["state:pr-open", "state:dispatched"],
        "add_labels": ["state:backlog"],
        "ignore_failure": True,
    }


def test_finalize_request_rejected_comments_pr_and_issue(tmp_path: Path) -> None:
    repo, base_sha = _init_repo(tmp_path)
    output_dir = tmp_path / "finalize-rejected"
    pr_body = tmp_path / "pr-body.md"
    pr_body.write_text("PR body\n")
    scope_report = tmp_path / "scope-report.txt"
    scope_report.write_text("edited files outside allowed scope\n")

    _run(
        "finalize-request",
        "--output-dir",
        str(output_dir),
        "--repo-root",
        str(repo),
        "--result",
        "rejected",
        "--cop",
        "Style/NegatedWhile",
        "--backend",
        "claude-normal",
        "--backend-label",
        "claude / normal",
        "--model-label",
        "Claude Sonnet",
        "--mode",
        "fix",
        "--issue-number",
        "77",
        "--run-url",
        "https://github.com/6/nitrocop/actions/runs/1",
        "--base-sha",
        base_sha,
        "--pr-url",
        "https://github.com/6/nitrocop/pull/123",
        "--pr-number",
        "123",
        "--pr-body-file",
        str(pr_body),
        "--scope-report-file",
        str(scope_report),
    )

    request = json.loads((output_dir / "request.json").read_text())
    assert [operation["type"] for operation in request["operations"]] == [
        "comment_pr",
        "close_pr",
        "comment_issue",
        "edit_issue_labels",
    ]
    rejected_body = (output_dir / "rejected.md").read_text()
    assert "Agent Fix Rejected" in rejected_body
    assert "outside allowed scope" in rejected_body


def test_reset_issue_request_edits_labels(tmp_path: Path) -> None:
    output_dir = tmp_path / "reset"
    _run(
        "reset-issue-request",
        "--output-dir",
        str(output_dir),
        "--issue-number",
        "88",
    )

    request = json.loads((output_dir / "request.json").read_text())
    assert request == {
        "match_mode": "contained",
        "operations": [
            {
                "type": "edit_issue_labels",
                "issue_number": 88,
                "remove_labels": ["state:pr-open", "state:dispatched"],
                "add_labels": ["state:backlog"],
                "ignore_failure": True,
            }
        ],
    }
