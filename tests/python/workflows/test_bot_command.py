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


def test_route_payload_maps_pr_mention_to_repair() -> None:
    routed = bot_command.route_payload(
        {
            "source_repo": "6/nitrocop",
            "trigger_kind": "mention",
            "subject_kind": "pull_request",
            "issue_number": 42,
            "pr_number": 42,
            "requested_by": "6",
            "requested_by_association": "OWNER",
            "request_url": "https://github.com/6/nitrocop/pull/42#issuecomment-99",
            "prompt_text": "Please retry with narrower changes.",
        },
        repo="6/nitrocop",
    )

    assert routed.action == "repair_pr"
    assert routed.trigger_kind == "mention"
    assert routed.prompt_text == "Please retry with narrower changes."
    assert routed.pr_number == 42
    assert routed.issue_number == 42


def test_route_payload_maps_issue_mention_to_fix() -> None:
    routed = bot_command.route_payload(
        {
            "source_repo": "6/nitrocop",
            "trigger_kind": "mention",
            "subject_kind": "issue",
            "issue_number": 17,
            "pr_number": None,
            "requested_by": "6",
            "requested_by_association": "OWNER",
            "request_url": "https://github.com/6/nitrocop/issues/17#issuecomment-99",
            "prompt_text": "please prioritize the corpus FN examples",
        },
        repo="6/nitrocop",
    )

    assert routed.action == "fix_issue"
    assert routed.pr_number is None
    assert routed.issue_number == 17
    assert routed.prompt_text == "please prioritize the corpus FN examples"


def test_route_payload_maps_issue_assignment_to_fix() -> None:
    routed = bot_command.route_payload(
        {
            "source_repo": "6/nitrocop",
            "trigger_kind": "assignment",
            "subject_kind": "issue",
            "issue_number": 17,
            "pr_number": None,
            "requested_by": "6",
            "requested_by_association": "OWNER",
            "request_url": "https://github.com/6/nitrocop/issues/17",
            "issue_title": "[cop] Style/FooBar",
            "issue_body": "Please focus on the edge cases.\n<!-- nitrocop-cop-tracker: cop=Style/FooBar -->",
        },
        repo="6/nitrocop",
    )

    assert routed.action == "fix_issue"
    assert routed.trigger_kind == "assignment"
    assert routed.trigger_summary == "issue assignment to @6"
    assert routed.prompt_text == (
        "## Tracker Issue Title\n"
        "[cop] Style/FooBar\n\n"
        "## Tracker Issue Body\n"
        "Please focus on the edge cases."
    )


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


def test_build_issue_assignment_prompt_strips_tracker_marker() -> None:
    prompt = bot_command.build_issue_assignment_prompt(
        "[cop] Style/Foo",
        "<!-- nitrocop-cop-tracker: cop=Style/Foo -->\nPlease use the issue text as the prompt.",
    )

    assert prompt == (
        "## Tracker Issue Title\n"
        "[cop] Style/Foo\n\n"
        "## Tracker Issue Body\n"
        "Please use the issue text as the prompt."
    )
