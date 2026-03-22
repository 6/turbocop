#!/usr/bin/env python3
"""Tests for resolve_backend.py."""
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[3] / "scripts" / "workflows"))
import resolve_backend

SCRIPT = Path(__file__).parents[3] / "scripts" / "workflows" / "resolve_backend.py"


def test_all_backends_resolve():
    """Every registered backend should resolve without error."""
    for name in resolve_backend.BACKENDS:
        config = resolve_backend.resolve(name)
        assert config["cli"] in ("claude", "codex"), f"{name}: unexpected cli"
        assert config["log_format"] in ("claude", "codex"), f"{name}: unexpected log_format"
        assert config["setup_cmd"], f"{name}: missing setup_cmd"
        assert config["run_cmd"], f"{name}: missing run_cmd"
        assert config["log_pattern"], f"{name}: missing log_pattern"


def test_minimax_uses_claude():
    config = resolve_backend.resolve("minimax")
    assert config["cli"] == "claude"
    assert config["log_format"] == "claude"
    assert "ANTHROPIC_BASE_URL" in config["env"]
    assert "guard_backend_secrets.py" in config["setup_cmd"]
    assert "MINIMAX_API_KEY" in config["setup_cmd"]
    assert "claude.ai/install.sh" in config["setup_cmd"]


def test_claude_uses_claude():
    config = resolve_backend.resolve("claude")
    assert config["cli"] == "claude"
    assert config["log_format"] == "claude"
    assert "ANTHROPIC_BASE_URL" not in config["env"]
    assert "guard_backend_secrets.py" in config["setup_cmd"]
    assert "ANTHROPIC_API_KEY" in config["setup_cmd"]
    assert "claude.ai/install.sh" in config["setup_cmd"]


def test_codex_uses_codex():
    config = resolve_backend.resolve("codex")
    assert config["cli"] == "codex"
    assert config["log_format"] == "codex"
    assert "CODEX_AUTH_JSON" in config["secrets"]
    assert "guard_backend_secrets.py" in config["setup_cmd"]
    assert "validate_codex_auth.py" in config["setup_cmd"]
    assert "@openai/codex@latest" in config["setup_cmd"]
    assert "chmod 700 ~/.codex" in config["setup_cmd"]
    assert "chmod 600 ~/.codex/auth.json" in config["setup_cmd"]
    assert "--dangerously-bypass-approvals-and-sandbox" in config["run_cmd"]
    assert "--json" in config["run_cmd"]
    assert '"$AGENT_EVENTS_FILE"' in config["run_cmd"]
    assert '"$FINAL_TASK_FILE"' in config["run_cmd"]
    assert '"$AGENT_RESULT_FILE"' in config["run_cmd"]
    assert '"$CI_SCRIPTS_DIR/agent_logs.py" summarize' in config["run_cmd"]


def test_unknown_backend_exits():
    result = subprocess.run(
        [sys.executable, str(SCRIPT), "unknown"],
        capture_output=True, text=True,
    )
    assert result.returncode != 0
    assert "Unknown backend" in result.stderr


def test_cli_output_format():
    """CLI should output key=value pairs."""
    result = subprocess.run(
        [sys.executable, str(SCRIPT), "minimax"],
        capture_output=True, text=True,
    )
    assert result.returncode == 0
    lines = result.stdout.strip().split("\n")
    for line in lines:
        assert "=" in line, f"Line missing '=': {line}"
    keys = [line_value.split("=", 1)[0] for line_value in lines]
    assert "cli" in keys
    assert "setup_cmd" in keys
    assert "log_format" in keys
    assert "run_cmd" in keys


def test_no_args_exits():
    result = subprocess.run(
        [sys.executable, str(SCRIPT)],
        capture_output=True, text=True,
    )
    assert result.returncode != 0


if __name__ == "__main__":
    test_all_backends_resolve()
    test_minimax_uses_claude()
    test_claude_uses_claude()
    test_codex_uses_codex()
    test_unknown_backend_exits()
    test_cli_output_format()
    test_no_args_exits()
    print("All tests passed.")
