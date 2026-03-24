#!/usr/bin/env python3
"""Smoke tests for helper-script entrypoints without workflow PYTHONPATH shims."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parents[3]


def run_help(path: Path) -> None:
    env = os.environ.copy()
    env.pop("PYTHONPATH", None)
    subprocess.run(
        [sys.executable, str(path), "--help"],
        cwd=str(ROOT),
        env=env,
        text=True,
        capture_output=True,
        check=True,
    )


def test_public_and_workflow_scripts_run_without_pythonpath():
    run_help(ROOT / "scripts" / "check_cop.py")
    run_help(ROOT / "scripts" / "dispatch_cops.py")
    run_help(ROOT / "scripts" / "workflows" / "prepare_pr_repair.py")
    run_help(ROOT / "scripts" / "workflows" / "agent_logs.py")


if __name__ == "__main__":
    test_public_and_workflow_scripts_run_without_pythonpath()
    print("All tests passed.")
