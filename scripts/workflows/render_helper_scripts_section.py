#!/usr/bin/env python3
"""Render a markdown section for helper scripts present in the reduced workspace."""

from __future__ import annotations

from pathlib import Path

HELPERS = [
    (
        "scripts/check-cop.py",
        "aggregate corpus regression check for one cop",
        "python3 scripts/check-cop.py Department/CopName --verbose --rerun --quick --clone",
    ),
    (
        "scripts/dispatch-cops.py",
        "dispatch helpers for changed cops, task generation, ranking, and prior attempts",
        "python3 scripts/dispatch-cops.py changed --base origin/main --head HEAD",
    ),
    (
        "scripts/investigate-cop.py",
        "inspect FP/FN examples from corpus oracle data",
        "python3 scripts/investigate-cop.py Department/CopName --context",
    ),
    (
        "scripts/verify-cop-locations.py",
        "verify exact known oracle FP/FN locations",
        "python3 scripts/verify-cop-locations.py Department/CopName",
    ),
    (
        "scripts/corpus-smoke-test.py",
        "smoke-test a few pinned repos",
        "python3 scripts/corpus-smoke-test.py --binary target/release/nitrocop",
    ),
]


def build_section(repo_root: Path) -> str:
    available: list[tuple[str, str, str | None]] = []
    for rel_path, description, example in HELPERS:
        if (repo_root / rel_path).is_file():
            available.append((rel_path, description, example))

    if not available:
        return ""

    lines = [
        "",
        "## Available Local Helper Scripts",
        "",
        "Only these helper scripts are present in this CI workspace. Prefer the stable top-level CLI paths shown below over ad hoc commands when they directly help with diagnosis or validation.",
        "",
    ]

    for rel_path, description, _example in available:
        lines.append(f"- `{rel_path}` — {description}")

    examples = [example for _rel_path, _description, example in available if example]
    if examples:
        lines.extend([
            "",
            "Typical usage when present:",
            "```bash",
            *examples,
            "```",
        ])

    lines.append("")
    return "\n".join(lines)


def main() -> int:
    section = build_section(Path.cwd())
    if section:
        print(section)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
