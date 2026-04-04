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


def _write_manifest(path, repo_ids):
    """Write a minimal manifest.jsonl with the given repo IDs."""
    with open(path, "w") as f:
        for rid in repo_ids:
            f.write(json.dumps({"id": rid, "repo_url": f"https://example.com/{rid}", "sha": "abc"}) + "\n")


def test_repo_manifest_index_found(tmp_path):
    manifest = tmp_path / "manifest.jsonl"
    _write_manifest(manifest, ["repo_a", "repo_b", "repo_c"])
    assert run_variant_batches._repo_manifest_index("repo_a", manifest) == 0
    assert run_variant_batches._repo_manifest_index("repo_b", manifest) == 1
    assert run_variant_batches._repo_manifest_index("repo_c", manifest) == 2


def test_repo_manifest_index_not_found(tmp_path):
    manifest = tmp_path / "manifest.jsonl"
    _write_manifest(manifest, ["repo_a", "repo_b"])
    assert run_variant_batches._repo_manifest_index("repo_z", manifest) is None


def test_run_variant_batches_skips_repo_beyond_limit(tmp_path):
    """Repos beyond max_variant_repos are skipped."""
    manifest = tmp_path / "manifest.jsonl"
    _write_manifest(manifest, ["repo_0", "repo_1", "repo_2"])

    # repo_2 is at index 2, limit is 2 → should be skipped
    result = run_variant_batches.run_variant_batches(
        repo_dir=str(tmp_path),
        repo_id="repo_2",
        binary="/nonexistent",
        batches_dir=str(tmp_path / "batches"),
        results_dir=str(tmp_path / "results"),
        manifest=str(manifest),
        max_variant_repos=2,
    )
    assert result == []


def test_run_variant_batches_runs_repo_within_limit(tmp_path):
    """Repos within max_variant_repos are not skipped (but may return [] if no batches)."""
    manifest = tmp_path / "manifest.jsonl"
    _write_manifest(manifest, ["repo_0", "repo_1", "repo_2"])

    # repo_1 is at index 1, limit is 2 → should proceed (returns [] because no batch configs)
    result = run_variant_batches.run_variant_batches(
        repo_dir=str(tmp_path),
        repo_id="repo_1",
        binary="/nonexistent",
        batches_dir=str(tmp_path / "batches"),
        results_dir=str(tmp_path / "results"),
        manifest=str(manifest),
        max_variant_repos=2,
    )
    assert result == []


def test_run_variant_batches_no_limit_runs_all(tmp_path):
    """Without manifest/limit args, no repos are skipped."""
    # No manifest → proceeds to discover_batches (returns [] because no batch configs)
    result = run_variant_batches.run_variant_batches(
        repo_dir=str(tmp_path),
        repo_id="anything",
        binary="/nonexistent",
        batches_dir=str(tmp_path / "batches"),
        results_dir=str(tmp_path / "results"),
    )
    assert result == []


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
