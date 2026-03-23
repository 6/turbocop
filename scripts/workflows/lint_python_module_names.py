#!/usr/bin/env python3
"""Enforce Python naming policy for public CLIs and internal modules."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SNAKE_CASE_RE = re.compile(r"^[a-z][a-z0-9_]*\.py$")
KEBAB_CASE_RE = re.compile(r"^[a-z][a-z0-9-]*\.py$")
PUBLIC_SCRIPT_ROOT = ROOT / "scripts"
INTERNAL_DIRS = [
    ROOT / "scripts" / "workflows",
    ROOT / "scripts" / "shared",
    ROOT / "tests" / "python",
    ROOT / "bench" / "corpus",
    ROOT / ".agents" / "skills",
    ROOT / ".claude" / "skills",
]


def find_public_cli_violations() -> list[Path]:
    violations: list[Path] = []
    if not PUBLIC_SCRIPT_ROOT.is_dir():
        return violations
    for path in sorted(PUBLIC_SCRIPT_ROOT.glob("*.py")):
        if not KEBAB_CASE_RE.match(path.name):
            violations.append(path)
    return violations


def find_internal_violations() -> list[Path]:
    violations: list[Path] = []
    for base in INTERNAL_DIRS:
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.py")):
            if path.name == "__init__.py":
                continue
            if base.name == "skills" and "scripts" not in path.parts:
                continue
            if not SNAKE_CASE_RE.match(path.name):
                violations.append(path)
    return violations


def main() -> int:
    public = find_public_cli_violations()
    internal = find_internal_violations()
    if not public and not internal:
        print("Python naming policy passed.")
        return 0

    if public:
        print("Found non-kebab-case public CLI scripts:", file=sys.stderr)
        for path in public:
            print(f"  {path.relative_to(ROOT)}", file=sys.stderr)
    if internal:
        print("Found non-snake_case internal Python modules:", file=sys.stderr)
        for path in internal:
            print(f"  {path.relative_to(ROOT)}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
