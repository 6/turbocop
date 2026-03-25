#!/usr/bin/env python3
"""Tests for check_cop.py."""
import importlib.util
import json
import sys
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).parents[2] / "scripts" / "check_cop.py"
sys.path.insert(0, str(SCRIPT.parent))
SPEC = importlib.util.spec_from_file_location("check_cop", SCRIPT)
assert SPEC and SPEC.loader
check_cop = importlib.util.module_from_spec(SPEC)
sys.modules["check_cop"] = check_cop
SPEC.loader.exec_module(check_cop)


def write_manifest(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    entry = {
        "id": "demo-repo",
        "repo_url": "https://example.com/demo.git",
        "sha": "deadbeef",
    }
    path.write_text(json.dumps(entry) + "\n")


def test_clone_repos_for_cop_creates_temp_dir_for_zero_divergence():
    original_manifest_path = check_cop.MANIFEST_PATH
    try:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            check_cop.MANIFEST_PATH = tmp_path / "bench" / "corpus" / "manifest.jsonl"
            write_manifest(check_cop.MANIFEST_PATH)

            result = check_cop.clone_repos_for_cop("Style/MixinUsage", {"by_repo_cop": {}})
            # Returns a temp dir with repos/ subdirectory
            assert (result / "repos").exists()
    finally:
        check_cop.MANIFEST_PATH = original_manifest_path


def test_relevant_repos_for_cop_unions_activity_and_divergence():
    data = {
        "cop_activity_repos": {
            "Style/MixinUsage": ["repo-active"],
        },
        "by_repo_cop": {
            "repo-diverging": {
                "Style/MixinUsage": {"matches": 0, "fp": 1, "fn": 0},
            },
        },
    }
    assert check_cop.relevant_repos_for_cop("Style/MixinUsage", data) == {
        "repo-active",
        "repo-diverging",
    }


def test_run_nitrocop_per_repo_skips_missing_corpus_when_no_relevant_repos():
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        original_corpus_dir = check_cop.CORPUS_DIR
        try:
            check_cop.CORPUS_DIR = tmp_path / "vendor" / "corpus"
            result = check_cop.run_nitrocop_per_repo(
                "Style/MixinUsage",
                relevant_repos=set(),
            )
            assert result == {}
        finally:
            check_cop.CORPUS_DIR = original_corpus_dir


def test_run_nitrocop_per_repo_errors_on_missing_required_repos():
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        original_corpus_dir = check_cop.CORPUS_DIR
        try:
            check_cop.CORPUS_DIR = tmp_path / "vendor" / "corpus"
            check_cop.CORPUS_DIR.mkdir(parents=True, exist_ok=True)
            try:
                check_cop.run_nitrocop_per_repo(
                    "Style/MixinUsage",
                    relevant_repos={"missing-repo"},
                )
                raise AssertionError("expected FileNotFoundError")
            except FileNotFoundError as exc:
                assert "missing-repo" in str(exc)
                assert str(check_cop.CORPUS_DIR) in str(exc)
        finally:
            check_cop.CORPUS_DIR = original_corpus_dir


def test_clone_repos_for_cop_uses_shared_clone_module():
    """clone_repos_for_cop delegates to the shared clone_repos module."""
    original_manifest_path = check_cop.MANIFEST_PATH
    original_clone = check_cop._clone_repos
    try:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            check_cop.MANIFEST_PATH = tmp_path / "manifest.jsonl"
            write_manifest(check_cop.MANIFEST_PATH)

            calls = []
            check_cop._clone_repos = lambda dest, manifest, repo_ids=None, parallel=3: calls.append(
                {"dest": str(dest), "ids": repo_ids}
            ) or 0

            result = check_cop.clone_repos_for_cop(
                "Style/MixinUsage",
                {"cop_activity_repos": {"Style/MixinUsage": ["demo-repo"]}, "by_repo_cop": {}},
            )

            assert len(calls) == 1
            assert calls[0]["ids"] == {"demo-repo"}
            assert (result / "repos").parent == result
    finally:
        check_cop.MANIFEST_PATH = original_manifest_path
        check_cop._clone_repos = original_clone


def test_rerun_local_per_repo_always_uses_per_repo_mode():
    original_ensure_binary_fresh = check_cop.ensure_binary_fresh
    original_clear_file_cache = check_cop.clear_file_cache
    original_run_nitrocop_per_repo = check_cop.run_nitrocop_per_repo
    try:
        calls = []

        check_cop.ensure_binary_fresh = lambda: calls.append("fresh")
        check_cop.clear_file_cache = lambda: calls.append("clear")

        def fake_per_repo(_cop_name, relevant_repos=None, **_kw):
            calls.append(("per_repo", relevant_repos))
            return {"repo-a": 2}

        check_cop.run_nitrocop_per_repo = fake_per_repo

        result = check_cop.rerun_local_per_repo(
            "Style/MixinUsage",
            {
                "cop_activity_repos": {"Style/MixinUsage": ["repo-a"]},
                "by_repo_cop": {},
            },
            quick=True,
            has_activity_index=True,
        )

        assert result == {"repo-a": 2}
        assert ("per_repo", {"repo-a"}) in calls
    finally:
        check_cop.ensure_binary_fresh = original_ensure_binary_fresh
        check_cop.clear_file_cache = original_clear_file_cache
        check_cop.run_nitrocop_per_repo = original_run_nitrocop_per_repo


def test_per_repo_gate_compares_against_rubocop_not_old_nitrocop():
    """The per-repo gate should compare local nitrocop vs RuboCop count (matches+fn),
    not vs old nitrocop count (matches+fp). A cop fix that moves closer to RuboCop
    should pass, even if it changes the nitrocop count."""
    by_repo_cop = {
        "repo-a": {
            "Style/Foo": {"matches": 10, "fp": 2, "fn": 3, "nitro_unfiltered": 12},
            # Old nitrocop: 12 (matches+fp), RuboCop: 13 (matches+fn)
        },
    }
    activity_counts = {}
    for repo_id, cops in by_repo_cop.items():
        if "Style/Foo" in cops:
            entry = cops["Style/Foo"]
            # Should use matches + fn (RuboCop = 13), not matches + fp (old nitrocop = 12)
            activity_counts[repo_id] = entry.get("matches", 0) + entry.get("fn", 0)

    # Case 1: local=13 matches RuboCop exactly → no regression
    local_count = 13
    diff = local_count - activity_counts["repo-a"]
    assert diff == 0, f"Expected 0 diff, got {diff} (local={local_count}, rubocop={activity_counts['repo-a']})"

    # Case 2: local=12 (old nitrocop count) → FN=1 vs RuboCop
    local_count = 12
    diff = local_count - activity_counts["repo-a"]
    assert diff == -1, f"Expected -1 diff (FN), got {diff}"

    # Case 3: local=14 → FP=1 vs RuboCop
    local_count = 14
    diff = local_count - activity_counts["repo-a"]
    assert diff == 1, f"Expected +1 diff (FP), got {diff}"


if __name__ == "__main__":
    test_clone_repos_for_cop_creates_temp_dir_for_zero_divergence()
    test_relevant_repos_for_cop_unions_activity_and_divergence()
    test_run_nitrocop_per_repo_skips_missing_corpus_when_no_relevant_repos()
    test_run_nitrocop_per_repo_errors_on_missing_required_repos()
    test_clone_repos_for_cop_uses_shared_clone_module()
    test_rerun_local_per_repo_always_uses_per_repo_mode()
    test_per_repo_gate_compares_against_rubocop_not_old_nitrocop()
    print("All tests passed.")
