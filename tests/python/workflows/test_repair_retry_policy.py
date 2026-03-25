#!/usr/bin/env python3
"""Tests for repair_retry_policy.py."""

from __future__ import annotations

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


def test_inspect_attempts_counts_pushes_and_codex():
    comments = [
        {
            "body": "<!-- nitrocop-auto-repair: phase=started head_sha=head-a backend=codex-normal checks_run_id=11 -->"
        },
        {
            "body": "<!-- nitrocop-auto-repair: phase=started head_sha=head-b backend=codex-hard checks_run_id=12 -->"
        },
        {
            "body": "<!-- nitrocop-auto-repair: phase=pushed head_sha=head-b backend=codex-hard repair_commit=123 -->"
        },
    ]
    result = repair_retry_policy.inspect_attempts(comments, "head-b")
    assert result["prior_pushes"] == 1
    assert result["prior_codex_attempts"] == 2
    assert result["prior_attempted_current_head"] is True


def test_gate_pr_accepts_trusted_bot_pr():
    pr = {
        "state": "OPEN",
        "baseRefName": "main",
        "isCrossRepository": False,
        "headRepository": {"nameWithOwner": "6/nitrocop"},
        "author": {"login": "6[bot]"},
        "labels": [{"name": "type:cop-fix"}],
        "headRefOid": "abc",
    }
    should_run, reason = repair_retry_policy.gate_pr(pr, "6/nitrocop", "abc")
    assert should_run is True
    assert reason == ""


def test_gate_pr_accepts_human_author_with_label():
    pr = {
        "state": "OPEN",
        "baseRefName": "main",
        "isCrossRepository": False,
        "headRepository": {"nameWithOwner": "6/nitrocop"},
        "author": {"login": "6"},
        "labels": [{"name": "type:cop-fix"}],
        "headRefOid": "abc",
    }
    should_run, reason = repair_retry_policy.gate_pr(pr, "6/nitrocop", "abc")
    assert should_run is True


def test_gate_pr_rejects_closed_pr():
    pr = {
        "state": "CLOSED",
        "baseRefName": "main",
        "isCrossRepository": False,
        "headRepository": {"nameWithOwner": "6/nitrocop"},
        "author": {"login": "6[bot]"},
        "labels": [{"name": "type:cop-fix"}],
        "headRefOid": "abc",
    }
    should_run, reason = repair_retry_policy.gate_pr(pr, "6/nitrocop", "abc")
    assert should_run is False
    assert reason == "PR is not open"


def test_gate_pr_rejects_head_moved_after_failed_checks():
    pr = {
        "state": "OPEN",
        "baseRefName": "main",
        "isCrossRepository": False,
        "headRepository": {"nameWithOwner": "6/nitrocop"},
        "author": {"login": "6[bot]"},
        "labels": [{"name": "type:cop-fix"}],
        "headRefOid": "def",
    }
    should_run, reason = repair_retry_policy.gate_pr(pr, "6/nitrocop", "abc")
    assert should_run is False
    assert reason == "PR head moved after the failed Checks run"


def test_policy_blocks_same_head_repeat():
    should_run, reason, needs_human = repair_retry_policy.apply_policy(
        route="easy",
        backend="codex-hard",
        force=False,
        prior_attempted_current_head=True,
        prior_pushes=0,
        prior_codex_attempts=0,
    )
    assert should_run is False
    assert "already had an automatic repair attempt" in reason
    assert needs_human is False


def test_policy_blocks_after_two_pushes():
    should_run, reason, needs_human = repair_retry_policy.apply_policy(
        route="easy",
        backend="codex-hard",
        force=False,
        prior_attempted_current_head=False,
        prior_pushes=2,
        prior_codex_attempts=0,
    )
    assert should_run is False
    assert "2 automatic repair pushes" in reason
    assert needs_human is True


def test_policy_blocks_second_codex_attempt():
    should_run, reason, needs_human = repair_retry_policy.apply_policy(
        route="hard",
        backend="codex-hard",
        force=False,
        prior_attempted_current_head=False,
        prior_pushes=1,
        prior_codex_attempts=1,
    )
    assert should_run is False
    assert "Codex automatic repair attempt" in reason
    assert needs_human is True


def test_policy_force_bypasses_caps():
    should_run, reason, needs_human = repair_retry_policy.apply_policy(
        route="hard",
        backend="codex-hard",
        force=True,
        prior_attempted_current_head=True,
        prior_pushes=99,
        prior_codex_attempts=99,
    )
    assert should_run is True
    assert reason == ""
    assert needs_human is False


if __name__ == "__main__":
    test_parse_marker_fields()
    test_parse_linked_issue()
    test_inspect_attempts_counts_pushes_and_codex()
    test_gate_pr_accepts_trusted_bot_pr()
    test_gate_pr_rejects_closed_pr()
    test_gate_pr_rejects_head_moved_after_failed_checks()
    test_policy_blocks_same_head_repeat()
    test_policy_blocks_after_two_pushes()
    test_policy_blocks_second_codex_attempt()
    test_policy_force_bypasses_caps()
    print("All tests passed.")
