#!/usr/bin/env python3
"""Tests for bench/corpus/patch_variant_stats.py."""

import importlib.util
import json
from pathlib import Path

SCRIPT = Path(__file__).parents[2] / "bench" / "corpus" / "patch_variant_stats.py"
SPEC = importlib.util.spec_from_file_location("patch_variant_stats", SCRIPT)
assert SPEC and SPEC.loader
patch_variant_stats = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(patch_variant_stats)


def test_patch_adds_variant_summary(tmp_path):
    corpus = {
        "summary": {"matches": 1000, "fp": 10, "fn": 5},
        "by_department": [
            {"department": "Style", "matches": 800, "fp": 8, "fn": 3},
            {"department": "Layout", "matches": 200, "fp": 2, "fn": 2},
        ],
    }
    variants = tmp_path / "variants.json"
    variants.write_text(json.dumps({
        "batches": [{
            "name": "batch_1",
            "by_cop": [
                {"cop": "Style/Foo", "style_label": "bar", "matches": 50, "fp": 5, "fn": 3},
                {"cop": "Layout/Baz", "style_label": "qux", "matches": 20, "fp": 1, "fn": 2},
            ],
        }]
    }))

    result = patch_variant_stats.patch(corpus, variants)
    assert result is True

    s = corpus["summary"]
    # Overall: default (1000+10+5) + variant (50+5+3 + 20+1+2) = 1096
    assert s["variant_matches"] == 1000 + 50 + 20
    assert s["variant_fp"] == 10 + 5 + 1
    assert s["variant_fn"] == 5 + 3 + 2

    # Department rates should use final default as base
    style = corpus["by_department"][0]
    assert "variant_match_rate" in style
    # Style: default_total=811, variant_extra=58, combined_matches=850, combined_total=869
    assert style["variant_match_rate"] < style["matches"] / (style["matches"] + style["fp"] + style["fn"])

    layout = corpus["by_department"][1]
    assert "variant_match_rate" in layout


def test_patch_variant_rate_always_lte_default(tmp_path):
    """Variant rate must always be <= default rate (adding offenses can only hurt)."""
    corpus = {
        "summary": {"matches": 10000, "fp": 100, "fn": 50},
        "by_department": [
            {"department": "Style", "matches": 10000, "fp": 100, "fn": 50},
        ],
    }
    variants = tmp_path / "variants.json"
    variants.write_text(json.dumps({
        "batches": [{
            "name": "batch_1",
            "by_cop": [
                {"cop": "Style/A", "style_label": "x", "matches": 500, "fp": 200, "fn": 100},
            ],
        }]
    }))

    patch_variant_stats.patch(corpus, variants)

    default_total = 10000 + 100 + 50
    default_rate = 10000 / default_total
    variant_rate = corpus["summary"]["variant_overall_match_rate"]
    assert variant_rate <= default_rate

    dept_variant = corpus["by_department"][0]["variant_match_rate"]
    dept_default = 10000 / default_total
    assert dept_variant <= dept_default


def test_patch_empty_variants(tmp_path):
    corpus = {"summary": {"matches": 100, "fp": 0, "fn": 0}, "by_department": []}
    variants = tmp_path / "variants.json"
    variants.write_text(json.dumps({"batches": [{"name": "b", "by_cop": []}]}))

    result = patch_variant_stats.patch(corpus, variants)
    assert result is False
    assert "variant_overall_match_rate" not in corpus["summary"]
