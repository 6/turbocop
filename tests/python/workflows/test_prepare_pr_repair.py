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
    assert result["guard_profile"] == "repair-rust-test"
    assert result["cop_check_failure"] is False
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
    assert result["guard_profile"] == "repair-cop-check"
    assert result["cop_check_failure"] is True
    assert any("scripts/check_cop.py" in command for command in result["verification_commands"])


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
    assert result["guard_profile"] == "repair-smoke"
    assert result["cop_check_failure"] is False
    assert any("cargo clippy" in command for command in result["verification_commands"])
    assert any("corpus_smoke_test.py" in command for command in result["verification_commands"])


def test_macos_only_failure_is_skipped():
    run = {
        "jobs": [
            make_job("build-and-test (macos-latest)", ["Build"]),
        ],
    }
    result = prepare_pr_repair.classify_run(run)
    assert result["route"] == "skip"
    assert result["backend"] == ""
    assert result["guard_profile"] == ""
    assert result["cop_check_failure"] is False


def test_python_only_failure_skips_as_infra():
    """Python-only failures are unrelated to cop changes and should skip repair."""
    run = {
        "jobs": [
            make_job("python", ["Python script tests"]),
        ],
    }
    result = prepare_pr_repair.classify_run(run)
    assert result["route"] == "skip"
    assert "infra" in result["reason"].lower()


def test_python_plus_rust_failure_uses_python_scope():
    """When Python AND Rust steps fail, route as easy with python scope."""
    run = {
        "jobs": [
            make_job("python-and-rust", ["Python script tests", "Test"]),
        ],
    }
    result = prepare_pr_repair.classify_run(run)
    assert result["route"] == "easy"
    assert result["guard_profile"] == "repair-python-workflow"


def test_fix_backend_used_as_default_for_hard_route():
    run = {
        "jobs": [
            make_job("cop-check", ["Check cops against corpus baseline"]),
        ],
    }
    result = prepare_pr_repair.classify_run(run, fix_backend="claude-oauth-hard")
    assert result["route"] == "hard"
    assert result["backend"] == "claude-oauth-hard"


def test_fix_backend_used_as_default_for_easy_route():
    run = {
        "jobs": [
            make_job("build-and-test (ubuntu-24.04)", ["Clippy", "Test"]),
        ],
    }
    result = prepare_pr_repair.classify_run(run, fix_backend="claude-oauth-normal")
    assert result["route"] == "easy"
    assert result["backend"] == "claude-oauth-normal"


def test_fix_backend_not_used_for_skip_route():
    run = {
        "jobs": [
            make_job("build-and-test (macos-latest)", ["Build"]),
        ],
    }
    result = prepare_pr_repair.classify_run(run, fix_backend="claude-oauth-hard")
    assert result["route"] == "skip"
    assert result["backend"] == ""


def test_fix_backend_falls_back_to_codex_hard_when_empty():
    run = {
        "jobs": [
            make_job("cop-check", ["Check cops against corpus baseline"]),
        ],
    }
    result = prepare_pr_repair.classify_run(run, fix_backend="")
    assert result["backend"] == "codex-hard"


def test_prompt_includes_route_and_failed_packet():
    run = {"number": 57, "workflowName": "Checks", "jobs": []}
    classification = {
        "route": "hard",
        "backend": "codex-hard",
        "guard_profile": "repair-cop-check",
        "reason": "cop-check: Check cops against corpus baseline",
        "verification_commands": ["cargo build --release", "python3 scripts/check_cop.py Foo/Bar"],
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
            "path": "/tmp/repair-corpus.json", "run_id": "123", "head_sha": "abc",
        },
    )
    assert "PR #130" in prompt
    assert "Selected backend: `codex / hard`" in prompt
    assert "Check cops against corpus baseline" in prompt
    assert "Keep the patch narrow." in prompt
    assert "Before making changes, read `docs/agent-ci.md`." in prompt
    assert "Do not repair this PR by reverting it back to `origin/main`" in prompt
    assert "empty PR is treated as a failed repair" in prompt
    assert "/tmp/repair-corpus.json" in prompt
    assert "read-only token is available in `GH_TOKEN`" in prompt
    assert prompt.index("## Local Corpus Context") < prompt.index("## Failed Checks Packet")


def test_normalize_log_strips_actions_prefix_and_cleanup_noise():
    raw = "\n".join(
        [
            "cop-check\tUNKNOWN STEP\t2026-03-23T01:25:57.6876571Z Results:",
            "cop-check\tUNKNOWN STEP\t2026-03-23T01:25:57.6877040Z   Expected (RuboCop):          540",
            "cop-check\tUNKNOWN STEP\t2026-03-23T01:25:57.6878585Z FAIL: FN increased from 0 to 38 (+38, threshold: 0)",
            "cop-check\tUNKNOWN STEP\t2026-03-23T01:25:57.7182573Z Post job cleanup.",
            "cop-check\tUNKNOWN STEP\t2026-03-23T01:25:57.8139436Z [command]/usr/bin/git version",
            "cop-check\tUNKNOWN STEP\t2026-03-23T01:25:57.8175848Z git version 2.53.0",
        ]
    )

    normalized = prepare_pr_repair.normalize_log(raw)

    assert "UNKNOWN STEP" not in normalized
    assert "Post job cleanup." not in normalized
    assert "[command]/usr/bin/git version" not in normalized
    assert "git version 2.53.0" not in normalized
    assert "Results:" in normalized
    assert "Expected (RuboCop):          540" in normalized
    assert "FAIL: FN increased from 0 to 38 (+38, threshold: 0)" in normalized


def test_prefetch_corpus_context_uses_runtime_env_paths():
    original_corpus = os.environ.get("REPAIR_CORPUS_FILE")
    tmpdir = Path(tempfile.mkdtemp())
    corpus_target = tmpdir / "repair" / "corpus.json"
    os.environ["REPAIR_CORPUS_FILE"] = str(corpus_target)

    copied = []

    def fake_download():
        return (Path("/source/corpus.json"), "run-123", "sha-abc")

    def fake_copy2(source, target):
        copied.append((str(source), str(target)))
        Path(target).parent.mkdir(parents=True, exist_ok=True)
        Path(target).write_text("{}")

    original_download = prepare_pr_repair._download_corpus
    original_copy2 = prepare_pr_repair.shutil.copy2
    prepare_pr_repair._download_corpus = fake_download
    prepare_pr_repair.shutil.copy2 = fake_copy2
    try:
        context = prepare_pr_repair.prefetch_corpus_context("hard")
    finally:
        prepare_pr_repair._download_corpus = original_download
        prepare_pr_repair.shutil.copy2 = original_copy2
        if original_corpus is None:
            os.environ.pop("REPAIR_CORPUS_FILE", None)
        else:
            os.environ["REPAIR_CORPUS_FILE"] = original_corpus
        if tmpdir.exists():
            shutil.rmtree(tmpdir)

    assert context["path"] == str(corpus_target)
    assert copied == [
        ("/source/corpus.json", str(corpus_target)),
    ]


if __name__ == "__main__":
    test_easy_linux_failure_routes_to_codex()
    test_hard_cop_check_routes_to_codex()
    test_mixed_failures_escalate_to_hard()
    test_macos_only_failure_is_skipped()
    test_python_only_failure_skips_as_infra()
    test_python_plus_rust_failure_uses_python_scope()
    test_fix_backend_used_as_default_for_hard_route()
    test_fix_backend_used_as_default_for_easy_route()
    test_fix_backend_not_used_for_skip_route()
    test_fix_backend_falls_back_to_codex_hard_when_empty()
    test_prompt_includes_route_and_failed_packet()
    test_normalize_log_strips_actions_prefix_and_cleanup_noise()
    test_prefetch_corpus_context_uses_runtime_env_paths()
    print("All tests passed.")
