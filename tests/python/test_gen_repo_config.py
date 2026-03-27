#!/usr/bin/env python3
"""Tests for bench/corpus/gen_repo_config.py.

Verifies that the overlay config excludes vendor-ish paths correctly.
Uses Ruby's File.fnmatch to test pattern matching — the same engine
RuboCop uses — so these tests catch glob syntax mismatches that
Python's fnmatch would miss.
"""

from __future__ import annotations

import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).parents[2] / "bench" / "corpus" / "gen_repo_config.py"
BASELINE = Path(__file__).parents[2] / "bench" / "corpus" / "baseline_rubocop.yml"


def generate_overlay(repo_id: str, repo_dir: str) -> str:
    """Run gen_repo_config.py and return the overlay file contents."""
    result = subprocess.run(
        [sys.executable, str(SCRIPT), repo_id, str(BASELINE), repo_dir],
        capture_output=True, text=True, check=True,
    )
    overlay_path = result.stdout.strip()
    return Path(overlay_path).read_text()


def extract_exclude_patterns(overlay_content: str) -> list[str]:
    """Extract Exclude patterns from an overlay YAML file."""
    patterns = []
    for line in overlay_content.splitlines():
        line = line.strip()
        if line.startswith('- "') and line.endswith('"'):
            patterns.append(line[3:-1])
    return patterns


def ruby_fnmatch(pattern: str, path: str) -> bool:
    """Test if a path matches a pattern using Ruby's File.fnmatch (same as RuboCop)."""
    result = subprocess.run(
        ["ruby", "-e", f'puts File.fnmatch({pattern!r}, {path!r}, File::FNM_PATHNAME | File::FNM_EXTGLOB)'],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        raise RuntimeError(f"Ruby failed: {result.stderr}")
    return result.stdout.strip() == "true"


def ruby_available() -> bool:
    try:
        r = subprocess.run(["ruby", "-e", "puts 1"], capture_output=True, text=True)
        return r.returncode == 0
    except FileNotFoundError:
        return False


NEEDS_RUBY = not ruby_available()


def test_overlay_always_generated():
    """Every repo gets an overlay (for global vendor excludes), never the bare baseline."""
    with tempfile.TemporaryDirectory() as tmpdir:
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "any_repo_id", str(BASELINE), tmpdir],
            capture_output=True, text=True, check=True,
        )
        overlay_path = result.stdout.strip()
        assert overlay_path != str(BASELINE), "Should return overlay, not baseline"
        assert Path(overlay_path).exists()


def test_overlay_contains_global_vendor_patterns():
    """Overlay includes all global vendor-ish exclude patterns."""
    with tempfile.TemporaryDirectory() as tmpdir:
        content = generate_overlay("test_repo", tmpdir)
        patterns = extract_exclude_patterns(content)
        abs_prefix = str(Path(tmpdir).resolve())

        expected_suffixes = [
            "/vendor/**/*",
            "/vendor*/**/*",
            "/_vendor/**/*",
            "/cookbooks/**/*",
        ]
        for suffix in expected_suffixes:
            expected = abs_prefix + suffix
            assert expected in patterns, f"Missing pattern: {expected}\nGot: {patterns}"


def test_overlay_contains_per_repo_excludes():
    """Repos with entries in repo_excludes.json get additional patterns."""
    with tempfile.TemporaryDirectory() as tmpdir:
        content = generate_overlay("jruby__jruby__0303464", tmpdir)
        patterns = extract_exclude_patterns(content)
        abs_prefix = str(Path(tmpdir).resolve())

        assert f"{abs_prefix}/bench/compiler/bench_compilation.rb" in patterns
        assert f"{abs_prefix}/test/jruby/test_regexp.rb" in patterns


def test_overlay_inherits_from_baseline():
    with tempfile.TemporaryDirectory() as tmpdir:
        content = generate_overlay("test_repo", tmpdir)
        assert "inherit_from:" in content
        assert str(BASELINE.resolve()) in content


class TestRubyGlobMatching:
    """Test that overlay patterns match real corpus FN paths using Ruby's File.fnmatch."""

    # Known FN paths from corpus oracle results that must be excluded
    VENDOR_PATHS = [
        ("opal__opal__07183b3", "vendored-minitest/minitest.rb"),
        ("opal__opal__07183b3", "vendored-minitest/minitest/spec.rb"),
        ("opal__opal__07183b3", "vendored-minitest/minitest/mock.rb"),
        ("teracyhq__dev__2ed1b6a", "vendor-cookbooks/compat_resource/files/lib/chef_compat/resource.rb"),
        ("teracyhq__dev__2ed1b6a", "vendor-cookbooks/docker/libraries/helpers_service.rb"),
        ("jashkenas__ruby-processing__2d83318", "vendors/Rakefile"),
        ("grab__engineering-blog__ba1b627", "_vendor/foo.rb"),
        ("hh__windows-fromscratch__ede01f1", "cookbooks/docker/recipes/default.rb"),
    ]

    # Paths that must NOT be excluded
    NON_VENDOR_PATHS = [
        ("rails__rails__abc", "lib/active_record/base.rb"),
        ("rails__rails__abc", "app/models/user.rb"),
        ("rails__rails__abc", "spec/models/user_spec.rb"),
        ("rails__rails__abc", "Gemfile"),
    ]

    def test_vendor_paths_excluded(self):
        if NEEDS_RUBY:
            import pytest
            pytest.skip("Ruby not available")

        with tempfile.TemporaryDirectory() as tmpdir:
            for repo_id, rel_path in self.VENDOR_PATHS:
                content = generate_overlay(repo_id, tmpdir)
                patterns = extract_exclude_patterns(content)
                abs_path = f"{Path(tmpdir).resolve()}/{rel_path}"

                matched = any(ruby_fnmatch(p, abs_path) for p in patterns)
                assert matched, (
                    f"MISS: {rel_path} in {repo_id} not matched by any pattern.\n"
                    f"Absolute path: {abs_path}\n"
                    f"Patterns: {patterns}"
                )

    def test_non_vendor_paths_not_excluded(self):
        if NEEDS_RUBY:
            import pytest
            pytest.skip("Ruby not available")

        with tempfile.TemporaryDirectory() as tmpdir:
            for repo_id, rel_path in self.NON_VENDOR_PATHS:
                content = generate_overlay(repo_id, tmpdir)
                patterns = extract_exclude_patterns(content)
                abs_path = f"{Path(tmpdir).resolve()}/{rel_path}"

                matched = any(ruby_fnmatch(p, abs_path) for p in patterns)
                assert not matched, (
                    f"FALSE MATCH: {rel_path} in {repo_id} incorrectly matched.\n"
                    f"Absolute path: {abs_path}\n"
                    f"Patterns: {patterns}"
                )
