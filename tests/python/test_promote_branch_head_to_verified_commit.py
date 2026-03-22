#!/usr/bin/env python3
"""Tests for promote_branch_head_to_verified_commit.py."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).parents[2] / "scripts" / "ci"))
import promote_branch_head_to_verified_commit as promote


def test_promote_branch_head_rewrites_ref_to_signed_commit():
    calls = []

    def fake_run_gh(args):
        calls.append(args)
        path = args[0]
        if path.endswith("/git/ref/heads/test-branch"):
            return json.dumps({"object": {"sha": "unsigned123"}})
        if path.endswith("/git/commits/unsigned123"):
            return json.dumps({
                "tree": {"sha": "tree456"},
                "parents": [{"sha": "parent789"}],
            })
        if path.endswith("/git/commits"):
            return json.dumps({"sha": "signed999"})
        if path.endswith("/git/refs/heads/test-branch"):
            return ""
        raise AssertionError(f"Unexpected gh api call: {args}")

    with patch.object(promote, "run_gh", side_effect=fake_run_gh):
        result = promote.promote("owner/repo", "test-branch", "Test message")

    assert result == {
        "unsigned_sha": "unsigned123",
        "signed_sha": "signed999",
        "tree_sha": "tree456",
        "parent_sha": "parent789",
    }
    assert calls[0] == ["repos/owner/repo/git/ref/heads/test-branch"]
    assert calls[1] == ["repos/owner/repo/git/commits/unsigned123"]
    assert calls[2][:5] == [
        "repos/owner/repo/git/commits",
        "-f",
        "message=Test message",
        "-f",
        "tree=tree456",
    ]
    assert "parents[]=parent789" in calls[2]
    assert calls[3] == [
        "repos/owner/repo/git/refs/heads/test-branch",
        "-X",
        "PATCH",
        "-f",
        "sha=signed999",
        "-F",
        "force=true",
    ]


if __name__ == "__main__":
    test_promote_branch_head_rewrites_ref_to_signed_commit()
    print("All tests passed.")
