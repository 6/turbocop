#!/usr/bin/env python3
"""Tests for dispatch-cops.py helper functions."""
import importlib.util
import io
import json
from contextlib import redirect_stdout
from pathlib import Path
from types import SimpleNamespace

SCRIPT = Path(__file__).parents[2] / "scripts" / "dispatch-cops.py"
SPEC = importlib.util.spec_from_file_location("dispatch_cops", SCRIPT)
assert SPEC and SPEC.loader
gct = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gct)


def test_pascal_to_snake():
    assert gct.pascal_to_snake("NegatedWhile") == "negated_while"
    assert gct.pascal_to_snake("AmbiguousRange") == "ambiguous_range"
    assert gct.pascal_to_snake("HashLikeCase") == "hash_like_case"
    assert gct.pascal_to_snake("I18nLocaleTexts") == "i18n_locale_texts"
    assert gct.pascal_to_snake("HTTPClient") == "http_client"


def test_parse_cop_name():
    dept, name, snake = gct.parse_cop_name("Style/NegatedWhile")
    assert dept == "Style"
    assert name == "NegatedWhile"
    assert snake == "negated_while"


def test_dept_dir_name():
    assert gct.dept_dir_name("Style") == "style"
    assert gct.dept_dir_name("RSpec") == "rspec"
    assert gct.dept_dir_name("RSpecRails") == "rspec_rails"
    assert gct.dept_dir_name("FactoryBot") == "factory_bot"
    assert gct.dept_dir_name("Lint") == "lint"


def test_extract_source_lines():
    src = [
        "  6: def foo",
        ">>>  7: \tinclude Bar",
        "  8: end",
    ]
    lines, offense, idx = gct._extract_source_lines(src)
    assert len(lines) == 3
    assert "include Bar" in offense
    assert idx == 1


def test_extract_source_lines_no_offense():
    src = ["  1: x = 1", "  2: y = 2"]
    lines, offense, idx = gct._extract_source_lines(src)
    assert len(lines) == 2
    assert offense is None
    assert idx is None


def test_find_enclosing_structure_begin():
    lines = [
        "BEGIN {",
        "\tinclude Foo",
        "}",
    ]
    result = gct._find_enclosing_structure(lines, 1)
    assert result is not None
    assert "BEGIN" in result
    assert "PreExecutionNode" in result


def test_find_enclosing_structure_class():
    lines = [
        "class MyClass",
        "  def foo",
        "    bar",
        "  end",
        "end",
    ]
    result = gct._find_enclosing_structure(lines, 2)
    assert result is not None
    assert "method body" in result


def test_find_enclosing_structure_none():
    lines = ["x = 1"]
    result = gct._find_enclosing_structure(lines, 0)
    assert result is None


def test_find_enclosing_structure_top_level():
    lines = [
        "include Foo",
    ]
    result = gct._find_enclosing_structure(lines, 0)
    assert result is None


def test_extract_spec_excerpts():
    spec = '''
    it 'flags bad code' do
      expect_offense(<<~RUBY)
        x = 1
        ^^^^^ Lint/Foo: Bad.
      RUBY
    end

    it 'accepts good code' do
      expect_no_offenses(<<~RUBY)
        y = 2
      RUBY
    end
    '''
    result = gct.extract_spec_excerpts(spec)
    assert "expect_offense" in result
    assert "expect_no_offenses" in result


def test_extract_spec_excerpts_empty():
    result = gct.extract_spec_excerpts("# no specs here")
    assert result == "(no expect_offense blocks found)"


def test_detect_prism_pitfalls():
    source_with_hash = "if let Some(h) = node.as_hash_node() {"
    pitfalls = gct.detect_prism_pitfalls(source_with_hash)
    assert len(pitfalls) == 1
    assert "KeywordHashNode" in pitfalls[0]


def test_detect_prism_pitfalls_none():
    source = "fn check_node(&self) { }"
    pitfalls = gct.detect_prism_pitfalls(source)
    assert len(pitfalls) == 0


def test_format_with_diagnostics_omits_no_source_examples_when_diagnosed_exists():
    diagnostics = [
        {
            "kind": "fp",
            "loc": "repo: file.rb:1",
            "msg": "Bad spacing",
            "diagnosed": True,
            "detected": True,
            "offense_line": "%w[ a ]",
            "test_snippet": "%w[ a ]\n^ Layout/Foo: Bad spacing",
            "enclosing": None,
            "node_type": None,
            "source_context": "%w[ a ]",
        },
        {
            "kind": "fp",
            "loc": "repo: file.rb:2",
            "msg": "Bad spacing",
            "diagnosed": False,
            "reason": "no source context",
        },
    ]
    output = gct._format_with_diagnostics(
        "Layout/Foo",
        diagnostics,
        fp_examples=[
            {"loc": "repo: file.rb:1", "msg": "Bad spacing", "src": [">>> 1: %w[ a ]"]},
            {"loc": "repo: file.rb:2", "msg": "Bad spacing"},
        ],
        fn_examples=[],
    )
    assert "Omitted 1 pre-diagnostic FP example(s) with no source context" in output
    assert "(could not diagnose: no source context)" not in output
    assert "### Additional examples (not pre-diagnosed)" not in output


def test_format_with_diagnostics_keeps_no_source_examples_when_they_are_all_we_have():
    diagnostics = [
        {
            "kind": "fp",
            "loc": "repo: file.rb:2",
            "msg": "Bad spacing",
            "diagnosed": False,
            "reason": "no source context",
        },
    ]
    output = gct._format_with_diagnostics(
        "Layout/Foo",
        diagnostics,
        fp_examples=[{"loc": "repo: file.rb:2", "msg": "Bad spacing"}],
        fn_examples=[],
    )
    assert "(could not diagnose: no source context)" in output


def test_select_backend_for_entry_retry_uses_claude():
    result = gct.select_backend_for_entry(
        "Style/Foo",
        {"cop": "Style/Foo", "fp": 1, "fn": 1, "matches": 100},
        mode="retry",
        binary=None,
        prior_prs=[],
    )
    assert result["backend"] == "claude-oauth-hard"
    assert "retry" in result["reason"]


def test_has_failed_attempt_ignores_open_prs():
    prs = [
        {"state": "OPEN", "mergedAt": None},
        {"state": "MERGED", "mergedAt": "2026-03-22T00:00:00Z"},
        {"state": "CLOSED", "mergedAt": None},
    ]
    assert gct.has_failed_attempt(prs) is True
    assert gct.has_failed_attempt(prs[:2]) is False


def test_select_backend_for_entry_uses_issue_difficulty_when_present():
    result = gct.select_backend_for_entry(
        "Style/Foo",
        {"cop": "Style/Foo", "fp": 20, "fn": 0, "matches": 20},
        mode="fix",
        binary=None,
        prior_prs=[],
        issue_difficulty="simple",
    )
    assert result["backend"] == "codex-normal"
    assert "issue difficulty label is simple" in result["reason"]


def test_select_backend_for_entry_easy_cop_uses_codex_normal():
    original = gct.diagnose_examples
    gct.diagnose_examples = lambda *args, **kwargs: (1, 0)
    try:
        result = gct.select_backend_for_entry(
            "Style/Foo",
            {"cop": "Style/Foo", "fp": 2, "fn": 1, "matches": 120},
            mode="fix",
            binary=Path(__file__),
            prior_prs=[],
        )
    finally:
        gct.diagnose_examples = original
    assert result["backend"] == "codex-normal"
    assert result["easy"] is True
    assert result["code_bugs"] == 2


def test_assess_cross_corpus_risk_flags_extended_only_edge_case():
    risk = gct.assess_cross_corpus_risk(
        {
            "fp": 3,
            "fn": 0,
            "matches": 100,
            "repo_breakdown": {"foo__bar__abc": {"fp": 3, "fn": 0}},
        },
        extended=True,
        standard_corpus={"fp": 0, "fn": 0, "matches": 540, "repo_breakdown": {}},
    )
    assert risk["extended_only_edge_case"] is True
    assert risk["requires_standard_quick_gate"] is True
    assert risk["risk_class"] == "extended_only_edge_case"


def test_build_cross_corpus_risk_section_warns_about_standard_perfect_baseline():
    section = gct.build_cross_corpus_risk_section(
        {"fp": 3, "fn": 0, "matches": 100, "repo_breakdown": {"foo__bar__abc": {"fp": 3, "fn": 0}}},
        extended=True,
        standard_corpus={"fp": 0, "fn": 0, "matches": 540, "repo_breakdown": {}},
        risk={
            "extended_only_edge_case": True,
            "requires_standard_quick_gate": True,
            "risk_class": "extended_only_edge_case",
            "affected_repos": 1,
            "concentrated": True,
        },
    )
    assert "## Cross-Corpus Risk" in section
    assert "Standard corpus: 540 matches, 0 FP, 0 FN" in section
    assert "extended-only edge case against a standard-perfect baseline" in section
    assert "risk of overfitting" in section


def test_select_backend_for_entry_uses_claude_for_extended_only_edge_case():
    result = gct.select_backend_for_entry(
        "Style/MixinUsage",
        {"cop": "Style/MixinUsage", "fp": 3, "fn": 0, "matches": 100, "repo_breakdown": {"foo__bar__abc": {"fp": 3, "fn": 0}}},
        mode="fix",
        binary=Path(__file__),
        extended=True,
        standard_entry={"cop": "Style/MixinUsage", "fp": 0, "fn": 0, "matches": 540, "repo_breakdown": {}},
        prior_prs=[],
        issue_difficulty="simple",
    )
    assert result["backend"] == "claude-oauth-hard"
    assert result["extended_only_edge_case"] is True
    assert "judgment" in result["reason"] or "document" in result["reason"]


def test_classify_issue_difficulty_escalates_extended_only_edge_case():
    difficulty = gct.classify_issue_difficulty(
        {"cop": "Style/MixinUsage", "fp": 3, "fn": 0, "matches": 100},
        {
            "easy": False,
            "extended_only_edge_case": True,
        },
    )
    assert difficulty == "complex"


def test_build_start_here_section_uses_repo_hotspots_and_examples():
    corpus = {
        "repo_breakdown": {
            "travis-ci__dpl__8c6eabc": {"fp": 3, "fn": 0},
            "puppetlabs__puppet__e227c27": {"fp": 0, "fn": 4},
            "autolab__Autolab__674efe9": {"fp": 2, "fn": 0},
        },
        "fp_examples": [
            {"loc": "travis-ci__dpl__8c6eabc: lib/foo.rb:10", "msg": "bad fp", "src": None},
            {"loc": "autolab__Autolab__674efe9: app/bar.rb:20", "msg": "another fp", "src": None},
        ],
        "fn_examples": [
            {"loc": "puppetlabs__puppet__e227c27: manifests/init.rb:30", "msg": "missed fn", "src": None},
        ],
    }
    section = gct.build_start_here_section("Style/MixinUsage", corpus)
    assert "## Start Here" in section
    assert "python3 scripts/investigate-cop.py Style/MixinUsage --repos-only" in section
    assert "`travis-ci__dpl__8c6eabc` (3 FP) — example `lib/foo.rb:10`" in section
    assert "`puppetlabs__puppet__e227c27` (4 FN) — example `manifests/init.rb:30`" in section
    assert "Representative FP examples:" in section
    assert "Representative FN examples:" in section


def test_build_start_here_section_empty_when_no_corpus_examples():
    section = gct.build_start_here_section(
        "Style/Foo",
        {"repo_breakdown": {}, "fp_examples": [], "fn_examples": []},
    )
    assert section == ""


def test_choose_issue_state_preserves_blocked_without_open_pr():
    issue = {"labels": [{"name": "state:blocked"}]}
    assert gct.choose_issue_state(issue, has_open_pr=False) == "state:blocked"
    assert gct.choose_issue_state(issue, has_open_pr=True) == "state:pr-open"


def test_sorted_dispatch_candidates_orders_by_tier_then_total_then_cop():
    issues = [
        {"number": 3, "title": "[cop] Style/Zed", "body": "<!-- nitrocop-cop-tracker: cop=Style/Zed total=4 difficulty=simple -->", "labels": []},
        {"number": 1, "title": "[cop] Layout/Foo", "body": "<!-- nitrocop-cop-tracker: cop=Layout/Foo total=3 difficulty=simple -->", "labels": []},
        {"number": 2, "title": "[cop] Metrics/Bar", "body": "<!-- nitrocop-cop-tracker: cop=Metrics/Bar total=2 difficulty=medium -->", "labels": []},
    ]
    ordered = gct.sorted_dispatch_candidates(issues)
    assert [issue["number"] for issue in ordered] == [1, 3, 2]


def test_cmd_issues_sync_reopens_diverging_issue_and_closes_resolved_issue():
    calls = []
    original_funcs = {
        "ensure_labels": gct.ensure_labels,
        "fetch_corpus_for_sync": gct.fetch_corpus_for_sync,
        "list_tracker_issues": gct.list_tracker_issues,
        "list_agent_fix_prs": gct.list_agent_fix_prs,
        "select_backend_for_entry": gct.select_backend_for_entry,
        "reopen_tracker_issue": gct.reopen_tracker_issue,
        "comment_on_issue": gct.comment_on_issue,
        "update_tracker_issue": gct.update_tracker_issue,
        "close_tracker_issue": gct.close_tracker_issue,
        "create_tracker_issue": gct.create_tracker_issue,
    }
    gct.ensure_labels = lambda repo: calls.append(("ensure", repo))
    gct.fetch_corpus_for_sync = lambda input_path, extended: (
        {
            "by_cop": [
                {"cop": "Style/Foo", "fp": 1, "fn": 2, "matches": 55},
                {"cop": "Layout/Done", "fp": 0, "fn": 0, "matches": 120},
            ]
        },
        "123",
        "abc",
    )
    gct.list_tracker_issues = lambda repo: [
        {
            "number": 11,
            "title": "[cop] Style/Foo",
            "state": "CLOSED",
            "url": "https://example.com/issues/11",
            "body": "<!-- nitrocop-cop-tracker: cop=Style/Foo fp=1 fn=2 total=3 matches=55 difficulty=simple -->",
            "labels": [{"name": "cop-tracker"}],
        },
        {
            "number": 12,
            "title": "[cop] Layout/Done",
            "state": "OPEN",
            "url": "https://example.com/issues/12",
            "body": "<!-- nitrocop-cop-tracker: cop=Layout/Done fp=0 fn=1 total=1 matches=120 difficulty=simple -->",
            "labels": [{"name": "cop-tracker"}, {"name": "state:backlog"}],
        },
    ]
    gct.list_agent_fix_prs = lambda repo, state="all": []
    gct.select_backend_for_entry = lambda *args, **kwargs: {
        "backend": "codex-normal",
        "reason": "easy",
        "tier": 1,
        "code_bugs": 1,
        "config_issues": 0,
        "easy": True,
    }
    gct.reopen_tracker_issue = lambda repo, number: calls.append(("reopen", number))
    gct.comment_on_issue = lambda repo, number, body: calls.append(("comment", number, body))
    gct.update_tracker_issue = lambda repo, number, title, body, labels: calls.append(("update", number, labels))
    gct.close_tracker_issue = lambda repo, number, body: calls.append(("close", number, body))
    gct.create_tracker_issue = lambda repo, title, body, labels: calls.append(("create", title, labels))
    try:
        gct.cmd_issues_sync(
            SimpleNamespace(repo="6/nitrocop", input=None, extended=True, binary=None)
        )
    finally:
        for name, func in original_funcs.items():
            setattr(gct, name, func)
    assert ("reopen", 11) in calls
    assert any(call[0] == "update" and call[1] == 11 for call in calls)
    assert any(call[0] == "close" and call[1] == 12 for call in calls)


def test_cmd_dispatch_issues_respects_capacity_and_uses_auto_backend():
    original_list_tracker_issues = gct.list_tracker_issues
    original_active_agent_fix_count = gct.active_agent_fix_count
    original_run = gct.subprocess.run
    gct.list_tracker_issues = lambda repo: [
        {
            "number": 21,
            "title": "[cop] Layout/Foo",
            "state": "OPEN",
            "body": "<!-- nitrocop-cop-tracker: cop=Layout/Foo fp=1 fn=2 total=3 matches=60 difficulty=simple -->",
            "labels": [{"name": "cop-tracker"}, {"name": "state:backlog"}, {"name": "difficulty:simple"}],
        },
        {
            "number": 22,
            "title": "[cop] Style/Bar",
            "state": "OPEN",
            "body": "<!-- nitrocop-cop-tracker: cop=Style/Bar fp=2 fn=2 total=4 matches=80 difficulty=medium -->",
            "labels": [{"name": "cop-tracker"}, {"name": "state:backlog"}, {"name": "difficulty:medium"}],
        },
    ]
    gct.active_agent_fix_count = lambda repo: (1, 1, 1)
    gct.subprocess.run = lambda *args, **kwargs: None
    stdout = io.StringIO()
    try:
        with redirect_stdout(stdout):
            gct.cmd_dispatch_issues(
                SimpleNamespace(
                    repo="6/nitrocop",
                    max_active=2,
                    dry_run=True,
                    backend_family_override="auto",
                    strength_override="auto",
                )
            )
    finally:
        gct.list_tracker_issues = original_list_tracker_issues
        gct.active_agent_fix_count = original_active_agent_fix_count
        gct.subprocess.run = original_run
    payload = json.loads(stdout.getvalue())
    assert payload["capacity"] == 1
    assert payload["selected"] == [
        {
            "issue": 21,
            "cop": "Layout/Foo",
            "difficulty": "simple",
            "backend_family": "auto",
            "strength": "auto",
        }
    ]


if __name__ == "__main__":
    test_pascal_to_snake()
    test_parse_cop_name()
    test_dept_dir_name()
    test_extract_source_lines()
    test_extract_source_lines_no_offense()
    test_find_enclosing_structure_begin()
    test_find_enclosing_structure_class()
    test_find_enclosing_structure_none()
    test_find_enclosing_structure_top_level()
    test_extract_spec_excerpts()
    test_extract_spec_excerpts_empty()
    test_detect_prism_pitfalls()
    test_detect_prism_pitfalls_none()
    test_format_with_diagnostics_omits_no_source_examples_when_diagnosed_exists()
    test_format_with_diagnostics_keeps_no_source_examples_when_they_are_all_we_have()
    test_select_backend_for_entry_retry_uses_claude()
    test_has_failed_attempt_ignores_open_prs()
    test_select_backend_for_entry_uses_issue_difficulty_when_present()
    test_select_backend_for_entry_easy_cop_uses_codex_normal()
    test_assess_cross_corpus_risk_flags_extended_only_edge_case()
    test_build_cross_corpus_risk_section_warns_about_standard_perfect_baseline()
    test_select_backend_for_entry_uses_claude_for_extended_only_edge_case()
    test_classify_issue_difficulty_escalates_extended_only_edge_case()
    test_build_start_here_section_uses_repo_hotspots_and_examples()
    test_build_start_here_section_empty_when_no_corpus_examples()
    test_choose_issue_state_preserves_blocked_without_open_pr()
    test_sorted_dispatch_candidates_orders_by_tier_then_total_then_cop()
    test_cmd_issues_sync_reopens_diverging_issue_and_closes_resolved_issue()
    test_cmd_dispatch_issues_respects_capacity_and_uses_auto_backend()
    print("All tests passed.")
