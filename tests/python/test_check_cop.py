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


def _compute_gate(by_repo_cop, cop, per_repo):
    """Replicate the gate logic from check_cop.py for testing.

    Returns dict with: new_fp, new_fn, resolved_fp, resolved_fn, net_fp, net_fn,
    total_baseline_fp, total_baseline_fn, total_local_fp, total_local_fn.
    """
    oracle_nitrocop_counts = {}
    oracle_rubocop_counts = {}
    for repo_id, cops in by_repo_cop.items():
        if cop in cops:
            entry = cops[cop]
            matches = entry.get("matches", 0)
            fp = entry.get("fp", 0)
            fn = entry.get("fn", 0)
            oracle_nitrocop_counts[repo_id] = matches + fp
            oracle_rubocop_counts[repo_id] = matches + fn

    new_fp, new_fn = 0, 0
    resolved_fp, resolved_fn = 0, 0
    total_baseline_fp, total_baseline_fn = 0, 0
    total_local_fp, total_local_fn = 0, 0
    for repo_id, local_count in per_repo.items():
        bl_nc = oracle_nitrocop_counts.get(repo_id)
        bl_rc = oracle_rubocop_counts.get(repo_id)
        if bl_nc is None or bl_rc is None:
            continue
        baseline_fp = max(0, bl_nc - bl_rc)
        baseline_fn = max(0, bl_rc - bl_nc)
        total_baseline_fp += baseline_fp
        total_baseline_fn += baseline_fn
        local_fp = max(0, local_count - bl_rc)
        local_fn = max(0, bl_rc - local_count)
        total_local_fp += local_fp
        total_local_fn += local_fn
        fp_increase = max(0, local_fp - baseline_fp)
        fn_increase = max(0, local_fn - baseline_fn)
        fp_decrease = max(0, baseline_fp - local_fp)
        fn_decrease = max(0, baseline_fn - local_fn)
        new_fp += fp_increase
        new_fn += fn_increase
        resolved_fp += fp_decrease
        resolved_fn += fn_decrease
    net_fp = new_fp - resolved_fp
    net_fn = new_fn - resolved_fn
    return {
        "new_fp": new_fp, "new_fn": new_fn,
        "resolved_fp": resolved_fp, "resolved_fn": resolved_fn,
        "net_fp": net_fp, "net_fn": net_fn,
        "total_baseline_fp": total_baseline_fp, "total_baseline_fn": total_baseline_fn,
        "total_local_fp": total_local_fp, "total_local_fn": total_local_fn,
    }


def test_gate_preexisting_fn_does_not_regress():
    """Pre-existing FN (already on main) should not be flagged."""
    by_repo_cop = {
        "repo-a": {
            # Oracle: nitrocop=12 (10+2fp), rubocop=13 (10+3fn) → baseline FN=1
            "Style/Foo": {"matches": 10, "fp": 2, "fn": 3},
        },
    }
    # Local produces same as oracle nitrocop → no regression
    g = _compute_gate(by_repo_cop, "Style/Foo", {"repo-a": 12})
    assert g["net_fp"] <= 0 and g["net_fn"] <= 0


def test_gate_improvement_passes():
    """Moving closer to rubocop is not a regression."""
    by_repo_cop = {
        "repo-a": {
            # Oracle: nitrocop=12, rubocop=13 → baseline FN=1
            "Style/Foo": {"matches": 10, "fp": 2, "fn": 3},
        },
    }
    # Local=13 matches rubocop exactly (fixed the FN)
    g = _compute_gate(by_repo_cop, "Style/Foo", {"repo-a": 13})
    assert g["net_fp"] <= 0 and g["net_fn"] <= 0


def test_gate_new_fp_detected():
    """New FP beyond baseline is flagged."""
    by_repo_cop = {
        "repo-a": {
            # Oracle: nitrocop=12, rubocop=13 → baseline FP=0
            "Style/Foo": {"matches": 10, "fp": 2, "fn": 3},
        },
    }
    # Local=15 → 2 more FP than rubocop, baseline had 0 excess over rubocop
    g = _compute_gate(by_repo_cop, "Style/Foo", {"repo-a": 15})
    assert g["new_fp"] == 2
    assert g["net_fp"] == 2  # no resolved FP to offset


def test_gate_new_fn_detected():
    """New FN beyond baseline is flagged."""
    by_repo_cop = {
        "repo-a": {
            # Oracle: nitrocop=12, rubocop=13 → baseline FN=1
            "Style/Foo": {"matches": 10, "fp": 2, "fn": 3},
        },
    }
    # Local=9 → FN=4 vs rubocop, baseline had FN=1, so +3 new FN
    g = _compute_gate(by_repo_cop, "Style/Foo", {"repo-a": 9})
    assert g["new_fn"] == 3
    assert g["net_fn"] == 3  # no resolved FN to offset


def test_gate_exact_match_no_regression():
    """Cop with zero baseline divergence — same count passes."""
    by_repo_cop = {
        "repo-a": {
            "Style/Foo": {"matches": 50, "fp": 0, "fn": 0},
        },
    }
    g = _compute_gate(by_repo_cop, "Style/Foo", {"repo-a": 50})
    assert g["net_fp"] <= 0 and g["net_fn"] <= 0


def test_gate_net_improvement_passes():
    """Per-repo FN regressions offset by improvements elsewhere should pass."""
    by_repo_cop = {
        # repo-a: nitrocop=5, rubocop=10 → baseline FN=5
        "repo-a": {"Style/Foo": {"matches": 5, "fp": 0, "fn": 5}},
        # repo-b: nitrocop=0, rubocop=20 → baseline FN=20
        "repo-b": {"Style/Foo": {"matches": 0, "fp": 0, "fn": 20}},
    }
    per_repo = {
        "repo-a": 3,   # worse: FN=7, was 5 → +2 new FN
        "repo-b": 15,  # better: FN=5, was 20 → resolved 15 FN
    }
    g = _compute_gate(by_repo_cop, "Style/Foo", per_repo)
    assert g["new_fn"] == 2        # repo-a regressed by 2
    assert g["resolved_fn"] == 15  # repo-b improved by 15
    assert g["net_fn"] < 0         # net improvement
    assert g["net_fp"] <= 0


def test_gate_net_regression_fails():
    """Per-repo regressions exceeding improvements should fail."""
    by_repo_cop = {
        # repo-a: nitrocop=10, rubocop=10 → baseline FN=0
        "repo-a": {"Style/Foo": {"matches": 10, "fp": 0, "fn": 0}},
        # repo-b: nitrocop=0, rubocop=3 → baseline FN=3
        "repo-b": {"Style/Foo": {"matches": 0, "fp": 0, "fn": 3}},
    }
    per_repo = {
        "repo-a": 0,  # worse: FN=10, was 0 → +10 new FN
        "repo-b": 3,  # better: FN=0, was 3 → resolved 3 FN
    }
    g = _compute_gate(by_repo_cop, "Style/Foo", per_repo)
    assert g["new_fn"] == 10
    assert g["resolved_fn"] == 3
    assert g["net_fn"] == 7  # net regression


def test_gate_net_zero_passes():
    """Regressions exactly offset by improvements should pass (net=0)."""
    by_repo_cop = {
        "repo-a": {"Style/Foo": {"matches": 10, "fp": 0, "fn": 0}},
        "repo-b": {"Style/Foo": {"matches": 0, "fp": 0, "fn": 5}},
    }
    per_repo = {
        "repo-a": 5,  # worse: FN=5, was 0 → +5 new FN
        "repo-b": 5,  # better: FN=0, was 5 → resolved 5 FN
    }
    g = _compute_gate(by_repo_cop, "Style/Foo", per_repo)
    assert g["new_fn"] == 5
    assert g["resolved_fn"] == 5
    assert g["net_fn"] == 0  # exactly offset — should pass (gate checks > threshold)


def test_gate_fp_net_improvement_passes():
    """Per-repo FP regressions offset by FP improvements should pass."""
    by_repo_cop = {
        # repo-a: nitrocop=15, rubocop=10 → baseline FP=5
        "repo-a": {"Style/Foo": {"matches": 10, "fp": 5, "fn": 0}},
        # repo-b: nitrocop=12, rubocop=10 → baseline FP=2
        "repo-b": {"Style/Foo": {"matches": 10, "fp": 2, "fn": 0}},
    }
    per_repo = {
        "repo-a": 13,  # FP=3, was 5 → resolved 2 FP
        "repo-b": 11,  # FP=1, was 2 → resolved 1 FP (improved)
    }
    g = _compute_gate(by_repo_cop, "Style/Foo", per_repo)
    assert g["new_fp"] == 0
    assert g["resolved_fp"] == 3
    assert g["net_fp"] < 0
    assert g["net_fn"] <= 0


def test_gate_strict_fails_on_any_regression():
    """Without --allow-net-improvement, any per-repo regression fails."""
    by_repo_cop = {
        "repo-a": {"Style/Foo": {"matches": 5, "fp": 0, "fn": 5}},
        "repo-b": {"Style/Foo": {"matches": 0, "fp": 0, "fn": 20}},
    }
    per_repo = {
        "repo-a": 3,   # worse: +2 new FN
        "repo-b": 15,  # better: resolved 15 FN
    }
    g = _compute_gate(by_repo_cop, "Style/Foo", per_repo)
    # Net is an improvement
    assert g["net_fn"] < 0
    # But strict mode (default) uses new_fn, not net_fn
    assert g["new_fn"] == 2  # would FAIL in strict mode (agents)
    # --allow-net-improvement uses net_fn which is negative → PASS in CI


def test_summary_emits_total_local_fp_fn_not_regressions():
    """SUMMARY line should report total local FP/FN, not just regressions."""
    by_repo_cop = {
        # repo-a: oracle nitrocop=1064, rubocop=1061 → baseline FP=3
        "repo-a": {"Metrics/MethodLength": {"matches": 1061, "fp": 3, "fn": 0}},
    }
    # Local produces same as oracle nitrocop → no regression, but FP=3 remains
    g = _compute_gate(by_repo_cop, "Metrics/MethodLength", {"repo-a": 1064})
    assert g["new_fp"] == 0           # no regression
    assert g["total_baseline_fp"] == 3
    assert g["total_local_fp"] == 3   # should report actual FP, not 0
    # SUMMARY should use total_local_fp (3), not new_fp (0)


def test_summary_shows_improvement_correctly():
    """When local FP is lower than baseline, SUMMARY should reflect that."""
    by_repo_cop = {
        "repo-a": {"Style/Foo": {"matches": 10, "fp": 5, "fn": 0}},
    }
    # Local=12 → FP=2, baseline FP=5 → improvement
    g = _compute_gate(by_repo_cop, "Style/Foo", {"repo-a": 12})
    assert g["total_baseline_fp"] == 5
    assert g["total_local_fp"] == 2
    assert g["resolved_fp"] == 3
    assert g["new_fp"] == 0
