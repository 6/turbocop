#!/usr/bin/env python3
"""Tests for agent_runtime.py."""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[3] / "scripts" / "workflows"))
import agent_runtime

SCRIPT = Path(__file__).parents[3] / "scripts" / "workflows" / "agent_runtime.py"
AGENT_COP_FIX = Path(__file__).parents[3] / ".github" / "workflows" / "agent-cop-fix.yml"
AGENT_PR_REPAIR = Path(__file__).parents[3] / ".github" / "workflows" / "agent-pr-repair.yml"


def test_build_paths_for_agent_cop_fix():
    runner_temp = Path("/tmp/runner")
    paths = agent_runtime.build_paths("agent-cop-fix", runner_temp)
    assert paths["AGENT_RUNTIME_ROOT"] == "/tmp/runner/agent-cop-fix"
    assert paths["CI_SCRIPTS_DIR"].endswith("/agent-cop-fix/ci-scripts")
    assert paths["SUMMARY_FILE"].endswith("/agent-cop-fix/context/summary.md")
    assert paths["AGENT_PATCH_FILE"].endswith("/agent-cop-fix/recovery/agent.patch")
    assert "REPAIR_SUMMARY_FILE" not in paths


def test_build_paths_for_agent_pr_repair():
    runner_temp = Path("/tmp/runner")
    paths = agent_runtime.build_paths("agent-pr-repair", runner_temp)
    assert paths["AGENT_RUNTIME_ROOT"] == "/tmp/runner/agent-pr-repair"
    assert paths["PR_DIFF_FILE"].endswith("/agent-pr-repair/context/pr.diff")
    assert paths["REPAIR_SUMMARY_FILE"].endswith("/agent-pr-repair/repair/summary.md")
    assert paths["REPAIR_CHANGED_COPS_FILE"].endswith("/agent-pr-repair/repair/changed-cops.txt")
    assert "SUMMARY_FILE" not in paths


def test_current_paths_uses_centralized_runtime_defaults():
    original_root = os.environ.get("AGENT_RUNTIME_ROOT")
    try:
        os.environ.pop("AGENT_RUNTIME_ROOT", None)
        paths = agent_runtime.current_paths("agent-pr-repair")
    finally:
        if original_root is None:
            os.environ.pop("AGENT_RUNTIME_ROOT", None)
        else:
            os.environ["AGENT_RUNTIME_ROOT"] = original_root

    assert paths["REPAIR_CORPUS_STANDARD_FILE"].endswith("/agent-pr-repair/repair/corpus-standard.json")
    assert paths["REPAIR_CORPUS_EXTENDED_FILE"].endswith("/agent-pr-repair/repair/corpus-extended.json")


def test_cli_emits_env_assignments_and_creates_directories():
    runner_temp = Path(tempfile.mkdtemp())
    result = subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            "agent-pr-repair",
            "--runner-temp",
            str(runner_temp),
        ],
        capture_output=True,
        text=True,
        check=True,
    )
    lines = dict(line.split("=", 1) for line in result.stdout.strip().splitlines())
    assert Path(lines["AGENT_RUNTIME_ROOT"]).is_dir()
    assert Path(lines["CI_SCRIPTS_DIR"]).is_dir()
    assert Path(lines["AGENT_REPAIR_DIR"]).is_dir()


def test_agent_workflows_do_not_hardcode_tmp_paths():
    assert "/tmp/" not in AGENT_COP_FIX.read_text()
    assert "/tmp/" not in AGENT_PR_REPAIR.read_text()


if __name__ == "__main__":
    test_build_paths_for_agent_cop_fix()
    test_build_paths_for_agent_pr_repair()
    test_current_paths_uses_centralized_runtime_defaults()
    test_cli_emits_env_assignments_and_creates_directories()
    test_agent_workflows_do_not_hardcode_tmp_paths()
    print("All tests passed.")
