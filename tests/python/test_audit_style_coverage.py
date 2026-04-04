#!/usr/bin/env python3
"""Tests for scripts/audit_style_coverage.py."""

import importlib.util
from pathlib import Path

SCRIPT = Path(__file__).parents[2] / "scripts" / "audit_style_coverage.py"
SPEC = importlib.util.spec_from_file_location("audit_style_coverage", SCRIPT)
assert SPEC and SPEC.loader
audit_style_coverage = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(audit_style_coverage)


def test_audit_returns_list():
    """audit() returns a list of dicts with expected keys."""
    results = audit_style_coverage.audit()
    assert isinstance(results, list)
    if results:
        r = results[0]
        assert "cop" in r
        assert "key" in r
        assert "style" in r
        assert "tested" in r
        assert isinstance(r["tested"], bool)


def test_audit_has_known_cop():
    """Audit should find Style/TrailingCommaInHashLiteral (a known configurable cop)."""
    results = audit_style_coverage.audit()
    cops = {r["cop"] for r in results}
    # At least some well-known configurable cops should appear
    assert len(cops) > 0
    # These are in the vendor config and have alternatives
    known_configurable = {"Style/StringLiterals", "Layout/DotPosition", "Style/AndOr"}
    assert known_configurable & cops, f"Expected some of {known_configurable} in {cops}"
