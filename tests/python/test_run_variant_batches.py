#!/usr/bin/env python3
"""Tests for bench/corpus/run_variant_batches.py."""

import importlib.util
import json
from pathlib import Path

SCRIPT = Path(__file__).parents[2] / "bench" / "corpus" / "run_variant_batches.py"
SPEC = importlib.util.spec_from_file_location("run_variant_batches", SCRIPT)
assert SPEC and SPEC.loader
run_variant_batches = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(run_variant_batches)


def test_discover_batches_empty(tmp_path):
    """No batches in empty directory."""
    assert run_variant_batches.discover_batches(tmp_path) == []


def test_discover_batches_finds_files(tmp_path):
    """Discovers variant_batch_*.yml files, sorted."""
    (tmp_path / "variant_batch_2.yml").write_text("x: 1\n")
    (tmp_path / "variant_batch_1.yml").write_text("x: 1\n")
    (tmp_path / "other.yml").write_text("x: 1\n")  # should be ignored
    batches = run_variant_batches.discover_batches(tmp_path)
    assert len(batches) == 2
    assert batches[0].name == "variant_batch_1.yml"
    assert batches[1].name == "variant_batch_2.yml"


def test_discover_batches_nonexistent_dir():
    """Returns empty list for nonexistent directory."""
    assert run_variant_batches.discover_batches("/nonexistent/path") == []


def test_merge_variant_results_empty():
    """Merging empty list produces empty batches."""
    result = run_variant_batches.merge_variant_results([])
    assert result == {"batches": []}


def test_merge_variant_results(tmp_path):
    """Merge multiple result files into combined summary."""
    f1 = tmp_path / "style-variant-variant_batch_1.json"
    f1.write_text(json.dumps({
        "summary": {"total_repos": 10, "matches": 100, "fp": 5, "fn": 3},
        "by_cop": [{"cop": "Style/Foo", "matches": 50, "fp": 2, "fn": 1}],
    }))
    f2 = tmp_path / "style-variant-variant_batch_2.json"
    f2.write_text(json.dumps({
        "summary": {"total_repos": 10, "matches": 90, "fp": 8, "fn": 2},
        "by_cop": [{"cop": "Style/Foo", "matches": 40, "fp": 5, "fn": 0}],
    }))

    result = run_variant_batches.merge_variant_results([f1, f2])
    assert len(result["batches"]) == 2
    assert result["batches"][0]["name"] == "variant_batch_1"
    assert result["batches"][0]["total_repos"] == 10
    assert result["batches"][0]["fp"] == 5
    assert result["batches"][1]["name"] == "variant_batch_2"
    assert "by_cop" in result["batches"][0]


def test_merge_variant_results_skips_bad_json(tmp_path):
    """Bad JSON files are skipped, not crashed on."""
    good = tmp_path / "style-variant-variant_batch_1.json"
    good.write_text(json.dumps({"summary": {"fp": 0}}))
    bad = tmp_path / "style-variant-variant_batch_2.json"
    bad.write_text("not json")

    result = run_variant_batches.merge_variant_results([good, bad])
    assert len(result["batches"]) == 1
