#!/usr/bin/env python3
"""Tests for merge_include_gated.py."""

import json
import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).parents[2] / "bench" / "corpus" / "merge_include_gated.py"


def _make_main():
    """Create a minimal main corpus-results.json."""
    return {
        "schema": 1,
        "summary": {
            "total_repos": 100,
            "repos_perfect": 50,
            "repos_error": 0,
            "total_offenses_compared": 1000,
            "matches": 900,
            "total_fp": 40,
            "total_fn": 60,
            "overall_match_rate": 0.9,
            "registered_cops": 4,
            "exercised_cops": 2,
            "perfect_cops": 1,
            "diverging_cops": 1,
            "inactive_cops": 2,
        },
        "by_department": [
            {
                "department": "Rails",
                "matches": 0, "fp": 0, "fn": 0,
                "match_rate": 1.0, "cops": 2,
                "exercised_cops": 0, "perfect_cops": 0,
                "diverging_cops": 0, "inactive_cops": 2,
            },
            {
                "department": "Style",
                "matches": 900, "fp": 40, "fn": 60,
                "match_rate": 0.9, "cops": 2,
                "exercised_cops": 2, "perfect_cops": 1,
                "diverging_cops": 1, "inactive_cops": 0,
            },
        ],
        "by_cop": [
            {
                "cop": "Style/FrozenStringLiteralComment",
                "matches": 500, "fp": 0, "fn": 0,
                "rubocop_total": 500, "nitro_total_unfiltered": 500,
                "unique_repos": 80, "match_rate": 1.0,
                "exercised": True, "perfect_match": True, "diverging": False,
                "fp_examples": [], "fn_examples": [],
            },
            {
                "cop": "Style/StringLiterals",
                "matches": 400, "fp": 40, "fn": 60,
                "rubocop_total": 460, "nitro_total_unfiltered": 440,
                "unique_repos": 70, "match_rate": 0.8,
                "exercised": True, "perfect_match": False, "diverging": True,
                "fp_examples": ["repo1: foo.rb:10"], "fn_examples": ["repo2: bar.rb:20"],
            },
            {
                "cop": "Rails/ReversibleMigration",
                "matches": 0, "fp": 0, "fn": 0,
                "rubocop_total": 0, "nitro_total_unfiltered": 0,
                "unique_repos": 0, "match_rate": 1.0,
                "exercised": False, "perfect_match": False, "diverging": False,
                "fp_examples": [], "fn_examples": [],
            },
            {
                "cop": "Rails/ThreeStateBooleanColumn",
                "matches": 0, "fp": 0, "fn": 0,
                "rubocop_total": 0, "nitro_total_unfiltered": 0,
                "unique_repos": 0, "match_rate": 1.0,
                "exercised": False, "perfect_match": False, "diverging": False,
                "fp_examples": [], "fn_examples": [],
            },
        ],
        "by_repo": [],
        "cop_activity_repos": {},
        "by_repo_cop": {},
    }


def _make_ig():
    """Create include-gated results with activity for one cop."""
    return {
        "schema": 1,
        "summary": {},
        "by_department": [],
        "by_cop": [
            {
                "cop": "Rails/ReversibleMigration",
                "matches": 15, "fp": 2, "fn": 1,
                "rubocop_total": 16, "nitro_total_unfiltered": 17,
                "unique_repos": 5, "match_rate": 0.8333,
                "exercised": True, "perfect_match": False, "diverging": True,
                "fp_examples": ["repo_a: db/migrate/001.rb:5"],
                "fn_examples": ["repo_b: db/migrate/002.rb:10"],
            },
            {
                "cop": "Rails/ThreeStateBooleanColumn",
                "matches": 3, "fp": 0, "fn": 0,
                "rubocop_total": 3, "nitro_total_unfiltered": 3,
                "unique_repos": 2, "match_rate": 1.0,
                "exercised": True, "perfect_match": True, "diverging": False,
                "fp_examples": [], "fn_examples": [],
            },
        ],
        "by_repo": [],
        "cop_activity_repos": {
            "Rails/ReversibleMigration": ["repo_a", "repo_b", "repo_c"],
            "Rails/ThreeStateBooleanColumn": ["repo_a", "repo_d"],
        },
        "by_repo_cop": {
            "repo_a": {
                "Rails/ReversibleMigration": {"matches": 3, "fp": 1, "fn": 0, "nitro_unfiltered": 4},
            },
            "repo_b": {
                "Rails/ReversibleMigration": {"matches": 0, "fp": 0, "fn": 1, "nitro_unfiltered": 0},
            },
        },
    }


def _run_merge(main_data, ig_data):
    """Run the merge script and return parsed output."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        main_path = tmp / "main.json"
        ig_path = tmp / "ig.json"
        out_path = tmp / "merged.json"

        main_path.write_text(json.dumps(main_data))
        ig_path.write_text(json.dumps(ig_data))

        result = subprocess.run(
            [sys.executable, str(SCRIPT),
             "--main", str(main_path),
             "--include-gated", str(ig_path),
             "--output", str(out_path)],
            capture_output=True, text=True,
        )
        assert result.returncode == 0, f"Script failed:\nstdout: {result.stdout}\nstderr: {result.stderr}"
        return json.loads(out_path.read_text())


def test_basic_merge():
    """Merge replaces zero-activity cops with IG data."""
    merged = _run_merge(_make_main(), _make_ig())

    # Rails/ReversibleMigration should have IG data
    cop_map = {c["cop"]: c for c in merged["by_cop"]}
    rm = cop_map["Rails/ReversibleMigration"]
    assert rm["matches"] == 15
    assert rm["fp"] == 2
    assert rm["fn"] == 1
    assert rm["exercised"] is True
    assert rm["diverging"] is True

    # Rails/ThreeStateBooleanColumn should have IG data
    tsb = cop_map["Rails/ThreeStateBooleanColumn"]
    assert tsb["matches"] == 3
    assert tsb["fp"] == 0
    assert tsb["perfect_match"] is True

    # Style cops should be untouched
    fslc = cop_map["Style/FrozenStringLiteralComment"]
    assert fslc["matches"] == 500


def test_department_rebuild():
    """Department totals are rebuilt from updated by_cop."""
    merged = _run_merge(_make_main(), _make_ig())

    dept_map = {d["department"]: d for d in merged["by_department"]}
    rails = dept_map["Rails"]
    assert rails["matches"] == 18  # 15 + 3
    assert rails["fp"] == 2
    assert rails["fn"] == 1
    assert rails["exercised_cops"] == 2
    assert rails["diverging_cops"] == 1  # ReversibleMigration
    assert rails["perfect_cops"] == 1  # ThreeStateBooleanColumn
    assert rails["inactive_cops"] == 0

    # Style should be unchanged
    style = dept_map["Style"]
    assert style["matches"] == 900


def test_summary_recalculation():
    """Summary totals are recalculated from updated by_cop."""
    merged = _run_merge(_make_main(), _make_ig())

    s = merged["summary"]
    # 500 + 400 + 15 + 3 = 918 matches
    assert s["matches"] == 918
    # 0 + 40 + 2 + 0 = 42 FP
    assert s["total_fp"] == 42
    # 0 + 60 + 1 + 0 = 61 FN
    assert s["total_fn"] == 61
    assert s["exercised_cops"] == 4  # all 4 now exercised
    assert s["perfect_cops"] == 2  # FSLC + ThreeState
    assert s["diverging_cops"] == 2  # StringLiterals + ReversibleMigration
    assert s["inactive_cops"] == 0


def test_by_repo_cop_merge():
    """by_repo_cop entries from IG are added to main."""
    merged = _run_merge(_make_main(), _make_ig())

    assert "repo_a" in merged["by_repo_cop"]
    assert "Rails/ReversibleMigration" in merged["by_repo_cop"]["repo_a"]
    assert merged["by_repo_cop"]["repo_a"]["Rails/ReversibleMigration"]["fp"] == 1


def test_cop_activity_repos_merge():
    """cop_activity_repos entries from IG are added to main."""
    merged = _run_merge(_make_main(), _make_ig())

    assert "Rails/ReversibleMigration" in merged["cop_activity_repos"]
    assert merged["cop_activity_repos"]["Rails/ReversibleMigration"] == ["repo_a", "repo_b", "repo_c"]


def test_noop_when_ig_has_no_activity():
    """If IG has zero activity for all cops, merge is a no-op."""
    ig = _make_ig()
    for c in ig["by_cop"]:
        c["matches"] = c["fp"] = c["fn"] = 0
        c["exercised"] = False
        c["perfect_match"] = False
        c["diverging"] = False

    main = _make_main()
    merged = _run_merge(main, ig)

    # Summary should be unchanged
    assert merged["summary"]["matches"] == 900


def test_overrides_active_main_cop():
    """IG data is authoritative — it replaces main data even when main is active."""
    main = _make_main()
    # Give ReversibleMigration some data in main (e.g., from scan_roots asymmetry)
    for c in main["by_cop"]:
        if c["cop"] == "Rails/ReversibleMigration":
            c["matches"] = 10
            c["exercised"] = True
            c["perfect_match"] = True

    merged = _run_merge(main, _make_ig())

    # Should be replaced with IG data (matches=15)
    cop_map = {c["cop"]: c for c in merged["by_cop"]}
    assert cop_map["Rails/ReversibleMigration"]["matches"] == 15


def test_overwrite_in_place():
    """--output can be the same as --main (overwrite in place)."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        main_path = tmp / "corpus-results.json"
        ig_path = tmp / "ig.json"

        main_path.write_text(json.dumps(_make_main()))
        ig_path.write_text(json.dumps(_make_ig()))

        result = subprocess.run(
            [sys.executable, str(SCRIPT),
             "--main", str(main_path),
             "--include-gated", str(ig_path),
             "--output", str(main_path)],  # same as --main
            capture_output=True, text=True,
        )
        assert result.returncode == 0

        merged = json.loads(main_path.read_text())
        cop_map = {c["cop"]: c for c in merged["by_cop"]}
        assert cop_map["Rails/ReversibleMigration"]["matches"] == 15
