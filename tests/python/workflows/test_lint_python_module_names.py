#!/usr/bin/env python3
"""Tests for the Python module naming policy."""

import subprocess
import sys
from pathlib import Path

SCRIPT = Path(__file__).parents[3] / "scripts" / "workflows" / "lint_python_module_names.py"


def test_importable_modules_use_snake_case():
    result = subprocess.run(
        [sys.executable, str(SCRIPT)],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr
    assert "naming policy" in result.stdout


if __name__ == "__main__":
    test_importable_modules_use_snake_case()
    print("All tests passed.")
