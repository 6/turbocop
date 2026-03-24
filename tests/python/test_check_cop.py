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


def test_clone_repos_for_cop_creates_corpus_dir_for_zero_divergence():
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        original_corpus_dir = check_cop.CORPUS_DIR
        original_manifest_path = check_cop.MANIFEST_PATH
        try:
            check_cop.CORPUS_DIR = tmp_path / "vendor" / "corpus"
            check_cop.MANIFEST_PATH = tmp_path / "bench" / "corpus" / "manifest.jsonl"
            write_manifest(check_cop.MANIFEST_PATH)

            check_cop.clone_repos_for_cop("Style/MixinUsage", {"by_repo_cop": {}})

            assert check_cop.CORPUS_DIR.exists()
            assert list(check_cop.CORPUS_DIR.iterdir()) == []
        finally:
            check_cop.CORPUS_DIR = original_corpus_dir
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


def test_clone_repos_for_cop_reclones_when_existing_sha_mismatches():
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        original_corpus_dir = check_cop.CORPUS_DIR
        original_manifest_path = check_cop.MANIFEST_PATH
        original_repo_head_sha = check_cop.repo_head_sha
        original_subprocess_run = check_cop.subprocess.run
        try:
            check_cop.CORPUS_DIR = tmp_path / "vendor" / "corpus"
            check_cop.MANIFEST_PATH = tmp_path / "bench" / "corpus" / "manifest.jsonl"
            write_manifest(check_cop.MANIFEST_PATH)

            existing = check_cop.CORPUS_DIR / "demo-repo"
            existing.mkdir(parents=True, exist_ok=True)
            existing.joinpath("placeholder.txt").write_text("old clone")

            check_cop.repo_head_sha = lambda path: "cafebabe"

            calls = []

            class FakeResult:
                def __init__(self):
                    self.returncode = 0
                    self.stdout = ""
                    self.stderr = ""

            def fake_run(cmd, **kwargs):
                calls.append(cmd)
                return FakeResult()

            check_cop.subprocess.run = fake_run

            check_cop.clone_repos_for_cop(
                "Style/MixinUsage",
                {"cop_activity_repos": {"Style/MixinUsage": ["demo-repo"]}, "by_repo_cop": {}},
            )

            assert any(cmd[:4] == ["git", "init", str(existing)] for cmd in calls)
            assert any(cmd[:4] == ["git", "-C", str(existing), "fetch"] for cmd in calls)
        finally:
            check_cop.CORPUS_DIR = original_corpus_dir
            check_cop.MANIFEST_PATH = original_manifest_path
            check_cop.repo_head_sha = original_repo_head_sha
            check_cop.subprocess.run = original_subprocess_run


def test_rerun_local_per_repo_always_uses_per_repo_mode():
    original_ensure_binary_fresh = check_cop.ensure_binary_fresh
    original_clear_file_cache = check_cop.clear_file_cache
    original_run_nitrocop_per_repo = check_cop.run_nitrocop_per_repo
    try:
        calls = []

        check_cop.ensure_binary_fresh = lambda: calls.append("fresh")
        check_cop.clear_file_cache = lambda: calls.append("clear")

        def fake_per_repo(_cop_name, relevant_repos=None, **_kwargs):
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


if __name__ == "__main__":
    test_clone_repos_for_cop_creates_corpus_dir_for_zero_divergence()
    test_relevant_repos_for_cop_unions_activity_and_divergence()
    test_run_nitrocop_per_repo_skips_missing_corpus_when_no_relevant_repos()
    test_run_nitrocop_per_repo_errors_on_missing_required_repos()
    test_clone_repos_for_cop_reclones_when_existing_sha_mismatches()
    test_rerun_local_per_repo_always_uses_per_repo_mode()
    print("All tests passed.")
