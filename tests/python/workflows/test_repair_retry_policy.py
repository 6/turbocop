#!/usr/bin/env python3
"""Tests for repair_retry_policy.py."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[3] / "scripts" / "workflows"))
import repair_retry_policy


def test_parse_marker_fields():
    body = (
        "hello\n"
        "<!-- nitrocop-auto-repair: phase=started head_sha=abc backend=codex-normal checks_run_id=1 -->\n"
        "<!-- nitrocop-auto-repair: phase=pushed head_sha=def backend=codex-hard repair_commit=123 -->\n"
    )
    markers = repair_retry_policy.parse_marker_fields(body)
    assert markers == [
        {
            "phase": "started",
            "head_sha": "abc",
            "backend": "codex-normal",
            "checks_run_id": "1",
        },
        {
            "phase": "pushed",
            "head_sha": "def",
            "backend": "codex-hard",
            "repair_commit": "123",
        },
    ]


def test_parse_linked_issue():
    body = (
        "Closes #130\n\n"
        "<!-- nitrocop-cop-issue: number=130 cop=Style/NegatedWhile -->\n"
    )
    issue_number, cop = repair_retry_policy.parse_linked_issue(body)
    assert issue_number == 130
    assert cop == "Style/NegatedWhile"


def test_inspect_attempts_counts_pushes_and_repairs():
    comments = [
        {
            "body": "<!-- nitrocop-auto-repair: phase=started head_sha=head-a backend=codex-normal checks_run_id=11 -->"
        },
        {
            "body": "<!-- nitrocop-auto-repair: phase=started head_sha=head-b backend=claude-oauth-hard checks_run_id=12 -->"
        },
        {
            "body": "<!-- nitrocop-auto-repair: phase=pushed head_sha=head-b backend=claude-oauth-hard repair_commit=123 -->"
        },
    ]
    result = repair_retry_policy.inspect_attempts(comments, "head-b")
    assert result["prior_pushes"] == 1
    assert result["prior_pr_repair_attempts"] == 2
    assert result["prior_attempted_current_head"] is True


def test_gate_pr_accepts_trusted_bot_pr():
    pr = {
        "state": "OPEN",
        "baseRefName": "main",
        "isCrossRepository": False,
        "headRepository": {"nameWithOwner": "6/nitrocop"},
        "author": {"login": "6[bot]"},
        "headRefName": "fix/style-negated-while-123",
        "labels": [{"name": "type:cop-fix"}],
        "headRefOid": "abc",
    }
    should_run, reason = repair_retry_policy.gate_pr(
        pr,
        "6/nitrocop",
        "abc",
        require_trusted_bot=True,
    )
    assert should_run is True
    assert reason == ""


def test_gate_pr_accepts_manual_dispatch_for_same_repo_human_pr():
    pr = {
        "state": "OPEN",
        "baseRefName": "main",
        "isCrossRepository": False,
        "headRepository": {"nameWithOwner": "6/nitrocop"},
        "author": {"login": "6"},
        "headRefName": "feature/manual-rerun",
        "labels": [{"name": "type:cop-fix"}],
        "headRefOid": "abc",
    }
    should_run, reason = repair_retry_policy.gate_pr(
        pr,
        "6/nitrocop",
        "abc",
        require_trusted_bot=False,
    )
    assert should_run is True


def test_gate_pr_rejects_human_author_for_automatic_repair():
    pr = {
        "state": "OPEN",
        "baseRefName": "main",
        "isCrossRepository": False,
        "headRepository": {"nameWithOwner": "6/nitrocop"},
        "author": {"login": "6"},
        "headRefName": "fix/style-negated-while-123",
        "labels": [{"name": "type:cop-fix"}],
        "headRefOid": "abc",
    }
    should_run, reason = repair_retry_policy.gate_pr(
        pr,
        "6/nitrocop",
        "abc",
        require_trusted_bot=True,
    )
    assert should_run is False
    assert reason == "PR author 6 is not trusted for automatic repair"


def test_gate_pr_rejects_non_fix_branch_for_automatic_repair():
    pr = {
        "state": "OPEN",
        "baseRefName": "main",
        "isCrossRepository": False,
        "headRepository": {"nameWithOwner": "6/nitrocop"},
        "author": {"login": "6[bot]"},
        "headRefName": "chore/release-notes",
        "labels": [{"name": "type:cop-fix"}],
        "headRefOid": "abc",
    }
    should_run, reason = repair_retry_policy.gate_pr(
        pr,
        "6/nitrocop",
        "abc",
        require_trusted_bot=True,
    )
    assert should_run is False
    assert reason == "PR branch chore/release-notes is not a trusted fix/* branch"


def test_gate_pr_rejects_closed_pr():
    pr = {
        "state": "CLOSED",
        "baseRefName": "main",
        "isCrossRepository": False,
        "headRepository": {"nameWithOwner": "6/nitrocop"},
        "author": {"login": "6[bot]"},
        "headRefName": "fix/style-negated-while-123",
        "labels": [{"name": "type:cop-fix"}],
        "headRefOid": "abc",
    }
    should_run, reason = repair_retry_policy.gate_pr(
        pr,
        "6/nitrocop",
        "abc",
        require_trusted_bot=True,
    )
    assert should_run is False
    assert reason == "PR is not open"


def test_gate_pr_rejects_head_moved_after_failed_checks():
    pr = {
        "state": "OPEN",
        "baseRefName": "main",
        "isCrossRepository": False,
        "headRepository": {"nameWithOwner": "6/nitrocop"},
        "author": {"login": "6[bot]"},
        "headRefName": "fix/style-negated-while-123",
        "labels": [{"name": "type:cop-fix"}],
        "headRefOid": "def",
    }
    should_run, reason = repair_retry_policy.gate_pr(
        pr,
        "6/nitrocop",
        "abc",
        require_trusted_bot=True,
    )
    assert should_run is False
    assert reason == "PR head moved after the failed Checks run"


def test_policy_blocks_same_head_repeat():
    should_run, reason, needs_human = repair_retry_policy.apply_policy(
        route="easy",
        force=False,
        prior_attempted_current_head=True,
        prior_pushes=0,
        prior_pr_repair_attempts=0,
    )
    assert should_run is False
    assert "already had an automatic repair attempt" in reason
    assert needs_human is False


def test_policy_blocks_after_two_pushes():
    should_run, reason, needs_human = repair_retry_policy.apply_policy(
        route="easy",
        force=False,
        prior_attempted_current_head=False,
        prior_pushes=2,
        prior_pr_repair_attempts=0,
    )
    assert should_run is False
    assert "2 automatic repair pushes" in reason
    assert needs_human is True


def test_policy_blocks_after_two_repair_attempts():
    should_run, reason, needs_human = repair_retry_policy.apply_policy(
        route="hard",
        force=False,
        prior_attempted_current_head=False,
        prior_pushes=1,
        prior_pr_repair_attempts=2,
    )
    assert should_run is False
    assert "2 automatic repair attempts" in reason
    assert needs_human is True


def test_policy_force_bypasses_caps():
    should_run, reason, needs_human = repair_retry_policy.apply_policy(
        route="hard",
        force=True,
        prior_attempted_current_head=True,
        prior_pushes=99,
        prior_pr_repair_attempts=99,
    )
    assert should_run is True
    assert reason == ""
    assert needs_human is False


def test_skip_comment_posts_pr_and_issue(monkeypatch, tmp_path):
    """cmd_skip_comment posts to PR and linked issue, then sets state:blocked."""
    calls: list[list[str]] = []

    def fake_run(cmd, **kwargs):
        calls.append(cmd)

        class Result:
            returncode = 0
        return Result()

    monkeypatch.setattr("subprocess.run", fake_run)

    args = argparse.Namespace(
        repo="6/nitrocop",
        pr_number="42",
        linked_issue_number="100",
        heading="Auto-repair Skipped",
        reason="route is skip",
        checks_run_id="999",
        checks_url="https://example.com/runs/999",
        backend_label="codex / hard",
        route="skip",
        run_id="888",
        run_url="https://example.com/runs/888",
        needs_human=False,
        issue_only_if_needs_human=False,
    )
    rc = repair_retry_policy.cmd_skip_comment(args)
    assert rc == 0

    # Should have: pr comment, issue comment, issue edit
    assert len(calls) == 3
    assert calls[0][0:3] == ["gh", "pr", "comment"]
    assert "42" in calls[0]
    assert calls[1][0:3] == ["gh", "issue", "comment"]
    assert "100" in calls[1]
    assert calls[2][0:3] == ["gh", "issue", "edit"]
    assert "state:blocked" in calls[2]


def test_skip_comment_skips_issue_when_needs_human_false(monkeypatch):
    """When issue_only_if_needs_human is set and needs_human is False, skip issue comment."""
    calls: list[list[str]] = []

    def fake_run(cmd, **kwargs):
        calls.append(cmd)

        class Result:
            returncode = 0
        return Result()

    monkeypatch.setattr("subprocess.run", fake_run)

    args = argparse.Namespace(
        repo="6/nitrocop",
        pr_number="42",
        linked_issue_number="100",
        heading="Automatic PR repair stopped",
        reason="too many attempts",
        checks_run_id="999",
        checks_url="https://example.com/runs/999",
        backend_label="n/a",
        route="",
        run_id="888",
        run_url="https://example.com/runs/888",
        needs_human=False,
        issue_only_if_needs_human=True,
    )
    rc = repair_retry_policy.cmd_skip_comment(args)
    assert rc == 0

    # Only PR comment, no issue comment/edit
    assert len(calls) == 1
    assert calls[0][0:3] == ["gh", "pr", "comment"]


def test_skip_comment_posts_issue_when_needs_human(monkeypatch):
    """When needs_human is True and issue_only_if_needs_human is set, post issue comment."""
    calls: list[list[str]] = []

    def fake_run(cmd, **kwargs):
        calls.append(cmd)

        class Result:
            returncode = 0
        return Result()

    monkeypatch.setattr("subprocess.run", fake_run)

    args = argparse.Namespace(
        repo="6/nitrocop",
        pr_number="42",
        linked_issue_number="100",
        heading="Automatic PR repair stopped",
        reason="too many attempts",
        checks_run_id="999",
        checks_url="https://example.com/runs/999",
        backend_label="codex / hard",
        route="",
        run_id="888",
        run_url="https://example.com/runs/888",
        needs_human=True,
        issue_only_if_needs_human=True,
    )
    rc = repair_retry_policy.cmd_skip_comment(args)
    assert rc == 0

    # PR comment + issue comment + issue edit
    assert len(calls) == 3


if __name__ == "__main__":
    test_parse_marker_fields()
    test_parse_linked_issue()
    test_inspect_attempts_counts_pushes_and_repairs()
    test_gate_pr_accepts_trusted_bot_pr()
    test_gate_pr_rejects_closed_pr()
    test_gate_pr_rejects_head_moved_after_failed_checks()
    test_policy_blocks_same_head_repeat()
    test_policy_blocks_after_two_pushes()
    test_policy_blocks_after_two_repair_attempts()
    test_policy_force_bypasses_caps()
    print("All tests passed.")
