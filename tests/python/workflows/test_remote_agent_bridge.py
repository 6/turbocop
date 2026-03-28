#!/usr/bin/env python3
"""Tests for the remote agent bridge helper."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parents[3]
SCRIPT = ROOT / "scripts" / "workflows" / "remote_agent_bridge.py"


def test_prepare_input_copies_prompt_files(tmp_path: Path) -> None:
    final_task = tmp_path / "final-task.md"
    task = tmp_path / "task.md"
    output_dir = tmp_path / "out"

    final_task.write_text("# final\n")
    task.write_text("# task\n")

    env = os.environ.copy()
    env["FINAL_TASK_FILE"] = str(final_task)
    env["TASK_FILE"] = str(task)

    subprocess.run(
        [sys.executable, str(SCRIPT), "prepare-input", "--output-dir", str(output_dir)],
        cwd=str(ROOT),
        env=env,
        text=True,
        capture_output=True,
        check=True,
    )

    assert (output_dir / "final-task.md").read_text() == "# final\n"
    assert (output_dir / "task.md").read_text() == "# task\n"
