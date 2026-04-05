#!/usr/bin/env python3
"""Tests for bench/corpus/gen_variant_batches.py."""

import importlib.util
import os
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


def test_inherit_from_uses_absolute_path(tmp_path):
    """inherit_from must use an absolute path so it works from any output dir."""
    out = tmp_path / "batches"
    gen_variant_batches.generate_batches(out)
    content = (out / "variant_batch_1.yml").read_text()
    assert "inherit_from: " in content
    # Extract the path and verify it's absolute and points to baseline_rubocop.yml
    import re
    m = re.search(r"inherit_from: (.+)", content)
    assert m is not None
    path = m.group(1)
    assert os.path.isabs(path), f"inherit_from should be absolute, got: {path}"
    assert path.endswith("baseline_rubocop.yml")


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


def test_max_3_batches(tmp_path):
    """Generation produces at most 3 batches (MAX_BATCHES)."""
    out = tmp_path / "batches"
    n = gen_variant_batches.generate_batches(out)
    assert n <= 3
    files = sorted(out.glob("variant_batch_*.yml"))
    assert len(files) <= 3


def test_overflow_keeps_first_alternative(tmp_path):
    """Cops with 4+ alternatives keep the first overflow (alt[2]) in batch 3.

    Specifically:
      - EmptyLinesAroundClassBody: keeps empty_lines_special, not beginning_only/ending_only
      - EndlessMethod: keeps require_single_line, not require_always
      - HashSyntax shorthand: keeps consistent, not either_consistent
    """
    out = tmp_path / "batches"
    n = gen_variant_batches.generate_batches(out)
    assert n == 3

    batch_3 = (out / "variant_batch_3.yml").read_text()

    # Kept styles should be present
    assert "empty_lines_special" in batch_3
    assert "require_single_line" in batch_3
    # HashSyntax.EnforcedShorthandSyntax: consistent — check it's there
    # but "consistent" appears in comments too, so check the value line
    assert "EnforcedShorthandSyntax: consistent" in batch_3

    # Dropped styles must NOT appear
    assert "beginning_only" not in batch_3
    assert "ending_only" not in batch_3
    assert "require_always" not in batch_3
    assert "either_consistent" not in batch_3


def test_all_variant_values_are_valid(tmp_path):
    """Every style value in every generated batch must be in that cop's SupportedStyles.

    This catches the MagicCommentFormat bug where parse_enforced_styles confused
    SupportedCapitalizations with SupportedStyles, producing variant configs that
    RuboCop rejects with 'invalid EnforcedStyle'.
    """
    import yaml

    out = tmp_path / "batches"
    gen_variant_batches.generate_batches(out)

    # Build a lookup of valid values per (cop, key) from vendor configs
    PROJECT_ROOT = Path(__file__).parents[2]
    valid_values: dict[tuple[str, str], set[str]] = {}

    # Vendor YAMLs use !ruby/regexp tags — add a constructor that ignores them
    class _PermissiveLoader(yaml.SafeLoader):
        pass

    _PermissiveLoader.add_multi_constructor(
        "!", lambda loader, suffix, node: loader.construct_scalar(node)
    )

    for config_path, _plugin in gen_variant_batches.VENDOR_CONFIGS:
        full_path = PROJECT_ROOT / config_path
        if not full_path.exists():
            continue
        data = yaml.load(full_path.read_text(), Loader=_PermissiveLoader) or {}
        for cop_name, cop_cfg in data.items():
            if not isinstance(cop_cfg, dict) or "/" not in str(cop_name):
                continue
            for k, v in cop_cfg.items():
                if k.startswith("Enforced") and isinstance(v, str):
                    # Find the corresponding Supported* list
                    for sk, sv in cop_cfg.items():
                        if sk.startswith("Supported") and isinstance(sv, list):
                            # Match using same normalization as parse_enforced_styles
                            enforced_core = k.replace("Enforced", "").replace("Styles", "Style")
                            supported_core = sk.replace("Supported", "").replace("Styles", "Style")
                            if enforced_core == supported_core:
                                valid_values[(cop_name, k)] = {str(x) for x in sv}

    # Validate every value in every generated batch
    errors = []
    for batch_path in sorted(out.glob("variant_batch_*.yml")):
        data = yaml.safe_load(batch_path.read_text()) or {}
        for cop_name, cop_cfg in data.items():
            if cop_name == "inherit_from" or not isinstance(cop_cfg, dict):
                continue
            for key, value in cop_cfg.items():
                if not key.startswith("Enforced"):
                    continue
                allowed = valid_values.get((cop_name, key))
                if allowed and str(value) not in allowed:
                    errors.append(
                        f"{batch_path.name}: {cop_name}.{key}={value} "
                        f"not in {sorted(allowed)}"
                    )

    assert not errors, "Invalid variant config values:\n" + "\n".join(errors)
