#!/usr/bin/env python3
"""Tests for the remote repo-write bridge helper."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parents[3]
SCRIPT = ROOT / "scripts" / "workflows" / "remote_repo_write_bridge.py"


def test_prepare_input_rewrites_and_copies_request_files(tmp_path: Path) -> None:
    body = tmp_path / "comment.md"
    patch = tmp_path / "repair.patch"
    request = tmp_path / "request.json"
    output_dir = tmp_path / "out"

    body.write_text("hello\n")
    patch.write_text("diff --git a/file b/file\n")
    request.write_text(
        json.dumps(
            {
                "match_mode": "current_head",
                "operations": [
                    {"type": "comment_pr", "pr_number": 1, "body_file": str(body)},
                    {"type": "push_patch", "patch_file": str(patch), "commit_message": "msg"},
                ],
            }
        )
    )

    subprocess.run(
        [sys.executable, str(SCRIPT), "prepare-input", "--request-file", str(request), "--output-dir", str(output_dir)],
        cwd=str(ROOT),
        text=True,
        capture_output=True,
        check=True,
    )

    rewritten = json.loads((output_dir / "request.json").read_text())
    assert rewritten["operations"][0]["body_file"].startswith("op1-body_file-")
    assert rewritten["operations"][1]["patch_file"].startswith("op2-patch_file-")
    assert (output_dir / rewritten["operations"][0]["body_file"]).read_text() == "hello\n"
    assert "diff --git" in (output_dir / rewritten["operations"][1]["patch_file"]).read_text()
