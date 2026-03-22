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
            },
            {
                "path": "repos/repo_a/extra.rb",
                "offenses": []
            }
        ],
        "summary": {"target_file_count": 2, "inspected_file_count": 2}
    }))

    # Repo with a true FP on an inspected file
    nitrocop_dir.joinpath("repo_a.json").write_text(json.dumps({
        "offenses": [
            {"path": "repos/repo_a/app.rb", "line": 1, "cop_name": "Style/FrozenStringLiteralComment"},
            {"path": "repos/repo_a/extra.rb", "line": 5, "cop_name": "Layout/TrailingWhitespace"},
        ]
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

    cop_list = tmp / "cops.txt"
    cop_list.write_text(
        "Style/FrozenStringLiteralComment\n"
        "Layout/TrailingWhitespace\n"
        "Lint/UnusedMethodArgument\n"
    )

    return nitrocop_dir, rubocop_dir, manifest, cop_list


def test_end_to_end():
    """Run diff_results.py with minimal fixtures and verify it exits 0."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        nc_dir, rc_dir, manifest, cop_list = write_fixture(tmp)
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
                "--cop-list", str(cop_list),
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
        assert data["summary"]["registered_cops"] == 3
        assert data["summary"]["perfect_cops"] == 1
        assert data["summary"]["diverging_cops"] == 1
        assert data["summary"]["inactive_cops"] == 1

        by_cop = {entry["cop"]: entry for entry in data["by_cop"]}
        assert by_cop["Style/FrozenStringLiteralComment"]["perfect_match"] is True
        assert by_cop["Layout/TrailingWhitespace"]["diverging"] is True
        assert by_cop["Lint/UnusedMethodArgument"]["exercised"] is False
        assert data["cop_activity_repos"]["Style/FrozenStringLiteralComment"] == ["repo_a"]
        assert "Layout/TrailingWhitespace" not in data["cop_activity_repos"]

        by_dept = {entry["department"]: entry for entry in data["by_department"]}
        assert by_dept["Style"]["perfect_cops"] == 1
        assert by_dept["Layout"]["diverging_cops"] == 1
        assert by_dept["Lint"]["inactive_cops"] == 1

        md = out_md.read_text()
        assert "## Summary" in md
        assert "## Per-Repo Results" in md
        assert "| Cops with exact match | 1 |" in md
        assert "| Department | Total cops | Exact match | Diverging | No corpus data |" in md
        assert "1 cops match RuboCop exactly. 1 cops have no corpus data." in md


def test_match_rate_never_rounds_up_to_100():
    """Match rates with FP>0 must never display as 100.0% — regression test for
    the Performance department bug where FP=2 but Match=100.0%."""
    # Import the formatting functions directly
    sys.path.insert(0, str(SCRIPT.parent))
    from diff_results import fmt_pct, trunc4

    # Performance department case: 43303 matches, 2 FP, 0 FN
    rate = 43303 / 43305  # 0.99995...
    assert trunc4(rate) < 1.0, f"trunc4({rate}) should be < 1.0, got {trunc4(rate)}"
    assert fmt_pct(trunc4(rate)) != "100.0%", \
        f"fmt_pct(trunc4({rate})) should not be 100.0%, got {fmt_pct(trunc4(rate))}"
    assert fmt_pct(trunc4(rate)) == "99.9%", \
        f"Expected 99.9%, got {fmt_pct(trunc4(rate))}"

    # Edge cases: exact 1.0 should still show 100.0%
    assert fmt_pct(trunc4(1.0)) == "100.0%"

    # Just under: 999/1000 = 0.999 → 99.9%
    assert fmt_pct(trunc4(999 / 1000)) == "99.9%"

    # Very close: 9999/10000 = 0.9999 → 99.9%
    assert fmt_pct(trunc4(9999 / 10000)) == "99.9%"

    # 99999/100000 = 0.99999 → trunc4 = 0.9999 → 99.9%
    assert trunc4(99999 / 100000) == 0.9999
    assert fmt_pct(trunc4(99999 / 100000)) == "99.9%"


def test_end_to_end_near_perfect_not_100():
    """Verify that a department with FP>0 does NOT show 100.0% in markdown output."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        nc_dir = tmp / "nitrocop"
        rc_dir = tmp / "rubocop"
        nc_dir.mkdir()
        rc_dir.mkdir()

        # Create a repo with many matches and a few FPs for a single cop
        # to reproduce the Performance scenario (high match rate, small FP)
        rc_offenses = []
        nc_offenses = []
        for i in range(1, 101):
            rc_offenses.append({
                "path": f"repos/repo_a/file{i}.rb",
                "offenses": [{"location": {"line": 1}, "cop_name": "Performance/TestCop"}]
            })
            nc_offenses.append({
                "path": f"repos/repo_a/file{i}.rb", "line": 1,
                "cop_name": "Performance/TestCop"
            })

        # Add 1 FP (nitrocop-only offense on a file RuboCop inspected but found clean)
        nc_offenses.append({
            "path": "repos/repo_a/extra.rb", "line": 1,
            "cop_name": "Performance/TestCop"
        })
        rc_offenses.append({
            "path": "repos/repo_a/extra.rb",
            "offenses": []
        })

        nc_dir.joinpath("repo_a.json").write_text(json.dumps({"offenses": nc_offenses}))
        rc_dir.joinpath("repo_a.json").write_text(json.dumps({
            "files": rc_offenses,
            "summary": {"target_file_count": 101, "inspected_file_count": 101}
        }))

        manifest = tmp / "manifest.jsonl"
        manifest.write_text(json.dumps({"id": "repo_a"}) + "\n")

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
            capture_output=True, text=True,
        )
        assert result.returncode == 0, f"Script failed:\nstderr: {result.stderr}"

        data = json.loads(out_json.read_text())

        # Per-cop match rate must be < 1.0
        perf_cop = [c for c in data["by_cop"] if c["cop"] == "Performance/TestCop"][0]
        assert perf_cop["fp"] == 1
        assert perf_cop["match_rate"] < 1.0, \
            f"Per-cop match_rate should be < 1.0 with FP=1, got {perf_cop['match_rate']}"

        # Department match rate must be < 1.0
        perf_dept = [d for d in data["by_department"] if d["department"] == "Performance"][0]
        assert perf_dept["fp"] == 1
        assert perf_dept["match_rate"] < 1.0, \
            f"Department match_rate should be < 1.0 with FP=1, got {perf_dept['match_rate']}"

        # Markdown must NOT show 100.0% for Performance
        md = out_md.read_text()
        for line in md.splitlines():
            if "Performance" in line and "|" in line:
                assert "100.0%" not in line, \
                    f"Performance line should not show 100.0% with FP>0: {line}"


def test_example_order_is_stable():
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        nc_dir = tmp / "nitrocop"
        rc_dir = tmp / "rubocop"
        nc_dir.mkdir()
        rc_dir.mkdir()

        nc_dir.joinpath("repo_a.json").write_text(json.dumps({
            "offenses": [
                {"path": "repos/repo_a/z.rb", "line": 9, "cop_name": "Layout/TestCop"},
                {"path": "repos/repo_a/a.rb", "line": 2, "cop_name": "Layout/TestCop"},
                {"path": "repos/repo_a/m.rb", "line": 5, "cop_name": "Layout/TestCop"},
            ]
        }))
        rc_dir.joinpath("repo_a.json").write_text(json.dumps({
            "files": [
                {"path": "repos/repo_a/a.rb", "offenses": []},
                {"path": "repos/repo_a/m.rb", "offenses": []},
                {"path": "repos/repo_a/z.rb", "offenses": []},
            ],
            "summary": {"target_file_count": 3, "inspected_file_count": 3}
        }))

        manifest = tmp / "manifest.jsonl"
        manifest.write_text(json.dumps({"id": "repo_a"}) + "\n")
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
            capture_output=True, text=True,
        )
        assert result.returncode == 0, f"Script failed:\nstderr: {result.stderr}"

        data = json.loads(out_json.read_text())
        cop = [c for c in data["by_cop"] if c["cop"] == "Layout/TestCop"][0]
        assert cop["fp_examples"][:3] == [
            "a.rb:2",
            "m.rb:5",
            "z.rb:9",
        ]


if __name__ == "__main__":
    test_end_to_end()
    test_match_rate_never_rounds_up_to_100()
    test_end_to_end_near_perfect_not_100()
    test_example_order_is_stable()
    print("OK: all tests passed")
