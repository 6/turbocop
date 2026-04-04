#!/usr/bin/env python3
"""Tests for bench/corpus/consolidate_cache.py."""

import importlib.util
import json
from pathlib import Path

SCRIPT = Path(__file__).parents[2] / "bench" / "corpus" / "consolidate_cache.py"
SPEC = importlib.util.spec_from_file_location("consolidate_cache", SCRIPT)
assert SPEC and SPEC.loader
consolidate_cache = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(consolidate_cache)


def _write_manifest(path, repos):
    """Write manifest.jsonl with [{id, sha}, ...]."""
    with open(path, "w") as f:
        for r in repos:
            f.write(json.dumps(r) + "\n")


def test_consolidate_baseline(tmp_path):
    manifest = tmp_path / "manifest.jsonl"
    _write_manifest(manifest, [
        {"id": "repo_a", "sha": "aaa"},
        {"id": "repo_b", "sha": "bbb"},
    ])

    src = tmp_path / "rubocop"
    src.mkdir()
    (src / "repo_a.json").write_text('{"files": []}')
    (src / "repo_b.json").write_text('{"files": []}')
    (src / "repo_a.err").write_text("")  # non-json files should be skipped

    dest = tmp_path / "cache"
    count = consolidate_cache.consolidate_baseline(str(manifest), str(src), str(dest))

    assert count == 2
    assert (dest / "repo_a_aaa.json").exists()
    assert (dest / "repo_b_bbb.json").exists()


def test_consolidate_baseline_missing_source(tmp_path):
    manifest = tmp_path / "manifest.jsonl"
    _write_manifest(manifest, [{"id": "repo_a", "sha": "aaa"}])

    count = consolidate_cache.consolidate_baseline(
        str(manifest), str(tmp_path / "nonexistent"), str(tmp_path / "cache"))
    assert count == 0


def test_consolidate_baseline_unknown_repo(tmp_path):
    """Repos not in the manifest are skipped."""
    manifest = tmp_path / "manifest.jsonl"
    _write_manifest(manifest, [{"id": "repo_a", "sha": "aaa"}])

    src = tmp_path / "rubocop"
    src.mkdir()
    (src / "repo_a.json").write_text("{}")
    (src / "repo_unknown.json").write_text("{}")

    dest = tmp_path / "cache"
    count = consolidate_cache.consolidate_baseline(str(manifest), str(src), str(dest))

    assert count == 1
    assert (dest / "repo_a_aaa.json").exists()
    assert not (dest / "repo_unknown_aaa.json").exists()


def test_consolidate_variants(tmp_path):
    manifest = tmp_path / "manifest.jsonl"
    _write_manifest(manifest, [
        {"id": "repo_a", "sha": "aaa"},
        {"id": "repo_b", "sha": "bbb"},
    ])

    src = tmp_path / "results"
    src.mkdir()
    batch1 = src / "variant-rubocop-variant_batch_1"
    batch1.mkdir()
    (batch1 / "repo_a.json").write_text("{}")
    (batch1 / "repo_b.json").write_text("{}")

    batch2 = src / "variant-rubocop-variant_batch_2"
    batch2.mkdir()
    (batch2 / "repo_a.json").write_text("{}")

    # Non-matching directory should be ignored
    (src / "variant-nitrocop-variant_batch_1").mkdir()

    dest = tmp_path / "cache"
    count = consolidate_cache.consolidate_variants(str(manifest), str(src), str(dest))

    assert count == 3
    assert (dest / "repo_a_aaa_variant_batch_1.json").exists()
    assert (dest / "repo_b_bbb_variant_batch_1.json").exists()
    assert (dest / "repo_a_aaa_variant_batch_2.json").exists()


def test_consolidate_variants_missing_source(tmp_path):
    manifest = tmp_path / "manifest.jsonl"
    _write_manifest(manifest, [{"id": "repo_a", "sha": "aaa"}])

    count = consolidate_cache.consolidate_variants(
        str(manifest), str(tmp_path / "nonexistent"), str(tmp_path / "cache"))
    assert count == 0
