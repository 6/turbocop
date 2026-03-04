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


# README with comma-formatted repo count (regression test for "1,1000" bug)
SAMPLE_README_COMMA = SAMPLE_README.replace(
    "[**500 open-source repos**]",
    "[**1,000 open-source repos**]",
)


def make_corpus(tmp: Path, *, fp: int = 100000, fn: int = 100000,
                matches: int = 4900000) -> tuple[Path, Path, Path]:
    """Write minimal corpus-results.json, manifest.jsonl, and README.md."""
    total = matches + fn
    corpus = {
        "schema": 1,
        "summary": {
            "total_repos": 500,
            "repos_perfect": 100,
            "repos_error": 0,
            "total_offenses_compared": total,
            "matches": matches,
            "fp": fp,
            "fn": fn,
            "overall_match_rate": matches / total if total > 0 else 0,
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
        assert "96.0% conformance" in updated
        assert "167k Ruby files" in updated
        assert "nitrocop extra (FP)" in updated
        assert "nitrocop missed (FN)" in updated
        assert "| RuboCop offenses |" in updated


def test_conformance_includes_fp():
    """Conformance rate should be matches/(matches+fp+fn), not matches/(matches+fn).

    With asymmetric FP/FN the two formulas diverge:
    - matches/(matches+fn) = 10M/(10M+500K) = 95.2%
    - matches/(matches+fp+fn) = 10M/(10M+120K+500K) = 94.1% (floored)
    """
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        input_path, manifest_path, readme_path = make_corpus(
            tmp, matches=10_000_000, fp=120_000, fn=500_000,
        )

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
        # matches/(matches+fp+fn) = 10M/10.62M = 94.1% (floored from 94.16%)
        assert "94.1% conformance" in updated, (
            f"Expected 94.1% (includes FP in denominator, floored), got: "
            + next(l for l in updated.splitlines() if "conformance" in l)
        )
        # NOT 95.2% (which would be matches/(matches+fn), ignoring FP)
        assert "95.2% conformance" not in updated


def test_comma_repo_count():
    """Repo count regex should handle comma-formatted numbers (e.g. '1,000')."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        input_path, manifest_path, readme_path = make_corpus(tmp)
        # Write README with comma-formatted repo count
        readme_path.write_text(SAMPLE_README_COMMA)

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
        # Should NOT produce "1,500" or "1,1000" mangled forms
        assert "1,500" not in updated
        assert "1,1000" not in updated
        assert "500 open-source repos" in updated


if __name__ == "__main__":
    test_dry_run()
    test_write()
    test_conformance_includes_fp()
    test_comma_repo_count()
    print("OK: all tests passed")
