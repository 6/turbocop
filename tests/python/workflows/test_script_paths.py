#!/usr/bin/env python3
"""Path consistency checks for the canonical script layout."""

from __future__ import annotations

import subprocess
from pathlib import Path

ROOT = Path(__file__).parents[3]


def tracked_files() -> list[Path]:
    result = subprocess.run(
        ["git", "ls-files"],
        cwd=str(ROOT),
        text=True,
        capture_output=True,
        check=True,
    )
    files = []
    for line in result.stdout.splitlines():
        if not line or line.startswith(".claude/worktrees/"):
            continue
        path = ROOT / line
        if path.exists() and path.is_file():
            files.append(path)
    return files


def banned_tokens() -> list[str]:
    return [
        "/".join(("scripts", "agent")) + "/",
        "/".join(("scripts", "ci")) + "/",
        "/".join(("scripts", "corpus")) + "/",
        "scripts/" + "corpus_download.py",
        "scripts/" + "stress-report.py",
        "scripts/workflows/" + "lint_python_module_names.py",
        "scripts/workflows/" + "extract_agent_log.py",
        "scripts/workflows/" + "summarize_agent_result.py",
        "scripts/workflows/" + "watch_agent_progress.py",
        "scripts/" + "gen-stress-configs.py",
        "scripts/" + "land-branch-commits.sh",
        "scripts/workflows/" + "check_python_module_names.py",
    ]


def test_removed_paths_no_longer_exist():
    removed = [
        ROOT / "scripts" / "agent",
        ROOT / "scripts" / "ci",
        ROOT / "scripts" / "corpus",
        ROOT / "scripts" / "corpus_download.py",
        ROOT / "scripts" / "stress-report.py",
        ROOT / "scripts" / "gen-stress-configs.py",
        ROOT / "scripts" / "land-branch-commits.sh",
        ROOT / "scripts" / "check_python_module_names.py",
        ROOT / "scripts" / "workflows" / "lint_python_module_names.py",
        ROOT / "scripts" / "workflows" / "extract_agent_log.py",
        ROOT / "scripts" / "workflows" / "summarize_agent_result.py",
        ROOT / "scripts" / "workflows" / "watch_agent_progress.py",
    ]
    for path in removed:
        assert not path.exists(), f"obsolete path still exists: {path.relative_to(ROOT)}"


def test_tracked_files_use_only_canonical_paths():
    offenders: list[tuple[str, str]] = []
    for path in tracked_files():
        if path == Path(__file__):
            continue
        text = path.read_text(errors="ignore")
        for token in banned_tokens():
            if token in text:
                offenders.append((str(path.relative_to(ROOT)), token))

    assert offenders == [], "\n".join(f"{path}: {token}" for path, token in offenders)


if __name__ == "__main__":
    test_removed_paths_no_longer_exist()
    test_tracked_files_use_only_canonical_paths()
    print("All tests passed.")
