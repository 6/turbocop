#!/usr/bin/env python3
"""Tests for bench/corpus/gen_variant_batches.py."""

import importlib.util
from pathlib import Path

SCRIPT = Path(__file__).parents[2] / "bench" / "corpus" / "gen_variant_batches.py"
SPEC = importlib.util.spec_from_file_location("gen_variant_batches", SCRIPT)
assert SPEC and SPEC.loader
gen_variant_batches = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gen_variant_batches)


def test_generate_batches_creates_files(tmp_path):
    """generate_batches writes variant_batch_N.yml files."""
    out = tmp_path / "batches"
    n = gen_variant_batches.generate_batches(out)
    assert n >= 1
    batch_1 = out / "variant_batch_1.yml"
    assert batch_1.exists()
    content = batch_1.read_text()
    assert "inherit_from:" in content
    # Should contain at least one cop override
    assert "/" in content  # cop names contain /


def test_generate_batches_dry_run(tmp_path, capsys):
    """--dry-run previews without writing."""
    out = tmp_path / "batches"
    n = gen_variant_batches.generate_batches(out, dry_run=True)
    assert n >= 1
    assert not out.exists()


def test_batch_1_has_most_cops(tmp_path):
    """Batch 1 should cover all configurable cops (every cop has >= 1 alt)."""
    out = tmp_path / "batches"
    n = gen_variant_batches.generate_batches(out)
    files = sorted(out.glob("variant_batch_*.yml"))
    assert len(files) >= 1
    # Batch 1 should have the most cop overrides
    batch_1_lines = (out / "variant_batch_1.yml").read_text().splitlines()
    cop_lines_1 = [x for x in batch_1_lines if "/" in x and not x.startswith("#") and not x.startswith(" ")]
    if n > 1:
        batch_2_lines = (out / "variant_batch_2.yml").read_text().splitlines()
        cop_lines_2 = [x for x in batch_2_lines if "/" in x and not x.startswith("#") and not x.startswith(" ")]
        assert len(cop_lines_1) >= len(cop_lines_2)
