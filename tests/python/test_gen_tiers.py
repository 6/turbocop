#!/usr/bin/env python3
"""Smoke tests for gen_tiers.py."""

import json
import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).parents[2] / "bench" / "corpus" / "gen_tiers.py"


def test_dry_run():
    """Run gen_tiers.py --dry-run with minimal corpus results."""
    corpus = {
        "schema": 1,
        "by_cop": [
            {"cop": "Style/FrozenStringLiteralComment", "fp": 0, "fn": 2, "matches": 100},
            {"cop": "Layout/TrailingWhitespace", "fp": 3, "fn": 0, "matches": 50},
            {"cop": "Style/StringLiterals", "fp": 0, "fn": 0, "matches": 200},
        ],
    }

    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        input_path = tmp / "corpus-results.json"
        input_path.write_text(json.dumps(corpus))

        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--input", str(input_path), "--dry-run"],
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0, f"Script failed:\nstdout: {result.stdout}\nstderr: {result.stderr}"

        tiers = json.loads(result.stdout)
        assert tiers["default_tier"] == "preview"
        # 0-FP cops should be stable
        assert "Style/FrozenStringLiteralComment" in tiers["overrides"]
        assert tiers["overrides"]["Style/FrozenStringLiteralComment"] == "stable"
        assert "Style/StringLiterals" in tiers["overrides"]
        # Non-zero FP cop should NOT be in overrides (defaults to preview)
        assert "Layout/TrailingWhitespace" not in tiers["overrides"]


def test_write_output():
    """Run gen_tiers.py with --output and verify file is written."""
    corpus = {
        "schema": 1,
        "by_cop": [
            {"cop": "Style/FrozenStringLiteralComment", "fp": 0, "fn": 0, "matches": 10},
        ],
    }

    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        input_path = tmp / "corpus-results.json"
        input_path.write_text(json.dumps(corpus))
        output_path = tmp / "tiers.json"

        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--input", str(input_path), "--output", str(output_path)],
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0, f"Script failed:\nstdout: {result.stdout}\nstderr: {result.stderr}"
        assert output_path.exists(), "Output file not written"
        tiers = json.loads(output_path.read_text())
        assert tiers["schema"] == 1


if __name__ == "__main__":
    test_dry_run()
    test_write_output()
    print("OK: all tests passed")
