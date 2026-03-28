#!/usr/bin/env python3
"""Tests for the generic repo_task workflow helper."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

ROOT = Path(__file__).parents[3]
SCRIPT = ROOT / "scripts" / "workflows" / "repo_task.py"
sys.path.insert(0, str(SCRIPT.parent))

SPEC = importlib.util.spec_from_file_location("repo_task", SCRIPT)
assert SPEC and SPEC.loader
repo_task = importlib.util.module_from_spec(SPEC)
sys.modules["repo_task"] = repo_task
SPEC.loader.exec_module(repo_task)


def test_build_extra_context_for_assignment() -> None:
    routed = repo_task.bot_command.RoutedCommand(
        action="fix_issue",
        trigger_kind="assignment",
        subject_kind="issue",
        issue_number=17,
        pr_number=None,
        requested_by="6",
        requested_by_association="OWNER",
        request_url="https://github.com/6/nitrocop/issues/17",
        prompt_text="## Tracker Issue Title\n[cop] Style/Foo",
        trigger_summary="issue assignment to @6",
    )

    extra = repo_task._build_extra_context(routed)  # noqa: SLF001
    assert extra == (
        "Requested by @6 via issue assignment to @6.\n"
        "Trigger URL: https://github.com/6/nitrocop/issues/17\n\n"
        "## Tracker Issue Title\n[cop] Style/Foo"
    )


def test_build_extra_context_for_comment_without_prompt() -> None:
    routed = repo_task.bot_command.RoutedCommand(
        action="fix_issue",
        trigger_kind="mention",
        subject_kind="issue",
        issue_number=17,
        pr_number=None,
        requested_by="6",
        requested_by_association="OWNER",
        request_url="",
        prompt_text="",
        trigger_summary="@6 mention",
    )

    extra = repo_task._build_extra_context(routed)  # noqa: SLF001
    assert extra == "Requested by @6 via @6 mention."
