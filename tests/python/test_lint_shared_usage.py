"""Tests for scripts/lint_shared_usage.py."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path
from unittest import mock

# Make scripts importable.
REPO_ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(REPO_ROOT / "scripts"))

import lint_shared_usage  # noqa: E402


def _lint_text(code: str) -> list[str]:
    """Helper: lint a string as if it were a cop file."""
    tmp = REPO_ROOT / "src" / "cop" / "_test_tmp.rs"
    try:
        tmp.write_text(textwrap.dedent(code), encoding="utf-8")
        return lint_shared_usage.lint_file(tmp)
    finally:
        tmp.unlink(missing_ok=True)


class TestIsCommandPattern:
    def test_catches_receiver_then_name(self):
        code = """\
        fn check(call: &CallNode) {
            if call.receiver().is_none() && call.name().as_slice() == b"puts" {
            }
        }
        """
        violations = _lint_text(code)
        assert len(violations) == 1
        assert "is_command" in violations[0]

    def test_catches_name_then_receiver(self):
        code = """\
        fn check(call: &CallNode) {
            if call.name().as_slice() == b"puts" && call.receiver().is_none() {
            }
        }
        """
        violations = _lint_text(code)
        assert len(violations) == 1
        assert "is_command" in violations[0]

    def test_ignores_test_section(self):
        code = """\
        fn real_code() {}

        #[cfg(test)]
        mod tests {
            fn test_something() {
                if call.receiver().is_none() && call.name().as_slice() == b"puts" {}
            }
        }
        """
        violations = _lint_text(code)
        assert len(violations) == 0

    def test_ignores_comments(self):
        code = """\
        // if call.receiver().is_none() && call.name().as_slice() == b"puts" {}
        """
        violations = _lint_text(code)
        assert len(violations) == 0


class TestOperatorLocPatterns:
    def test_catches_logical_and(self):
        code = """\
        fn check(node: &AndNode) {
            if node.operator_loc().as_slice() == b"&&" {}
        }
        """
        violations = _lint_text(code)
        assert len(violations) == 1
        assert "is_logical_and" in violations[0]

    def test_catches_logical_or(self):
        code = """\
        fn check(node: &OrNode) {
            if node.operator_loc().as_slice() == b"||" {}
        }
        """
        violations = _lint_text(code)
        assert len(violations) == 1
        assert "is_logical_or" in violations[0]

    def test_catches_semantic_and(self):
        code = """\
        fn check(node: &AndNode) {
            if node.operator_loc().as_slice() == b"and" {}
        }
        """
        violations = _lint_text(code)
        assert len(violations) == 1
        assert "is_semantic_and" in violations[0]

    def test_catches_semantic_or(self):
        code = """\
        fn check(node: &OrNode) {
            if node.operator_loc().as_slice() == b"or" {}
        }
        """
        violations = _lint_text(code)
        assert len(violations) == 1
        assert "is_semantic_or" in violations[0]


class TestCallOperatorPatterns:
    def test_catches_safe_navigation(self):
        code = """\
        fn check(call: &CallNode) {
            call.call_operator_loc().is_some_and(|loc| loc.as_slice() == b"&.")
        }
        """
        violations = _lint_text(code)
        assert len(violations) == 1
        assert "is_safe_navigation" in violations[0]

    def test_catches_dot_call(self):
        code = """\
        fn check(call: &CallNode) {
            call.call_operator_loc().is_some_and(|loc| loc.as_slice() == b".")
        }
        """
        violations = _lint_text(code)
        assert len(violations) == 1
        assert "is_dot_call" in violations[0]

    def test_catches_double_colon(self):
        code = """\
        fn check(call: &CallNode) {
            call.call_operator_loc().is_some_and(|op| op.as_slice() == b"::")
        }
        """
        violations = _lint_text(code)
        assert len(violations) == 1
        assert "is_double_colon_call" in violations[0]


class TestCleanCodePasses:
    def test_using_shared_function_passes(self):
        code = """\
        use crate::cop::shared::method_dispatch_predicates;

        fn check(call: &CallNode) {
            if method_dispatch_predicates::is_command(call, b"puts") {}
        }
        """
        violations = _lint_text(code)
        assert len(violations) == 0


class TestMainExitCode:
    def test_returns_zero_on_clean(self):
        assert lint_shared_usage.main() == 0
