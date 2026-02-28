#!/usr/bin/env python3
"""Smoke tests for update_readme.py."""

import json
import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).parents[2] / "bench" / "corpus" / "update_readme.py"


SAMPLE_README = """\
# nitrocop

Features:
- **93.0% conformance** across a corpus of open-source repos
- Tested against [**500 open-source repos**](docs/corpus.md) (163k Ruby files)

## Conformance

We diff nitrocop against RuboCop on [**500 open-source repos**](docs/corpus.md) (163k Ruby files) with all cops enabled. Every offense is compared by file, line, and cop name.

|                        |    Count |  Rate |
|:-----------------------|--------: |------:|
| Agreed                 |     4.0M | 93.0% |
| nitrocop extra (FP)    |   200.0K |  3.5% |
| nitrocop missed (FN)   |   200.0K |  3.5% |

Per-repo results (top 15 by GitHub stars):

| Repo | .rb files | RuboCop offenses | nitrocop extra (FP) | nitrocop missed (FN) | Agreement |
|------|----------:|-----------------:|--------------------:|---------------------:|----------:|
| [rails](https://github.com/rails/rails) | 3,000 | 11,760 | 240 | 240 | 95.0% |

More text here.
"""


def make_corpus(tmp: Path) -> tuple[Path, Path, Path]:
    """Write minimal corpus-results.json, manifest.jsonl, and README.md."""
    corpus = {
        "schema": 1,
        "summary": {
            "total_repos": 500,
            "repos_perfect": 100,
            "repos_error": 0,
            "total_offenses_compared": 5100000,
            "matches": 4900000,
            "fp": 100000,
            "fn": 100000,
            "overall_match_rate": 0.9608,
            "total_files_inspected": 167000,
        },
        "by_repo": [
            {
                "repo": "rails__rails__abc123",
                "status": "ok",
                "match_rate": 0.96,
                "matches": 11520,
                "fp": 240,
                "fn": 240,
                "files_inspected": 3100,
            },
        ],
    }

    manifest_entry = {
        "id": "rails__rails__abc123",
        "repo_url": "https://github.com/rails/rails",
        "notes": "auto-discovered, 55000 stars",
    }

    input_path = tmp / "corpus-results.json"
    input_path.write_text(json.dumps(corpus))

    manifest_path = tmp / "manifest.jsonl"
    manifest_path.write_text(json.dumps(manifest_entry) + "\n")

    readme_path = tmp / "README.md"
    readme_path.write_text(SAMPLE_README)

    return input_path, manifest_path, readme_path


def test_dry_run():
    """Run update_readme.py --dry-run and verify it exits 0 without modifying README."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        input_path, manifest_path, readme_path = make_corpus(tmp)

        result = subprocess.run(
            [
                sys.executable, str(SCRIPT),
                "--input", str(input_path),
                "--manifest", str(manifest_path),
                "--readme", str(readme_path),
                "--dry-run",
            ],
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0, f"Script failed:\nstdout: {result.stdout}\nstderr: {result.stderr}"
        # README should be unchanged in dry-run mode
        assert readme_path.read_text() == SAMPLE_README


def test_write():
    """Run update_readme.py and verify it updates the conformance rate."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        input_path, manifest_path, readme_path = make_corpus(tmp)

        result = subprocess.run(
            [
                sys.executable, str(SCRIPT),
                "--input", str(input_path),
                "--manifest", str(manifest_path),
                "--readme", str(readme_path),
            ],
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0, f"Script failed:\nstdout: {result.stdout}\nstderr: {result.stderr}"

        updated = readme_path.read_text()
        assert "96.1% conformance" in updated
        assert "167k Ruby files" in updated
        assert "nitrocop extra (FP)" in updated
        assert "nitrocop missed (FN)" in updated
        assert "| RuboCop offenses |" in updated


if __name__ == "__main__":
    test_dry_run()
    test_write()
    print("OK: all tests passed")
