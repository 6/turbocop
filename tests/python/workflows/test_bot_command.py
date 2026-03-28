#!/usr/bin/env python3
"""Tests for the bot-command workflow helper."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

ROOT = Path(__file__).parents[3]
SCRIPT = ROOT / "scripts" / "workflows" / "bot_command.py"

SPEC = importlib.util.spec_from_file_location("bot_command", SCRIPT)
assert SPEC and SPEC.loader
bot_command = importlib.util.module_from_spec(SPEC)
sys.modules["bot_command"] = bot_command
SPEC.loader.exec_module(bot_command)


def test_parse_force_and_context_extracts_force_flag() -> None:
    force, context = bot_command.parse_force_and_context("--force retry after corpus check")
    assert force is True
    assert context == "retry after corpus check"


def test_route_payload_maps_repair_command() -> None:
    routed = bot_command.route_payload(
        {
            "source_repo": "6/nitrocop",
            "subject_kind": "pull_request",
            "issue_number": 42,
            "command": "/6bot repair",
            "command_args": "--force rerun after latest push",
            "pr_number": 42,
            "comment_id": 99,
            "requested_by": "6",
            "requested_by_association": "OWNER",
            "comment_url": "https://github.com/6/nitrocop/pull/42#issuecomment-99",
        },
        repo="6/nitrocop",
    )

    assert routed.action == "repair_pr"
    assert routed.force is True
    assert routed.extra_context == "rerun after latest push"
    assert routed.pr_number == 42
    assert routed.issue_number == 42


def test_route_payload_maps_fix_issue_command() -> None:
    routed = bot_command.route_payload(
        {
            "source_repo": "6/nitrocop",
            "subject_kind": "issue",
            "issue_number": 17,
            "command": "/6bot fix",
            "command_args": "please prioritize the corpus FN examples",
            "pr_number": None,
            "comment_id": 99,
            "requested_by": "6",
            "requested_by_association": "OWNER",
            "comment_url": "https://github.com/6/nitrocop/issues/17#issuecomment-99",
        },
        repo="6/nitrocop",
    )

    assert routed.action == "fix_issue"
    assert routed.pr_number is None
    assert routed.issue_number == 17
    assert routed.extra_context == "please prioritize the corpus FN examples"


def test_route_payload_rejects_fix_on_pr_comments() -> None:
    routed = bot_command.route_payload(
        {
            "source_repo": "6/nitrocop",
            "subject_kind": "pull_request",
            "issue_number": 42,
            "command": "/6bot fix",
            "command_args": "",
            "pr_number": 42,
            "comment_id": 99,
            "requested_by": "6",
            "requested_by_association": "OWNER",
            "comment_url": "https://github.com/6/nitrocop/pull/42#issuecomment-99",
        },
        repo="6/nitrocop",
    )

    assert routed.action == "comment_only"
    assert "cop tracker issue comments" in routed.reason


def test_choose_failed_checks_run_prefers_failed_current_head() -> None:
    run, reason = bot_command.choose_failed_checks_run(
        [
            {"id": 11, "head_sha": "deadbeef", "status": "completed", "conclusion": "success"},
            {"id": 12, "head_sha": "cafebabe", "status": "completed", "conclusion": "failure"},
        ],
        head_sha="cafebabe",
    )

    assert reason == ""
    assert run and run["id"] == 12


def test_choose_failed_checks_run_reports_non_failing_head() -> None:
    run, reason = bot_command.choose_failed_checks_run(
        [
            {"id": 11, "head_sha": "cafebabe", "status": "completed", "conclusion": "success"},
        ],
        head_sha="cafebabe",
    )

    assert run is None
    assert reason == "Checks is not currently failing for the current PR head"


def test_extract_cop_from_issue_prefers_tracker_marker() -> None:
    cop = bot_command.extract_cop_from_issue(
        {
            "title": "[cop] Wrong/Title",
            "body": "<!-- nitrocop-cop-tracker: cop=Style/Foo fp=1 fn=2 total=3 difficulty=simple -->",
        }
    )
    assert cop == "Style/Foo"
