#!/usr/bin/env python3
"""Tests for scripts/check_cop_styles.py."""

import importlib.util
from pathlib import Path

SCRIPT = Path(__file__).parents[2] / "scripts" / "check_cop_styles.py"
SPEC = importlib.util.spec_from_file_location("check_cop_styles", SCRIPT)
assert SPEC and SPEC.loader
check_cop_styles = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(check_cop_styles)


def test_get_all_styles_returns_dict():
    """get_all_styles() returns a non-empty dict keyed by cop name."""
    styles = check_cop_styles.get_all_styles()
    assert isinstance(styles, dict)
    assert len(styles) > 0


def test_get_all_styles_values_have_required_keys():
    """Each style entry has key, default, and alternatives."""
    styles = check_cop_styles.get_all_styles()
    for cop, entries in styles.items():
        assert "/" in cop, f"Cop name should contain /: {cop}"
        for entry in entries:
            assert "key" in entry
            assert "default" in entry
            assert "alternatives" in entry
            assert isinstance(entry["alternatives"], list)
            assert len(entry["alternatives"]) > 0


def test_get_all_styles_includes_known_cops():
    """Known configurable cops should appear in the result."""
    styles = check_cop_styles.get_all_styles()
    # These are well-known cops with EnforcedStyle in vendor config
    assert "Style/StringLiterals" in styles
    assert "Layout/DotPosition" in styles


def test_get_all_styles_department_filtering():
    """Can filter by department prefix."""
    styles = check_cop_styles.get_all_styles()
    layout_cops = [c for c in styles if c.startswith("Layout/")]
    style_cops = [c for c in styles if c.startswith("Style/")]
    assert len(layout_cops) > 0
    assert len(style_cops) > 0


def test_check_cop_all_styles_marks_defaults():
    """check_cop_all_styles sets is_default correctly on results."""
    # Use a mock style list to test the logic without subprocess
    styles = [{"key": "EnforcedStyle", "default": "foo", "alternatives": ["bar", "baz"]}]

    # Patch run_check_cop_style to avoid subprocess
    original = check_cop_styles.run_check_cop_style
    calls = []

    def fake_run(cop, param, value, **kwargs):
        calls.append((cop, param, value))
        return {"param": param, "value": value, "passed": True, "output": ""}

    check_cop_styles.run_check_cop_style = fake_run
    try:
        results = check_cop_styles.check_cop_all_styles("Style/Fake", styles)
    finally:
        check_cop_styles.run_check_cop_style = original

    assert len(results) == 3
    assert results[0]["value"] == "foo"
    assert results[0]["is_default"] is True
    assert results[1]["value"] == "bar"
    assert results[1]["is_default"] is False
    assert results[2]["value"] == "baz"
    assert results[2]["is_default"] is False
    assert calls == [
        ("Style/Fake", "EnforcedStyle", "foo"),
        ("Style/Fake", "EnforcedStyle", "bar"),
        ("Style/Fake", "EnforcedStyle", "baz"),
    ]
