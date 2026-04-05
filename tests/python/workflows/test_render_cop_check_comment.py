#!/usr/bin/env python3
"""Tests for render_cop_check_comment.py."""

import importlib.util
from pathlib import Path

SCRIPT = Path(__file__).parents[3] / "scripts" / "workflows" / "render_cop_check_comment.py"
SPEC = importlib.util.spec_from_file_location("render_cop_check_comment", SCRIPT)
assert SPEC and SPEC.loader
mod = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(mod)


def test_is_variant_row():
    assert mod.is_variant_row("Style/Foo (comma)") is True
    assert mod.is_variant_row("Style/Foo") is False
    assert mod.is_variant_row("Naming/Bar (camelCase)") is True


def test_aggregate_rows_sums_variant_across_shards():
    """Variant rows from different shards are summed into one."""
    rows = [
        # Default rows — kept as-is
        {"cop": "Style/Foo", "bl_fp": 3, "bl_fn": 0, "local_fp": 2, "local_fn": 0, "result": "pass"},
        {"cop": "Style/Foo", "bl_fp": 3, "bl_fn": 0, "local_fp": 1, "local_fn": 0, "result": "pass"},
        # Variant rows — should be aggregated
        {"cop": "Style/Foo (comma)", "bl_fp": 10, "bl_fn": 5, "local_fp": 100, "local_fn": 20, "result": "pass"},
        {"cop": "Style/Foo (comma)", "bl_fp": 10, "bl_fn": 5, "local_fp": 200, "local_fn": 30, "result": "pass"},
        {"cop": "Style/Foo (comma)", "bl_fp": 10, "bl_fn": 5, "local_fp": 50, "local_fn": 10, "result": "pass"},
    ]
    result = mod.aggregate_rows(rows)

    # Default rows kept individually (2)
    default = [r for r in result if not mod.is_variant_row(r["cop"])]
    assert len(default) == 2

    # Variant rows aggregated into 1
    variant = [r for r in result if mod.is_variant_row(r["cop"])]
    assert len(variant) == 1
    assert variant[0]["local_fp"] == 350  # 100 + 200 + 50
    assert variant[0]["local_fn"] == 60   # 20 + 30 + 10
    assert variant[0]["bl_fp"] == 10      # baseline stays as-is (global)


def test_aggregate_rows_variant_regression_detected():
    """Aggregated variant with more FP than baseline is marked as regression."""
    rows = [
        {"cop": "Naming/Foo (bar)", "bl_fp": 6, "bl_fn": 0, "local_fp": 5000, "local_fn": 0, "result": "pass"},
        {"cop": "Naming/Foo (bar)", "bl_fp": 6, "bl_fn": 0, "local_fp": 3000, "local_fn": 0, "result": "pass"},
    ]
    result = mod.aggregate_rows(rows)
    variant = [r for r in result if mod.is_variant_row(r["cop"])]
    assert len(variant) == 1
    assert variant[0]["local_fp"] == 8000
    assert variant[0]["result"] == "regression"


def test_aggregate_rows_variant_improvement_passes():
    """Aggregated variant with fewer FP than baseline passes."""
    rows = [
        {"cop": "Style/Foo (comma)", "bl_fp": 100, "bl_fn": 50, "local_fp": 30, "local_fn": 10, "result": "pass"},
        {"cop": "Style/Foo (comma)", "bl_fp": 100, "bl_fn": 50, "local_fp": 20, "local_fn": 5, "result": "pass"},
    ]
    result = mod.aggregate_rows(rows)
    variant = [r for r in result if mod.is_variant_row(r["cop"])]
    assert variant[0]["local_fp"] == 50  # 30 + 20
    assert variant[0]["result"] == "pass"  # 50 <= 100 baseline


def test_render_comment_shows_regression():
    rows = [
        {"cop": "Style/Foo", "bl_fp": 0, "bl_fn": 0, "local_fp": 0, "local_fn": 0, "result": "pass"},
        {"cop": "Style/Foo (comma)", "bl_fp": 10, "bl_fn": 5, "local_fp": 50, "local_fn": 5, "result": "regression",
         "count_bl_fp": None, "count_bl_fn": None},
    ]
    comment = mod.render_comment(rows, "pass")
    assert "| `Style/Foo (comma)` |" in comment
    assert "+40" in comment  # FP delta: 50 - 10
    assert "❌" in comment  # regression row


def test_parse_summary_lines(tmp_path):
    f = tmp_path / "shard-0.txt"
    f.write_text("Style/Foo|3|0|2|0|pass|3|0\nStyle/Foo (bar)|10|5|100|20|pass|0|0\n")
    rows = mod.parse_summary_lines(tmp_path)
    assert len(rows) == 2
    assert rows[0]["cop"] == "Style/Foo"
    assert rows[1]["cop"] == "Style/Foo (bar)"
    assert rows[1]["local_fp"] == 100
