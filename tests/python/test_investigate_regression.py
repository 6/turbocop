#!/usr/bin/env python3
"""Tests for investigate_regression.py."""

import importlib.util
from pathlib import Path

SCRIPT = Path(__file__).parents[2] / "scripts" / "investigate_regression.py"
SPEC = importlib.util.spec_from_file_location("investigate_regression", SCRIPT)
assert SPEC and SPEC.loader
mod = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(mod)


def test_compute_regressions_detects_fp_and_fn_increases():
    before = {
        "by_cop": [
            {"cop": "Style/Foo", "fp": 1, "fn": 2, "matches": 10},
            {"cop": "Layout/Bar", "fp": 0, "fn": 0, "matches": 10},
        ]
    }
    after = {
        "by_cop": [
            {"cop": "Style/Foo", "fp": 3, "fn": 2, "matches": 10},
            {"cop": "Layout/Bar", "fp": 0, "fn": 1, "matches": 10},
        ]
    }
    regressions = mod.compute_regressions(before, after)
    assert [entry["cop"] for entry in regressions] == ["Style/Foo", "Layout/Bar"]
    assert regressions[0]["delta_fp"] == 2
    assert regressions[1]["delta_fn"] == 1


def test_issue_difficulty_prefers_labels():
    issue = {"labels": [{"name": "difficulty:medium"}], "body": ""}
    assert mod.issue_difficulty(issue) == "medium"


def test_extract_cop_from_pr_uses_marker_then_title():
    pr = {
        "title": "[bot] Fix Style/Fallback",
        "body": "<!-- nitrocop-cop-issue: number=123 cop=Style/Explicit -->",
    }
    assert mod.extract_cop_from_pr(pr) == "Style/Explicit"
    assert mod.extract_cop_from_pr({"title": "[bot] Fix Style/Fallback", "body": ""}) == "Style/Fallback"


def test_recommended_action_prefers_strong_revert_candidate():
    regression = {
        "difficulty": "simple",
        "pr_candidates": [{"number": 1}],
    }
    assert mod.recommended_action(regression) == "strong_revert_candidate"


def test_recommended_action_dispatches_simple_without_pr_candidate():
    regression = {
        "difficulty": "simple",
        "pr_candidates": [],
    }
    assert mod.recommended_action(regression) == "dispatch_repair"


def test_render_report_mentions_issue_and_action():
    report = mod.render_report(
        {"id": 1, "html_url": "https://example.com/1", "head_sha": "abc"},
        {"id": 2, "html_url": "https://example.com/2", "head_sha": "def"},
        [
            {
                "cop": "Style/Foo",
                "before_fp": 1,
                "before_fn": 0,
                "after_fp": 2,
                "after_fn": 1,
                "delta_fp": 1,
                "delta_fn": 1,
                "difficulty": "medium",
                "issue_ref": "#12",
                "action": "manual_investigation",
                "pr_candidates": [],
                "commit_candidates": [],
            }
        ],
    )
    assert "Style/Foo" in report
    assert "#12" in report
    assert "manual_investigation" in report


if __name__ == "__main__":
    test_compute_regressions_detects_fp_and_fn_increases()
    test_issue_difficulty_prefers_labels()
    test_extract_cop_from_pr_uses_marker_then_title()
    test_recommended_action_prefers_strong_revert_candidate()
    test_recommended_action_dispatches_simple_without_pr_candidate()
    test_render_report_mentions_issue_and_action()
    print("All tests passed.")
