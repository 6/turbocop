#!/usr/bin/env python3
"""Tests for bench/corpus/compute_batches.py."""

import importlib.util
from pathlib import Path

SCRIPT = Path(__file__).parents[2] / "bench" / "corpus" / "compute_batches.py"
SPEC = importlib.util.spec_from_file_location("compute_batches", SCRIPT)
assert SPEC and SPEC.loader
compute_batches = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(compute_batches)


compute_matrix = compute_batches.compute_matrix
compute_batch_repos = compute_batches.compute_batch_repos


def _make_repos(n):
    return [{"id": f"repo_{i}", "repo_url": f"https://example.com/{i}", "sha": f"sha{i}"} for i in range(n)]


def test_compute_matrix_basic():
    repos = _make_repos(10)
    result = compute_matrix(repos, batch_size=3)
    # 10 repos / 3 = 4 batches (ceil)
    normal = [b for b in result["include"] if b["is_large"] == "false"]
    assert len(normal) == 4
    assert all(b["num_batches"] == "4" for b in normal)


def test_compute_matrix_large_repos_get_solo_batches():
    repos = _make_repos(10)
    large_ids = {"repo_0", "repo_5"}
    result = compute_matrix(repos, batch_size=3, large_repo_ids=large_ids)

    normal = [b for b in result["include"] if b["is_large"] == "false"]
    large = [b for b in result["include"] if b["is_large"] == "true"]

    # 8 normal repos / 3 = 3 batches
    assert len(normal) == 3
    # 2 large repos = 2 solo batches
    assert len(large) == 2
    large_ids_in_matrix = {b["batch_id"].replace("large-", "") for b in large}
    assert large_ids_in_matrix == {"repo_0", "repo_5"}


def test_compute_matrix_no_large_repos():
    repos = _make_repos(5)
    result = compute_matrix(repos, batch_size=2)
    large = [b for b in result["include"] if b["is_large"] == "true"]
    assert len(large) == 0


def test_compute_matrix_all_large():
    repos = _make_repos(3)
    large_ids = {"repo_0", "repo_1", "repo_2"}
    result = compute_matrix(repos, batch_size=5, large_repo_ids=large_ids)

    normal = [b for b in result["include"] if b["is_large"] == "false"]
    large = [b for b in result["include"] if b["is_large"] == "true"]
    assert len(normal) == 0
    assert len(large) == 3


def test_compute_matrix_stable_hash():
    """Same repos produce same batch hashes."""
    repos = _make_repos(10)
    r1 = compute_matrix(repos, batch_size=3)
    r2 = compute_matrix(repos, batch_size=3)
    hashes1 = [b["batch_hash"] for b in r1["include"]]
    hashes2 = [b["batch_hash"] for b in r2["include"]]
    assert hashes1 == hashes2


def test_compute_batch_repos_normal():
    repos = _make_repos(10)
    # With batch_size=3 → 4 batches, round-robin
    chunk = compute_batch_repos(repos, "0", num_batches=4)
    # Repos 0, 4, 8 should be in batch 0
    ids = [r["id"] for r in chunk]
    assert ids == ["repo_0", "repo_4", "repo_8"]


def test_compute_batch_repos_excludes_large():
    repos = _make_repos(10)
    large_ids = {"repo_0", "repo_5"}
    # 8 normal repos / 3 = 3 batches
    chunk = compute_batch_repos(repos, "0", num_batches=3, large_repo_ids=large_ids)
    ids = [r["id"] for r in chunk]
    assert "repo_0" not in ids
    assert "repo_5" not in ids


def test_compute_batch_repos_large():
    repos = _make_repos(10)
    chunk = compute_batch_repos(repos, "large-repo_3", num_batches=4, is_large=True)
    assert len(chunk) == 1
    assert chunk[0]["id"] == "repo_3"


def test_compute_batch_repos_large_missing():
    repos = _make_repos(5)
    chunk = compute_batch_repos(repos, "large-repo_999", num_batches=2, is_large=True)
    assert chunk == []
