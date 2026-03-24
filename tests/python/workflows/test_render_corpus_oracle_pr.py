#!/usr/bin/env python3
"""Tests for render_corpus_oracle_pr.py."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[3] / "scripts" / "workflows"))
import render_corpus_oracle_pr as renderer


def test_build_metadata_for_standard_full_refresh():
    metadata = renderer.build_metadata(
        corpus_size="standard",
        repo_filter="all",
        run_number="147",
        run_url="https://github.com/6/nitrocop/actions/runs/123",
        changed_files=[
            "README.md",
            "docs/corpus.md",
            "src/resources/tiers.json",
        ],
    )

    expected = "[skip ci] Corpus oracle (standard): refresh tiers.json, README, and standard corpus report"
    assert metadata["commit_message"] == expected
    assert metadata["pr_title"] == expected
    assert "Automated corpus oracle refresh from [run #147]" in metadata["pr_body"]
    assert "| **Corpus** | `standard` |" in metadata["pr_body"]
    assert "| **Scope** | `all repos` |" in metadata["pr_body"]
    assert "| **CI** | intentionally skipped via `[skip ci]` |" in metadata["pr_body"]
    assert "- `src/resources/tiers.json`: Stable and preview tiers regenerated" in metadata["pr_body"]
    assert "- `README.md`: Top-level conformance summary refreshed" in metadata["pr_body"]
    assert "- `docs/corpus.md`: Full standard corpus report regenerated." in metadata["pr_body"]


def test_build_metadata_for_extended_partial_refresh():
    metadata = renderer.build_metadata(
        corpus_size="extended",
        repo_filter="rails__rails__abc123",
        run_number="148",
        run_url="https://github.com/6/nitrocop/actions/runs/456",
        changed_files=["docs/corpus_extended.md"],
    )

    expected = "[skip ci] Corpus oracle (extended): refresh extended corpus report"
    assert metadata["commit_message"] == expected
    assert metadata["pr_title"] == expected
    assert "| **Corpus** | `extended` |" in metadata["pr_body"]
    assert "| **Scope** | `rails__rails__abc123` |" in metadata["pr_body"]
    assert "| **Changed artifacts** | `extended corpus report` |" in metadata["pr_body"]
    assert "- `docs/corpus_extended.md`: Full extended corpus report regenerated." in metadata["pr_body"]
    assert "cop coverage report" not in metadata["pr_body"]


if __name__ == "__main__":
    test_build_metadata_for_standard_full_refresh()
    test_build_metadata_for_extended_partial_refresh()
    print("All tests passed.")
