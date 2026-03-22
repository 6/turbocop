#!/usr/bin/env python3
"""Tests for prepare_pr_repair.py."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[2] / "scripts" / "ci"))
import prepare_pr_repair


def make_job(name: str, failed_steps: list[str], conclusion: str = "failure") -> dict:
    steps = [{"name": step, "conclusion": "failure"} for step in failed_steps]
    return {"name": name, "conclusion": conclusion, "steps": steps, "databaseId": 1}


def test_easy_linux_failure_routes_to_minimax():
    run = {
        "jobs": [
            make_job("build-and-test (ubuntu-24.04)", ["Clippy", "Test"]),
        ],
    }
    result = prepare_pr_repair.classify_run(run)
    assert result["route"] == "easy"
    assert result["backend"] == "minimax"
    assert "cargo clippy --profile ci -- -D warnings" in result["verification_commands"]
    assert "cargo test" in result["verification_commands"]


def test_hard_cop_check_routes_to_codex():
    run = {
        "jobs": [
            make_job("cop-check", ["Check cops against corpus baseline"]),
        ],
    }
    result = prepare_pr_repair.classify_run(run)
    assert result["route"] == "hard"
    assert result["backend"] == "codex"
    assert any("scripts/check-cop.py" in command for command in result["verification_commands"])


def test_mixed_failures_escalate_to_hard():
    run = {
        "jobs": [
            make_job("build-and-test (ubuntu-24.04)", ["Clippy"]),
            make_job("corpus-smoke-test", ["Run smoke test"]),
        ],
    }
    result = prepare_pr_repair.classify_run(run)
    assert result["route"] == "hard"
    assert result["backend"] == "codex"
    assert any("cargo clippy" in command for command in result["verification_commands"])
    assert any("corpus_smoke_test.py" in command for command in result["verification_commands"])


def test_macos_only_failure_is_skipped():
    run = {
        "jobs": [
            make_job("build-and-test (macos-latest)", ["Build"]),
        ],
    }
    result = prepare_pr_repair.classify_run(run)
    assert result["route"] == "skip"
    assert result["backend"] == ""


def test_prompt_includes_route_and_failed_packet():
    run = {"number": 57, "workflowName": "Checks", "jobs": []}
    classification = {
        "route": "hard",
        "backend": "codex",
        "reason": "cop-check: Check cops against corpus baseline",
        "verification_commands": ["cargo build --release", "python3 scripts/check-cop.py Foo/Bar"],
        "jobs": [
            {
                "name": "cop-check",
                "repair_route": "hard",
                "failed_step_names": ["Check cops against corpus baseline"],
                "failed_log": "Traceback\nFileNotFoundError",
            }
        ],
    }
    prompt = prepare_pr_repair.build_prompt(
        run=run,
        classification=classification,
        pr_meta={"number": "130", "title": "Fix Style/MixinUsage", "headRefName": "fix/style-mixin_usage"},
        diff_stat=" 1 file changed",
        diff_text="diff --git a/a b/a\n+change\n",
        extra_context="Keep the patch narrow.",
    )
    assert "PR #130" in prompt
    assert "Selected backend: `codex`" in prompt
    assert "Check cops against corpus baseline" in prompt
    assert "Keep the patch narrow." in prompt


if __name__ == "__main__":
    test_easy_linux_failure_routes_to_minimax()
    test_hard_cop_check_routes_to_codex()
    test_mixed_failures_escalate_to_hard()
    test_macos_only_failure_is_skipped()
    test_prompt_includes_route_and_failed_packet()
    print("All tests passed.")
