#!/usr/bin/env python3
"""Tests for corpus_stress.py."""

import importlib.util
from pathlib import Path

SCRIPT = Path(__file__).parents[2] / "scripts" / "corpus_stress.py"
SPEC = importlib.util.spec_from_file_location("corpus_stress", SCRIPT)
assert SPEC and SPEC.loader
corpus_stress = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(corpus_stress)


def test_find_stress_failures_only_flags_baseline_perfect_cops():
    corpus = {
        "Style/Foo": {"matches": 10, "fp": 0, "fn": 0},
        "Style/Bar": {"matches": 20, "fp": 1, "fn": 0},
    }
    stress = {
        "Style/Foo": {"matches": 10, "fp": 2, "fn": 1},
        "Style/Bar": {"matches": 20, "fp": 5, "fn": 5},
    }
    failures = corpus_stress.find_stress_failures(corpus, stress)
    assert failures == [{
        "cop": "Style/Foo",
        "stress_fp": 2,
        "stress_fn": 1,
        "stress_matches": 10,
    }]


def test_patch_summary_table_inserts_stress_row():
    original = "\n".join([
        "# Corpus Oracle Results",
        "",
        "| Metric | Value |",
        "|--------|------:|",
        "| Repos | 1000 |",
        "",
    ])
    patched = corpus_stress.patch_summary_table(original, [{"cop": "Style/Foo"}], 25)
    assert "| Stress failures (flipped styles, 25 repos) | 1 |" in patched


def test_append_stress_section_mentions_top_cops():
    patched = corpus_stress.append_stress_section(
        "# Report\n",
        [{"cop": "Style/Foo", "stress_fp": 3, "stress_fn": 1}],
        12,
    )
    assert "## Stress Test (Flipped EnforcedStyles)" in patched
    assert "Style/Foo" in patched
    assert "12 repos tested" in patched


if __name__ == "__main__":
    test_find_stress_failures_only_flags_baseline_perfect_cops()
    test_patch_summary_table_inserts_stress_row()
    test_append_stress_section_mentions_top_cops()
    print("All tests passed.")
