#!/usr/bin/env python3
"""Tests for git_activity_snapshot.py."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).parents[3] / "scripts" / "workflows" / "git_activity_snapshot.py"


def git(repo: Path, *args: str, check: bool = True) -> str:
    result = subprocess.run(
        ["git", *args],
        cwd=str(repo),
        text=True,
        capture_output=True,
        check=check,
    )
    return result.stdout.strip()


def make_repo() -> Path:
    tmp = Path(tempfile.mkdtemp())
    git(tmp, "init")
    git(tmp, "config", "user.name", "Test Bot")
    git(tmp, "config", "user.email", "test@example.com")
    (tmp / "file.txt").write_text("one\n")
    git(tmp, "add", "file.txt")
    git(tmp, "commit", "-m", "init")
    return tmp


def run_capture(repo: Path, output: Path) -> None:
    subprocess.run(
        [sys.executable, str(SCRIPT), "capture", "--repo-root", str(repo), "--output", str(output)],
        cwd=str(repo),
        text=True,
        capture_output=True,
        check=True,
    )


def run_report(before: Path, after: Path, out_dir: Path) -> None:
    subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            "report",
            "--before",
            str(before),
            "--after",
            str(after),
            "--out-dir",
            str(out_dir),
        ],
        text=True,
        capture_output=True,
        check=True,
    )


def test_capture_records_main_worktree_and_refs():
    repo = make_repo()
    output = repo / "snapshot.json"
    run_capture(repo, output)

    snapshot = json.loads(output.read_text())
    assert snapshot["head"]
    assert snapshot["head_ref"]
    assert snapshot["refs"][snapshot["head_ref"]] == snapshot["head"]
    assert any(item["path"] == str(repo.resolve()) for item in snapshot["worktrees"])


def test_report_captures_extra_worktree_and_branch_artifacts():
    repo = make_repo()
    before = repo / "before.json"
    after = repo / "after.json"
    out_dir = repo / "artifacts"
    run_capture(repo, before)

    git(repo, "branch", "side")
    wt_dir = Path(tempfile.mkdtemp()) / "side-worktree"
    git(repo, "worktree", "add", str(wt_dir), "side")
    (wt_dir / "file.txt").write_text("one\ntwo\n")
    git(wt_dir, "add", "file.txt")
    git(wt_dir, "commit", "-m", "side change")

    run_capture(repo, after)
    run_report(before, after, out_dir)

    report = (out_dir / "report.md").read_text()
    assert "## Extra Local Refs" in report
    assert "refs/heads/side" in report
    assert "## Extra Worktrees" in report
    assert str(wt_dir) in report

    artifact_files = {path.name for path in out_dir.iterdir()}
    assert "report.json" in artifact_files
    assert "report.md" in artifact_files
    assert any(name.startswith("ref-side") for name in artifact_files)
    assert any(name.startswith("worktree_") for name in artifact_files)


if __name__ == "__main__":
    test_capture_records_main_worktree_and_refs()
    test_report_captures_extra_worktree_and_branch_artifacts()
    print("All tests passed.")
