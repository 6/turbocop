#!/usr/bin/env python3
"""Thin wrapper to keep one source of truth for gem-progress logic.

The canonical implementation lives at:
  .claude/skills/fix-department/scripts/gem_progress.py
"""

from __future__ import annotations

import runpy
from pathlib import Path


def main() -> None:
    repo_root = Path(__file__).resolve().parents[4]
    canonical = repo_root / ".claude" / "skills" / "fix-department" / "scripts" / "gem_progress.py"
    runpy.run_path(str(canonical), run_name="__main__")


if __name__ == "__main__":
    main()
