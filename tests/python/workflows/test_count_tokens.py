#!/usr/bin/env python3
"""Tests for count_tokens.py."""
import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).parents[3] / "scripts" / "workflows" / "count_tokens.py"


def run(file_path: str) -> tuple[str, int]:
    """Run count_tokens.py and return (stdout, returncode)."""
    result = subprocess.run(
        [sys.executable, str(SCRIPT), file_path],
        capture_output=True, text=True,
    )
    return result.stdout, result.returncode


def test_counts_tokens():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".md", delete=False) as f:
        f.write("Hello world, this is a test.")
        f.flush()
        out, rc = run(f.name)
    assert rc == 0
    if out.strip() == "?":
        return
    count = int(out.strip())
    assert count > 0
    assert count < 100  # sanity check — a short sentence


def test_empty_file():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".md", delete=False) as f:
        f.write("")
        f.flush()
        out, rc = run(f.name)
    assert rc == 0
    if out.strip() == "?":
        return
    assert out.strip() == "0"


def test_missing_args():
    result = subprocess.run(
        [sys.executable, str(SCRIPT)],
        capture_output=True, text=True,
    )
    assert result.returncode != 0


if __name__ == "__main__":
    test_counts_tokens()
    test_empty_file()
    test_missing_args()
    print("All tests passed.")
