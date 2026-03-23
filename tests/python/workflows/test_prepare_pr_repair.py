#!/usr/bin/env python3
"""Tests for prepare_pr_repair.py."""
import os
import shutil
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[3] / "scripts" / "workflows"))
import prepare_pr_repair


def make_job(name: str, failed_steps: list[str], conclusion: str = "failure") -> dict:
    steps = [{"name": step, "conclusion": "failure"} for step in failed_steps]
    return {"name": name, "conclusion": conclusion, "steps": steps, "databaseId": 1}


def test_easy_linux_failure_routes_to_codex():
    run = {
        "jobs": [
            make_job("build-and-test (ubuntu-24.04)", ["Clippy", "Test"]),
        ],
    }
    result = prepare_pr_repair.classify_run(run)
    assert result["route"] == "easy"
    assert result["backend"] == "codex-hard"
    assert "cargo clippy --profile ci -- -D warnings" in result["verification_commands"]
    assert "cargo test" in result["verification_commands"]


def test_hard_cop_check_routes_to_codex():
    run = {
        "jobs": [
            make_job("cop-check", ["Check cops against corpus baseline"]),
        ],
    }
    result = prepare_pr_repair.classify_run(run)
    assert result["route"] == "hard"
    assert result["backend"] == "codex-hard"
    assert any("scripts/check-cop.py" in command for command in result["verification_commands"])


def test_mixed_failures_escalate_to_hard():
    run = {
        "jobs": [
            make_job("build-and-test (ubuntu-24.04)", ["Clippy"]),
            make_job("corpus-smoke-test", ["Run smoke test"]),
        ],
    }
    result = prepare_pr_repair.classify_run(run)
    assert result["route"] == "hard"
    assert result["backend"] == "codex-hard"
    assert any("cargo clippy" in command for command in result["verification_commands"])
    assert any("corpus-smoke-test.py" in command for command in result["verification_commands"])


def test_macos_only_failure_is_skipped():
    run = {
        "jobs": [
            make_job("build-and-test (macos-latest)", ["Build"]),
        ],
    }
    result = prepare_pr_repair.classify_run(run)
    assert result["route"] == "skip"
    assert result["backend"] == ""


def test_prompt_includes_route_and_failed_packet():
    run = {"number": 57, "workflowName": "Checks", "jobs": []}
    classification = {
        "route": "hard",
        "backend": "codex-hard",
        "reason": "cop-check: Check cops against corpus baseline",
        "verification_commands": ["cargo build --release", "python3 scripts/check-cop.py Foo/Bar"],
        "jobs": [
            {
                "name": "cop-check",
                "repair_route": "hard",
                "failed_step_names": ["Check cops against corpus baseline"],
                "failed_log": "Traceback\nFileNotFoundError",
            }
        ],
    }
    prompt = prepare_pr_repair.build_prompt(
        run=run,
        classification=classification,
        pr_meta={"number": "130", "title": "Fix Style/MixinUsage", "headRefName": "fix/style-mixin_usage"},
        diff_stat=" 1 file changed",
        diff_text="diff --git a/a b/a\n+change\n",
        extra_context="Keep the patch narrow.",
        corpus_context={
            "standard": {"path": "/tmp/repair-corpus-standard.json", "run_id": "123", "head_sha": "abc"},
            "extended": {"path": "/tmp/repair-corpus-extended.json", "run_id": "456", "head_sha": "def"},
        },
    )
    assert "PR #130" in prompt
    assert "Selected backend: `codex / hard`" in prompt
    assert "Check cops against corpus baseline" in prompt
    assert "Keep the patch narrow." in prompt
    assert "Do not repair this PR by reverting it back to `origin/main`" in prompt
    assert "empty PR is treated as a failed repair" in prompt
    assert "/tmp/repair-corpus-standard.json" in prompt
    assert "/tmp/repair-corpus-extended.json" in prompt
    assert "read-only token is available in `GH_TOKEN`" in prompt


def test_prefetch_corpus_context_uses_runtime_env_paths():
    original_standard = os.environ.get("REPAIR_CORPUS_STANDARD_FILE")
    original_extended = os.environ.get("REPAIR_CORPUS_EXTENDED_FILE")
    tmpdir = Path(tempfile.mkdtemp())
    standard_target = tmpdir / "repair" / "corpus-standard.json"
    extended_target = tmpdir / "repair" / "corpus-extended.json"
    os.environ["REPAIR_CORPUS_STANDARD_FILE"] = str(standard_target)
    os.environ["REPAIR_CORPUS_EXTENDED_FILE"] = str(extended_target)

    copied = []

    def fake_download(prefer: str):
        return (Path(f"/source/{prefer}.json"), f"{prefer}-run", f"{prefer}-sha")

    def fake_copy2(source, target):
        copied.append((str(source), str(target)))
        Path(target).parent.mkdir(parents=True, exist_ok=True)
        Path(target).write_text("{}")

    original_download = prepare_pr_repair._download_corpus
    original_copy2 = prepare_pr_repair.shutil.copy2
    prepare_pr_repair._download_corpus = fake_download
    prepare_pr_repair.shutil.copy2 = fake_copy2
    try:
        contexts = prepare_pr_repair.prefetch_corpus_context("hard")
    finally:
        prepare_pr_repair._download_corpus = original_download
        prepare_pr_repair.shutil.copy2 = original_copy2
        if original_standard is None:
            os.environ.pop("REPAIR_CORPUS_STANDARD_FILE", None)
        else:
            os.environ["REPAIR_CORPUS_STANDARD_FILE"] = original_standard
        if original_extended is None:
            os.environ.pop("REPAIR_CORPUS_EXTENDED_FILE", None)
        else:
            os.environ["REPAIR_CORPUS_EXTENDED_FILE"] = original_extended
        if tmpdir.exists():
            shutil.rmtree(tmpdir)

    assert contexts["standard"]["path"] == str(standard_target)
    assert contexts["extended"]["path"] == str(extended_target)
    assert copied == [
        ("/source/standard.json", str(standard_target)),
        ("/source/extended.json", str(extended_target)),
    ]


if __name__ == "__main__":
    test_easy_linux_failure_routes_to_codex()
    test_hard_cop_check_routes_to_codex()
    test_mixed_failures_escalate_to_hard()
    test_macos_only_failure_is_skipped()
    test_prompt_includes_route_and_failed_packet()
    test_prefetch_corpus_context_uses_runtime_env_paths()
    print("All tests passed.")
