#!/usr/bin/env python3
"""Lint cop files for patterns that should use shared modules instead.

Scans src/cop/ (excluding src/cop/shared/) for inline patterns that have
shared equivalents in src/cop/shared/. Exits non-zero if violations found.

Run: uv run python3 scripts/lint_shared_usage.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
COP_DIR = REPO_ROOT / "src" / "cop"
SHARED_DIR = COP_DIR / "shared"

# Each rule: (compiled regex, message, optional file-level skip regex)
# The regex is matched against each line of non-test Rust code.
RULES: list[tuple[re.Pattern[str], str]] = [
    # --- method_dispatch_predicates ---
    (
        re.compile(
            r"\.receiver\(\)\.is_none\(\)\s*&&\s*\w+\.name\(\)\.as_slice\(\)\s*=="
        ),
        "use method_dispatch_predicates::is_command() instead of "
        "inline receiver().is_none() && name() == check",
    ),
    (
        re.compile(
            r"\.name\(\)\.as_slice\(\)\s*==\s*b\"[^\"]+\"\s*&&\s*\w+\.receiver\(\)\.is_none\(\)"
        ),
        "use method_dispatch_predicates::is_command() instead of "
        "inline name() == && receiver().is_none() check",
    ),
    (
        re.compile(
            r"\.call_operator_loc\(\)\s*\.\s*is_some_and\(\|[^|]+\|\s*\w+\.as_slice\(\)\s*==\s*b\"&\.\""
        ),
        "use method_dispatch_predicates::is_safe_navigation() instead of "
        "inline call_operator_loc &. check",
    ),
    (
        re.compile(
            r"\.call_operator_loc\(\)\s*\.\s*is_some_and\(\|[^|]+\|\s*\w+\.as_slice\(\)\s*==\s*b\"\.\""
        ),
        "use method_dispatch_predicates::is_dot_call() instead of "
        "inline call_operator_loc . check",
    ),
    (
        re.compile(
            r"\.call_operator_loc\(\)\s*\.\s*is_some_and\(\|[^|]+\|\s*\w+\.as_slice\(\)\s*==\s*b\"::\""
        ),
        "use method_dispatch_predicates::is_double_colon_call() instead of "
        "inline call_operator_loc :: check",
    ),
    # --- predicate_operator_predicates ---
    (
        re.compile(r"\.operator_loc\(\)\.as_slice\(\)\s*==\s*b\"&&\""),
        "use predicate_operator_predicates::is_logical_and() instead of "
        "inline operator_loc == b\"&&\" check",
    ),
    (
        re.compile(r"\.operator_loc\(\)\.as_slice\(\)\s*==\s*b\"\|\|\""),
        "use predicate_operator_predicates::is_logical_or() instead of "
        "inline operator_loc == b\"||\" check",
    ),
    (
        re.compile(r"\.operator_loc\(\)\.as_slice\(\)\s*==\s*b\"and\""),
        "use predicate_operator_predicates::is_semantic_and() instead of "
        "inline operator_loc == b\"and\" check",
    ),
    (
        re.compile(r"\.operator_loc\(\)\.as_slice\(\)\s*==\s*b\"or\""),
        "use predicate_operator_predicates::is_semantic_or() instead of "
        "inline operator_loc == b\"or\" check",
    ),
]


def is_in_test_block(lines: list[str], line_idx: int) -> bool:
    """Heuristic: check if a line is inside a #[cfg(test)] mod tests block."""
    for i in range(line_idx, -1, -1):
        stripped = lines[i].strip()
        if stripped == "#[cfg(test)]":
            return True
        # If we hit the top of a non-test module, stop looking.
        if stripped.startswith("pub fn ") or stripped.startswith("fn "):
            # Could be inside a test fn or a real fn — keep scanning.
            pass
        if stripped == "mod tests {":
            return True
    return False


def lint_file(path: Path) -> list[str]:
    """Return list of violation messages for a single file."""
    violations: list[str] = []
    try:
        text = path.read_text(encoding="utf-8")
    except Exception:
        return violations

    lines = text.splitlines()
    in_test_section = False

    for i, line in enumerate(lines):
        stripped = line.strip()

        # Track test sections — skip them.
        if stripped == "#[cfg(test)]":
            in_test_section = True
            continue
        if in_test_section:
            # Stay in test section until end of file (tests are always at bottom).
            continue

        # Skip comments.
        if stripped.startswith("//"):
            continue

        for pattern, message in RULES:
            if pattern.search(line):
                rel_path = path.relative_to(REPO_ROOT)
                violations.append(f"{rel_path}:{i + 1}: {message}")

    return violations


def main() -> int:
    all_violations: list[str] = []

    for rs_file in sorted(COP_DIR.rglob("*.rs")):
        # Skip shared module files — they define the patterns, not consume them.
        if SHARED_DIR in rs_file.parents or rs_file.parent == SHARED_DIR:
            continue

        all_violations.extend(lint_file(rs_file))

    if all_violations:
        print(f"Found {len(all_violations)} shared module usage violation(s):\n")
        for v in all_violations:
            print(f"  {v}")
        print(
            "\nThese patterns have shared equivalents in src/cop/shared/. "
            "Use the shared functions instead."
        )
        return 1

    print("No shared module usage violations found.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
