#!/usr/bin/env python3
"""Tests for cop_fix_lifecycle.py."""
import json
import os
import subprocess
import sys
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).parents[3] / "scripts" / "workflows"))
import cop_fix_lifecycle

SCRIPT = Path(__file__).parents[3] / "scripts" / "workflows" / "cop_fix_lifecycle.py"


# ── snake_case ──────────────────────────────────────────────────────────

def test_snake_case_simple():
    assert cop_fix_lifecycle.snake_case("NegatedWhile") == "negated_while"


def test_snake_case_consecutive_caps():
    assert cop_fix_lifecycle.snake_case("ABCSize") == "abc_size"


def test_snake_case_already_snake():
    assert cop_fix_lifecycle.snake_case("already_snake") == "already_snake"


def test_snake_case_single_word():
    assert cop_fix_lifecycle.snake_case("Foo") == "foo"


# ── cop_identifiers ────────────────────────────────────────────────────

def test_cop_identifiers_standard():
    ids = cop_fix_lifecycle.cop_identifiers("Style/NegatedWhile")
    assert ids["dept_dir"] == "style"
    assert ids["cop_snake"] == "negated_while"
    assert ids["branch_prefix"] == "fix/style-negated_while"
    assert ids["filter"] == "cop::style::negated_while"


def test_cop_identifiers_rspec():
    ids = cop_fix_lifecycle.cop_identifiers("RSpec/DescribedClass")
    assert ids["dept_dir"] == "rspec"
    assert ids["cop_snake"] == "described_class"
    assert ids["branch_prefix"] == "fix/rspec-described_class"


def test_cop_identifiers_factory_bot():
    ids = cop_fix_lifecycle.cop_identifiers("FactoryBot/CreateList")
    assert ids["dept_dir"] == "factory_bot"


def test_cop_identifiers_metrics():
    ids = cop_fix_lifecycle.cop_identifiers("Metrics/ABCSize")
    assert ids["dept_dir"] == "metrics"
    assert ids["cop_snake"] == "abc_size"


# ── cmd_init ────────────────────────────────────────────────────────────

def test_init_outputs(tmp_path):
    output_file = tmp_path / "output.txt"
    env = {**os.environ, "GITHUB_OUTPUT": str(output_file)}
    result = subprocess.run(
        [sys.executable, str(SCRIPT), "init",
         "--cop", "Style/NegatedWhile",
         "--mode", "fix",
         "--backend-input", "auto",
         "--strength-input", "auto",
         "--run-id", "12345"],
        capture_output=True, text=True, env=env,
    )
    assert result.returncode == 0
    outputs = output_file.read_text()
    assert "branch=fix/style-negated_while-12345" in outputs
    assert "filter=cop::style::negated_while" in outputs
    assert "branch_prefix=fix/style-negated_while" in outputs


def test_init_retry_mode(tmp_path):
    output_file = tmp_path / "output.txt"
    env = {**os.environ, "GITHUB_OUTPUT": str(output_file)}
    result = subprocess.run(
        [sys.executable, str(SCRIPT), "init",
         "--cop", "Metrics/AbcSize",
         "--mode", "retry",
         "--backend-input", "claude",
         "--strength-input", "hard",
         "--run-id", "99"],
        capture_output=True, text=True, env=env,
    )
    assert result.returncode == 0
    assert "Retry cop:" in result.stderr


# ── build-prompt ────────────────────────────────────────────────────────

def test_build_prompt_basic(tmp_path):
    task_file = tmp_path / "context" / "task.md"
    final_file = tmp_path / "context" / "final-task.md"
    prior_file = tmp_path / "context" / "prior-attempts.md"
    task_file.parent.mkdir(parents=True, exist_ok=True)
    task_file.write_text("Fix the cop.\n")

    env = {
        **os.environ,
        "TASK_FILE": str(task_file),
        "FINAL_TASK_FILE": str(final_file),
        "PRIOR_ATTEMPTS_FILE": str(prior_file),
        "GITHUB_OUTPUT": str(tmp_path / "output.txt"),
    }
    # Avoid actual gh CLI calls by using fix mode (no retry)
    result = subprocess.run(
        [sys.executable, str(SCRIPT), "build-prompt",
         "--cop", "Style/NegatedWhile",
         "--mode", "fix",
         "--extra-context", "",
         "--filter", "cop::style::negated_while"],
        capture_output=True, text=True, env=env,
    )
    assert result.returncode == 0
    content = final_file.read_text()
    assert "Before making changes, read `docs/agent-ci.md`." in content
    assert "Fix the cop." in content
    # Should NOT have retry note
    assert "CRITICAL" not in content


def test_build_prompt_with_extra_context(tmp_path):
    task_file = tmp_path / "context" / "task.md"
    final_file = tmp_path / "context" / "final-task.md"
    prior_file = tmp_path / "context" / "prior-attempts.md"
    task_file.parent.mkdir(parents=True, exist_ok=True)
    task_file.write_text("Fix the cop.\n")

    env = {
        **os.environ,
        "TASK_FILE": str(task_file),
        "FINAL_TASK_FILE": str(final_file),
        "PRIOR_ATTEMPTS_FILE": str(prior_file),
        "GITHUB_OUTPUT": str(tmp_path / "output.txt"),
    }
    result = subprocess.run(
        [sys.executable, str(SCRIPT), "build-prompt",
         "--cop", "Style/NegatedWhile",
         "--mode", "fix",
         "--extra-context", "Focus on the edge case with blocks.",
         "--filter", "cop::style::negated_while"],
        capture_output=True, text=True, env=env,
    )
    assert result.returncode == 0
    content = final_file.read_text()
    assert "## Additional Instructions" in content
    assert "Focus on the edge case with blocks." in content


# ── _build_claim_body ───────────────────────────────────────────────────

def test_build_claim_body_with_issue():
    body = cop_fix_lifecycle._build_claim_body(
        cop="Style/NegatedWhile",
        mode="fix",
        backend_label="claude / normal",
        model_label="Claude Sonnet",
        backend_reason="auto-selected",
        run_url="https://example.com/run/1",
        issue_number="42",
    )
    assert "Style/NegatedWhile" in body
    assert "Closes #42" in body
    assert "nitrocop-cop-issue" in body
    assert "claude / normal" in body


def test_build_claim_body_without_issue():
    body = cop_fix_lifecycle._build_claim_body(
        cop="Style/NegatedWhile",
        mode="retry",
        backend_label="codex / hard",
        model_label="GPT-5.4",
        backend_reason="manual override",
        run_url="https://example.com/run/2",
        issue_number="",
    )
    assert "Style/NegatedWhile" in body
    assert "Closes" not in body
    assert "codex / hard" in body


# ── _build_task_body ────────────────────────────────────────────────────

def test_build_task_body():
    body = cop_fix_lifecycle._build_task_body(
        cop="Metrics/AbcSize",
        mode="fix",
        backend_label="claude / hard",
        model_label="Claude Opus",
        run_url="https://example.com/run/3",
        issue_number="99",
        code_bugs="5",
        tokens="2000",
        task_text="## Task\n\nFix AbcSize.\n",
    )
    assert "Metrics/AbcSize" in body
    assert "Code bugs:** 5" in body
    assert "2000 tokens" in body
    assert "## Task" in body
    assert "Closes #99" in body


# ── _generate_summary ───────────────────────────────────────────────────

def test_generate_summary_with_result(tmp_path):
    # Set up env vars pointing to tmp files
    agent_result = tmp_path / "agent" / "agent-result.json"
    stat_file = tmp_path / "recovery" / "diff.stat"
    diff_file = tmp_path / "recovery" / "diff.diff"
    git_dir = tmp_path / "recovery" / "git-activity"
    task_file = tmp_path / "context" / "task.md"
    logfile_ptr = tmp_path / "recovery" / "logfile-path.txt"
    agent_log = tmp_path / "agent" / "agent.log"

    for d in [agent_result.parent, stat_file.parent, git_dir, task_file.parent]:
        d.mkdir(parents=True, exist_ok=True)

    agent_result.write_text(json.dumps({
        "total_cost_usd": 1.23,
        "num_turns": 15,
        "result": "Fixed the cop successfully.",
    }))
    stat_file.write_text(" 3 files changed, 20 insertions(+), 5 deletions(-)\n")
    diff_file.write_text("")
    task_file.write_text("## Task\n\nFix the cop.\n")
    logfile_ptr.write_text("")
    agent_log.write_text("")

    env_patch = {
        "AGENT_RESULT_FILE": str(agent_result),
        "AGENT_RECOVERY_STAT_FILE": str(stat_file),
        "AGENT_RECOVERY_DIFF_FILE": str(diff_file),
        "AGENT_GIT_ACTIVITY_DIR": str(git_dir),
        "TASK_FILE": str(task_file),
        "AGENT_LOGFILE_POINTER_FILE": str(logfile_ptr),
        "AGENT_LOG_FILE": str(agent_log),
    }
    with patch.dict(os.environ, env_patch):
        summary = cop_fix_lifecycle._generate_summary(
            cop="Style/Foo",
            backend="claude-normal",
            mode="fix",
            run_url="https://example.com/run/1",
            run_number="42",
            base_sha="abc123",
        )

    assert "Style/Foo" in summary
    assert "$1.23" in summary
    assert "15" in summary
    assert "Fixed the cop successfully." in summary
    assert "3 files changed" in summary


def test_generate_summary_no_result(tmp_path):
    agent_result = tmp_path / "agent" / "agent-result.json"
    stat_file = tmp_path / "recovery" / "diff.stat"
    diff_file = tmp_path / "recovery" / "diff.diff"
    git_dir = tmp_path / "recovery" / "git-activity"
    task_file = tmp_path / "context" / "task.md"
    logfile_ptr = tmp_path / "recovery" / "logfile-path.txt"
    agent_log = tmp_path / "agent" / "agent.log"

    for d in [agent_result.parent, stat_file.parent, git_dir, task_file.parent]:
        d.mkdir(parents=True, exist_ok=True)

    # Empty result file
    agent_result.write_text("")
    stat_file.write_text("")
    diff_file.write_text("")
    task_file.write_text("## Task\n")
    logfile_ptr.write_text("")
    agent_log.write_text("")

    env_patch = {
        "AGENT_RESULT_FILE": str(agent_result),
        "AGENT_RECOVERY_STAT_FILE": str(stat_file),
        "AGENT_RECOVERY_DIFF_FILE": str(diff_file),
        "AGENT_GIT_ACTIVITY_DIR": str(git_dir),
        "TASK_FILE": str(task_file),
        "AGENT_LOGFILE_POINTER_FILE": str(logfile_ptr),
        "AGENT_LOG_FILE": str(agent_log),
    }
    with patch.dict(os.environ, env_patch):
        summary = cop_fix_lifecycle._generate_summary(
            cop="Style/Foo",
            backend="claude-normal",
            mode="fix",
            run_url="https://example.com/run/1",
            run_number="42",
            base_sha="abc123",
        )

    assert "empty or missing" in summary
    assert "No file changes detected." in summary


# ── CLI error handling ──────────────────────────────────────────────────

def test_unknown_command():
    result = subprocess.run(
        [sys.executable, str(SCRIPT), "nonexistent"],
        capture_output=True, text=True,
    )
    assert result.returncode == 1


def test_no_args():
    result = subprocess.run(
        [sys.executable, str(SCRIPT)],
        capture_output=True, text=True,
    )
    assert result.returncode == 1


# ── Manual runner ───────────────────────────────────────────────────────

if __name__ == "__main__":
    import tempfile

    test_snake_case_simple()
    test_snake_case_consecutive_caps()
    test_snake_case_already_snake()
    test_snake_case_single_word()
    test_cop_identifiers_standard()
    test_cop_identifiers_rspec()
    test_cop_identifiers_factory_bot()
    test_cop_identifiers_metrics()
    test_build_claim_body_with_issue()
    test_build_claim_body_without_issue()
    test_build_task_body()
    test_unknown_command()
    test_no_args()

    with tempfile.TemporaryDirectory() as d:
        test_init_outputs(Path(d))
    with tempfile.TemporaryDirectory() as d:
        test_init_retry_mode(Path(d))
    with tempfile.TemporaryDirectory() as d:
        test_build_prompt_basic(Path(d))
    with tempfile.TemporaryDirectory() as d:
        test_build_prompt_with_extra_context(Path(d))
    with tempfile.TemporaryDirectory() as d:
        test_generate_summary_with_result(Path(d))
    with tempfile.TemporaryDirectory() as d:
        test_generate_summary_no_result(Path(d))
    print("All tests passed.")
