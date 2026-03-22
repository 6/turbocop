#!/usr/bin/env python3
"""Tests for agent_artifacts.py."""

from __future__ import annotations

import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).parents[3] / "scripts" / "workflows" / "agent_artifacts.py"


def render_manifest(workflow: str) -> list[str]:
    tmpdir = Path(tempfile.mkdtemp())
    runtime_root = tmpdir / workflow
    output = tmpdir / "manifest.txt"
    subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            workflow,
            "--runtime-root",
            str(runtime_root),
            "--output",
            str(output),
        ],
        text=True,
        capture_output=True,
        check=True,
    )
    return output.read_text().splitlines()


def test_agent_cop_fix_manifest_contains_common_and_summary_paths():
    lines = render_manifest("agent-cop-fix")
    assert any(line.endswith("/agent-cop-fix/agent/agent.log") for line in lines)
    assert any(line.endswith("/agent-cop-fix/recovery/git-activity/**/*") for line in lines)
    assert any(line.endswith("/agent-cop-fix/context/task.md") for line in lines)
    assert any(line.endswith("/agent-cop-fix/context/summary.md") for line in lines)
    assert not any(line.endswith("/repair/summary.md") for line in lines)
    assert lines[-1].endswith("manifest.txt")


def test_agent_pr_repair_manifest_contains_repair_specific_paths():
    lines = render_manifest("agent-pr-repair")
    assert any(line.endswith("/agent-pr-repair/agent/agent.log") for line in lines)
    assert any(line.endswith("/agent-pr-repair/context/pr.diff") for line in lines)
    assert any(line.endswith("/agent-pr-repair/repair/summary.md") for line in lines)
    assert not any(line.endswith("/context/summary.md") for line in lines)
    assert lines[-1].endswith("manifest.txt")


if __name__ == "__main__":
    test_agent_cop_fix_manifest_contains_common_and_summary_paths()
    test_agent_pr_repair_manifest_contains_repair_specific_paths()
    print("All tests passed.")
