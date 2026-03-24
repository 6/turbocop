#!/usr/bin/env python3
"""Tests for reduce_mismatch.py."""

import importlib.util
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import Mock, patch

SCRIPT = Path(__file__).parents[2] / "scripts" / "reduce_mismatch.py"

spec = importlib.util.spec_from_file_location("reduce_mismatch", SCRIPT)
reduce_mismatch = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(reduce_mismatch)


def setup_function(_function=None):
    reduce_mismatch._predicate_calls = 0
    reduce_mismatch._predicate_cache.clear()


def test_rubocop_runner_uses_no_server():
    """RubocopRunner should use --no-server to avoid contention."""
    runner = reduce_mismatch.RubocopRunner()
    cmd = runner._base_cmd("Style/Test", "/tmp/example.rb")
    assert "--no-server" in cmd
    assert "--server" not in cmd or cmd.index("--no-server") < len(cmd)
    assert "--start-server" not in cmd


def test_rubocop_runner_filters_by_cop():
    """Only offense lines matching the requested cop should be returned."""
    fake_data = {
        "files": [{
            "offenses": [
                {"cop_name": "Style/Test", "location": {"line": 3}},
                {"cop_name": "Style/Other", "location": {"line": 5}},
                {"cop_name": "Style/Test", "location": {"line": 10}},
            ]
        }]
    }

    with patch.object(reduce_mismatch, "cached_rubocop_run", return_value=fake_data):
        runner = reduce_mismatch.RubocopRunner()
        lines = runner.run("Style/Test", "/tmp/example.rb")

    assert lines == {3, 10}


def test_is_interesting_caches_repeated_candidates():
    """Repeated candidate text should not rerun the expensive predicate."""
    runner = SimpleNamespace(run=Mock(return_value=set()))

    with patch.object(reduce_mismatch, "run_nitrocop", return_value={3}) as nitrocop:
        with patch.object(reduce_mismatch, "is_parseable", return_value=True) as parseable:
            first = reduce_mismatch.is_interesting(
                "Style/Test",
                "/tmp/example.rb",
                "fp",
                runner,
                skip_rubocop=True,
                candidate_text="value\n",
            )
            second = reduce_mismatch.is_interesting(
                "Style/Test",
                "/tmp/example.rb",
                "fp",
                runner,
                skip_rubocop=True,
                candidate_text="value\n",
            )

    assert first is True
    assert second is True
    assert reduce_mismatch._predicate_calls == 1
    nitrocop.assert_called_once()
    parseable.assert_called_once()
    runner.run.assert_not_called()


def test_fp_short_circuits_before_parse_when_nitrocop_is_silent():
    """FP candidates rejected by nitrocop should not pay the parseability check."""
    runner = SimpleNamespace(run=Mock(return_value=set()))

    with patch.object(reduce_mismatch, "run_nitrocop", return_value=set()):
        with patch.object(
            reduce_mismatch,
            "is_parseable",
            side_effect=AssertionError("parseability should not be checked"),
        ):
            interesting = reduce_mismatch.is_interesting(
                "Style/Test",
                "/tmp/example.rb",
                "fp",
                runner,
                skip_rubocop=True,
                candidate_text="value\n",
            )

    assert interesting is False
    runner.run.assert_not_called()


def test_fn_short_circuits_before_parse_when_rubocop_is_silent():
    """FN candidates rejected by RuboCop should not pay the parseability check."""
    runner = SimpleNamespace(run=Mock(return_value=set()))

    with patch.object(reduce_mismatch, "run_nitrocop", return_value=set()):
        with patch.object(
            reduce_mismatch,
            "is_parseable",
            side_effect=AssertionError("parseability should not be checked"),
        ):
            interesting = reduce_mismatch.is_interesting(
                "Style/Test",
                "/tmp/example.rb",
                "fn",
                runner,
                candidate_text="value\n",
            )

    assert interesting is False
    runner.run.assert_called_once_with("Style/Test", "/tmp/example.rb")
