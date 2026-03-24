#!/usr/bin/env python3
"""Tests for precompute_repair_cop_check.py."""

from __future__ import annotations

import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[3] / "scripts" / "workflows"))
import precompute_repair_cop_check


def test_render_packet_includes_changed_cop_results():
    packet = precompute_repair_cop_check.render_packet(
        [
            {
                "cop": "Style/MixinUsage",
                "command": "python3 scripts/check_cop.py Style/MixinUsage --verbose --rerun --quick --clone",
                "status": 1,
                "output": (
                    "  (used batch --corpus-check mode)\n"
                    "Repos with offenses (2):\n"
                    "      35  travis-ci__dpl__8c6eabc\n"
                    "      29  puppetlabs__puppet__e227c27\n\n"
                    "FAIL: FN increased from 0 to 38"
                ),
            }
        ],
        standard_corpus=Path("/tmp/standard.json"),
        corpus_dir=Path("/repo/vendor/corpus"),
        oracle_by_cop={
            "Style/MixinUsage": {
                "fn_examples": [
                    {
                        "loc": "puppetlabs__puppet__e227c27: manifests/init.rb:30",
                        "msg": "include in BEGIN should still count",
                        "src": ["  29: BEGIN {", ">>30:   include UtilityFunctions", "  31: }"],
                    }
                ],
                "fp_examples": [
                    {
                        "loc": "travis-ci__dpl__8c6eabc: lib/foo.rb:10",
                        "msg": "false positive around wrapper",
                        "src": ["   9: module X", ">>10: include Something", "  11: end"],
                    }
                ],
            }
        },
        oracle_repo_breakdown={
            "Style/MixinUsage": {
                "puppetlabs__puppet__e227c27": {"fp": 0, "fn": 4},
                "travis-ci__dpl__8c6eabc": {"fp": 3, "fn": 0},
            }
        },
    )
    assert "## Local Cop-Check Diagnosis" in packet
    assert "`Style/MixinUsage`" in packet
    assert "Exit status: `1`" in packet
    assert "FN increased from 0 to 38" in packet
    assert "Start here:" in packet
    assert "python3 scripts/investigate_cop.py Style/MixinUsage --input /tmp/standard.json --repos-only" in packet
    assert "python3 scripts/investigate_cop.py Style/MixinUsage --input /tmp/standard.json --fn-only --context --limit 10" in packet
    assert "python3 scripts/check_cop.py Style/MixinUsage --verbose --rerun --quick --clone --no-batch" in packet
    assert "Oracle FN hotspots:" in packet
    assert "`puppetlabs__puppet__e227c27` (4 FN)" in packet
    assert "Representative oracle FN examples:" in packet
    assert "Representative oracle FP examples:" in packet
    assert "include in BEGIN should still count" in packet
    assert "/repo/vendor/corpus/travis-ci__dpl__8c6eabc" in packet
    assert "/repo/vendor/corpus/puppetlabs__puppet__e227c27" in packet


def test_render_packet_handles_no_changed_cops():
    packet = precompute_repair_cop_check.render_packet([])
    assert "No changed cops were detected" in packet


def test_tail_lines_truncates_to_suffix():
    text = "\n".join(f"line {idx}" for idx in range(300))
    trimmed = precompute_repair_cop_check.tail_lines(text, max_lines=3)
    assert "showing last 3 of 300 lines" in trimmed
    assert "line 297" in trimmed
    assert "line 299" in trimmed
    assert "line 0" not in trimmed


def test_extract_top_repo_ids_reads_repo_block():
    output = (
        "something\n"
        "Repos with offenses (3):\n"
        "      35  travis-ci__dpl__8c6eabc\n"
        "      29  puppetlabs__puppet__e227c27\n"
        "      21  seyhunak__twitter-bootstrap-rails__de5f917\n\n"
        "Results:\n"
    )
    assert precompute_repair_cop_check.extract_top_repo_ids(output, limit=2) == [
        "travis-ci__dpl__8c6eabc",
        "puppetlabs__puppet__e227c27",
    ]


def test_used_batch_mode_detects_batch_marker():
    assert precompute_repair_cop_check.used_batch_mode("  (used batch --corpus-check mode)\n")
    assert not precompute_repair_cop_check.used_batch_mode("Repos with offenses (2):\n")


def test_load_oracle_context_reconstructs_by_cop_and_repo_breakdown():
    with tempfile.TemporaryDirectory() as tmp:
        corpus = Path(tmp) / "corpus-results.json"
        corpus.write_text(
            """
            {
              "by_cop": [
                {"cop": "Style/MixinUsage", "fp_examples": [], "fn_examples": []}
              ],
              "by_repo_cop": {
                "repo-a": {"Style/MixinUsage": {"fp": 1, "fn": 2}}
              }
            }
            """
        )
        by_cop, repo_breakdown = precompute_repair_cop_check.load_oracle_context(corpus)
        assert "Style/MixinUsage" in by_cop
        assert repo_breakdown["Style/MixinUsage"]["repo-a"] == {"fp": 1, "fn": 2}


if __name__ == "__main__":
    test_render_packet_includes_changed_cop_results()
    test_render_packet_handles_no_changed_cops()
    test_tail_lines_truncates_to_suffix()
    test_extract_top_repo_ids_reads_repo_block()
    test_used_batch_mode_detects_batch_marker()
    test_load_oracle_context_reconstructs_by_cop_and_repo_breakdown()
    print("All tests passed.")
