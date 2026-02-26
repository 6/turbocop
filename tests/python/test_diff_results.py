#!/usr/bin/env python3
"""Smoke tests for diff_results.py to catch regressions like undefined variables."""

import json
import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).parents[2] / "bench" / "corpus" / "diff_results.py"


def write_fixture(tmp: Path):
    """Create minimal fixture data: 1 repo with 1 matching offense, 1 with errors."""
    nitrocop_dir = tmp / "nitrocop"
    rubocop_dir = tmp / "rubocop"
    nitrocop_dir.mkdir()
    rubocop_dir.mkdir()

    # Repo with matching offenses
    nitrocop_dir.joinpath("repo_a.json").write_text(json.dumps({
        "offenses": [
            {"path": "repos/repo_a/app.rb", "line": 1, "cop_name": "Style/FrozenStringLiteralComment"}
        ]
    }))
    rubocop_dir.joinpath("repo_a.json").write_text(json.dumps({
        "files": [
            {
                "path": "repos/repo_a/app.rb",
                "offenses": [
                    {"location": {"line": 1}, "cop_name": "Style/FrozenStringLiteralComment"}
                ]
            }
        ],
        "summary": {"target_file_count": 1, "inspected_file_count": 1}
    }))

    # Repo with only nitrocop results (rubocop missing → error repo)
    nitrocop_dir.joinpath("repo_b.json").write_text(json.dumps({
        "offenses": [
            {"path": "repos/repo_b/lib.rb", "line": 5, "cop_name": "Layout/TrailingWhitespace"}
        ]
    }))

    # Manifest
    manifest = tmp / "manifest.jsonl"
    manifest.write_text(
        json.dumps({"id": "repo_a"}) + "\n"
        + json.dumps({"id": "repo_b"}) + "\n"
    )

    return nitrocop_dir, rubocop_dir, manifest


def test_end_to_end():
    """Run diff_results.py with minimal fixtures and verify it exits 0."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        nc_dir, rc_dir, manifest = write_fixture(tmp)
        out_json = tmp / "out.json"
        out_md = tmp / "out.md"

        result = subprocess.run(
            [
                sys.executable, str(SCRIPT),
                "--nitrocop-dir", str(nc_dir),
                "--rubocop-dir", str(rc_dir),
                "--manifest", str(manifest),
                "--output-json", str(out_json),
                "--output-md", str(out_md),
            ],
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0, f"Script failed:\nstdout: {result.stdout}\nstderr: {result.stderr}"
        assert out_json.exists(), "JSON output not written"
        assert out_md.exists(), "Markdown output not written"

        data = json.loads(out_json.read_text())
        assert data["summary"]["total_repos"] == 2
        assert data["summary"]["repos_error"] == 1  # repo_b missing rubocop
        assert data["summary"]["matches"] == 1

        md = out_md.read_text()
        assert "## Summary" in md
        assert "## Per-Repo Results" in md


if __name__ == "__main__":
    test_end_to_end()
    print("OK: all tests passed")
