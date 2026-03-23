#!/usr/bin/env python3
"""Tests for render_helper_scripts_section.py."""

from __future__ import annotations

import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[3] / "scripts" / "workflows"))
import render_helper_scripts_section as renderer


def test_empty_when_no_helper_scripts_present():
    with tempfile.TemporaryDirectory() as tmpdir:
        section = renderer.build_section(Path(tmpdir))
        assert section == ""


def test_renders_only_present_helper_scripts():
    with tempfile.TemporaryDirectory() as tmpdir:
        root = Path(tmpdir)
        (root / "scripts").mkdir()
        (root / "scripts" / "check-cop.py").write_text("print('ok')\n")
        (root / "scripts" / "dispatch-cops.py").write_text("print('ok')\n")

        section = renderer.build_section(root)
        assert "## Available Local Helper Scripts" in section
        assert "`scripts/check-cop.py`" in section
        assert "`scripts/dispatch-cops.py`" in section
        assert "scripts/corpus-smoke-test.py" not in section
        assert "python3 scripts/check-cop.py Department/CopName --verbose --rerun --quick --clone" in section
        assert "python3 scripts/dispatch-cops.py changed --base origin/main --head HEAD" in section


if __name__ == "__main__":
    test_empty_when_no_helper_scripts_present()
    test_renders_only_present_helper_scripts()
    print("All tests passed.")
