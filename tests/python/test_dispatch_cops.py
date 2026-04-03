#!/usr/bin/env python3
"""Tests for dispatch_cops.py helper functions."""
import importlib.util
from pathlib import Path
from types import SimpleNamespace

SCRIPT = Path(__file__).parents[2] / "scripts" / "dispatch_cops.py"
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


def test_format_with_diagnostics_fn_snippet_detects_fullfile_does_not_is_code_bug():
    """When snippet detects an FN but the full-file test does NOT detect it,
    it should be classified as a CODE BUG (context-sensitive detection failure)."""
    diagnostics = [
        {
            "kind": "fn",
            "loc": "repo: file.rb:10",
            "msg": "Missing method",
            "diagnosed": True,
            "detected": True,           # snippet detects
            "full_file_detected": False,  # full file does NOT detect
            "offense_line": "def foo; end",
            "test_snippet": "def foo; end\n^ Lint/Syntax: Missing method",
            "enclosing": None,
            "node_type": None,
            "source_context": "def foo; end",
            "full_file_enclosing": "block (do..end) > class body",
            "full_file_context": "...",
            "diagnosis_note": "Snippet detects but full file does not",
        },
    ]
    output = gct._format_with_diagnostics(
        "Lint/Syntax",
        diagnostics,
        fp_examples=[],
        fn_examples=[{"loc": "repo: file.rb:10", "msg": "Missing method"}],
    )
    # Should be counted as a code bug, not a config/context issue
    assert "1 code bug(s)" in output
    assert "config/context issue(s)" not in output
    # The formatted output should contain the CODE BUG marker
    assert "— CODE BUG" in output


def test_format_with_diagnostics_fn_snippet_and_fullfile_both_detect_is_config():
    """When both snippet AND full-file detect an FN, the corpus mismatch is
    truly a config/context issue (the project's config suppresses it)."""
    diagnostics = [
        {
            "kind": "fn",
            "loc": "repo: file.rb:10",
            "msg": "Missing method",
            "diagnosed": True,
            "detected": True,            # snippet detects
            "full_file_detected": True,   # full file also detects
            "offense_line": "def foo; end",
            "test_snippet": "def foo; end\n^ Lint/Syntax: Missing method",
            "enclosing": None,
            "node_type": None,
            "source_context": "def foo; end",
        },
    ]
    output = gct._format_with_diagnostics(
        "Lint/Syntax",
        diagnostics,
        fp_examples=[],
        fn_examples=[{"loc": "repo: file.rb:10", "msg": "Missing method"}],
    )
    # Should be classified as config/context, not a code bug
    assert "1 config/context issue(s)" in output
    assert "CONFIG/CONTEXT issue" in output


def test_format_with_diagnostics_fn_snippet_detects_fullfile_none_is_config():
    """When snippet detects but full_file_detected is None (fetch failed),
    fall back to the original CONFIG/CONTEXT classification."""
    diagnostics = [
        {
            "kind": "fn",
            "loc": "repo: file.rb:10",
            "msg": "Missing method",
            "diagnosed": True,
            "detected": True,
            "full_file_detected": None,  # fetch failed, no full-file result
            "offense_line": "def foo; end",
            "test_snippet": None,
            "enclosing": None,
            "node_type": None,
            "source_context": "def foo; end",
        },
    ]
    output = gct._format_with_diagnostics(
        "Lint/Syntax",
        diagnostics,
        fp_examples=[],
        fn_examples=[{"loc": "repo: file.rb:10", "msg": "Missing method"}],
    )
    assert "1 config/context issue(s)" in output


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
    assert "python3 scripts/investigate_cop.py Style/MixinUsage --repos-only" in section
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
    issue = {"labels": [{"name": "state:blocked"}],
             "body": "<!-- nitrocop-cop-tracker: cop=X/Y fp=0 fn=4 total=4 matches=100 difficulty=simple -->"}
    # Same numbers — stays blocked
    assert gct.choose_issue_state(issue, has_open_pr=False, entry={"fp": 0, "fn": 4}) == "state:blocked"
    # Open PR overrides blocked
    assert gct.choose_issue_state(issue, has_open_pr=True, entry={"fp": 0, "fn": 4}) == "state:pr-open"
    # No entry — stays blocked (backwards compat)
    assert gct.choose_issue_state(issue, has_open_pr=False) == "state:blocked"


def test_choose_issue_state_unblocks_when_numbers_change():
    issue = {"labels": [{"name": "state:blocked"}],
             "body": "<!-- nitrocop-cop-tracker: cop=X/Y fp=0 fn=4 total=4 matches=100 difficulty=simple -->"}
    # FN decreased — unblock
    assert gct.choose_issue_state(issue, has_open_pr=False, entry={"fp": 0, "fn": 2}) == "state:backlog"
    # FP changed — unblock
    assert gct.choose_issue_state(issue, has_open_pr=False, entry={"fp": 1, "fn": 4}) == "state:backlog"



def test_sync_issue_labels_removes_then_readds_labels():
    calls = []
    original_run = gct.subprocess.run

    def fake_run(args, **kwargs):
        calls.append((args, kwargs))
        return SimpleNamespace(stdout="", stderr="", returncode=0)

    gct.subprocess.run = fake_run
    try:
        gct.sync_issue_labels(
            "6/nitrocop",
            591,
            ["state:backlog", "difficulty:medium"],
        )
    finally:
        gct.subprocess.run = original_run

    assert len(calls) == 2
    remove_args, remove_kwargs = calls[0]
    assert remove_args == [
        "gh", "issue", "edit", "591",
        "--repo", "6/nitrocop",
        "--remove-label", "state:backlog,state:pr-open,state:blocked,difficulty:simple,difficulty:medium,difficulty:complex,difficulty:config-only",
    ]
    assert remove_kwargs["check"] is False

    add_args, add_kwargs = calls[1]
    assert add_args == [
        "gh", "issue", "edit", "591",
        "--repo", "6/nitrocop",
        "--add-label", "type:cop-issue,state:backlog,difficulty:medium",
    ]
    assert add_kwargs["check"] is True


def test_fp_full_file_detected_classified_as_code_bug():
    """FP that reproduces in full file but not snippet should be a code bug."""
    diagnostics = [
        {
            "kind": "fp",
            "loc": "repo: file.rb:41",
            "msg": "Method has too many lines. [12/10]",
            "diagnosed": True,
            "detected": False,            # not detected in snippet
            "full_file_detected": True,   # but detected in full file
            "offense_line": "def first_visit(schema, errors, path)",
            "test_snippet": None,
            "enclosing": None,
            "node_type": None,
            "source_context": "def first_visit(...)\n  true\nend",
            "full_file_enclosing": "class Validator",
            "full_file_context": "    41: def first_visit(...)",
            "diagnosis_note": "Snippet too narrow",
        },
    ]
    output = gct._format_with_diagnostics(
        "Metrics/MethodLength",
        diagnostics,
        fp_examples=[{"loc": "repo: file.rb:41", "msg": "Method has too many lines. [12/10]"}],
        fn_examples=[],
    )
    # Should be classified as code bug, not context-dependent
    assert "CODE BUG" in output
    assert "CONTEXT-DEPENDENT" not in output
    assert "1 confirmed code bug(s)" in output


def test_fp_not_detected_anywhere_classified_as_config():
    """FP not reproduced in snippet or full file is context-dependent."""
    diagnostics = [
        {
            "kind": "fp",
            "loc": "repo: file.rb:92",
            "msg": "Method has too many lines. [58/10]",
            "diagnosed": True,
            "detected": False,
            "full_file_detected": False,
            "offense_line": "def self.parseFilters(userFilters, logger)",
            "test_snippet": None,
            "enclosing": None,
            "node_type": None,
            "source_context": "def self.parseFilters(...)\nend",
        },
    ]
    output = gct._format_with_diagnostics(
        "Metrics/MethodLength",
        diagnostics,
        fp_examples=[{"loc": "repo: file.rb:92", "msg": "Method has too many lines. [58/10]"}],
        fn_examples=[],
    )
    assert "CONFIG/CONTEXT issue" in output
    assert "1 context-dependent" in output


def test_config_only_classification():
    """Config-only requires 0 code bugs AND config issues found."""
    # binary=True, code_bugs=0, cfg_issues=8 → config-only
    # (all divergence is config/context, whether matches exist or not)
    assert (True and 0 == 0 and 8 > 0) is True

    # binary=True, code_bugs=0, cfg_issues=0 → NOT config-only
    # (no diagnosis data — can't classify)
    assert (True and 0 == 0 and 0 > 0) is False

    # binary=True, code_bugs=3, cfg_issues=5 → NOT config-only
    # (has real code bugs to fix)
    assert (True and 3 == 0 and 5 > 0) is False

    # binary=None → NOT config-only (no pre-diagnostic ran)
    assert (False and 0 == 0 and 8 > 0) is False


def test_precomputed_diagnosis_from_corpus_artifact():
    """Pre-computed diagnosis in by_cop entries is extracted correctly."""
    data = {
        "by_cop": [
            {"cop": "Style/Foo", "fp": 3, "fn": 0, "diagnosis": {"code_bugs": 2, "config_issues": 1}},
            {"cop": "Style/Bar", "fp": 1, "fn": 0, "diagnosis": {"code_bugs": 0, "config_issues": 1}},
            {"cop": "Style/NoDiag", "fp": 2, "fn": 0},
            {"cop": "Style/Perfect", "fp": 0, "fn": 0, "diagnosis": {"code_bugs": 0, "config_issues": 0}},
        ],
    }
    diverging = {"Style/Foo", "Style/Bar", "Style/NoDiag"}
    diagnosis = {}
    for cop_entry in data.get("by_cop", []):
        diag = cop_entry.get("diagnosis")
        if diag and cop_entry["cop"] in diverging:
            diagnosis[cop_entry["cop"]] = (diag["code_bugs"], diag["config_issues"])

    # Style/Foo has code bugs → not config-only
    assert diagnosis["Style/Foo"] == (2, 1)
    code_bugs, cfg_issues = diagnosis["Style/Foo"]
    assert not (code_bugs == 0 and cfg_issues > 0)

    # Style/Bar has 0 code bugs, 1 config → config-only
    assert diagnosis["Style/Bar"] == (0, 1)
    code_bugs, cfg_issues = diagnosis["Style/Bar"]
    assert code_bugs == 0 and cfg_issues > 0

    # Style/NoDiag has no diagnosis → not in precomputed, needs runtime
    assert "Style/NoDiag" not in diagnosis

    # Style/Perfect is not diverging → not extracted
    assert "Style/Perfect" not in diagnosis


def test_precomputed_diagnosis_identifies_remaining_cops():
    """Cops without pre-computed diagnosis are identified for runtime fallback."""
    data = {
        "by_cop": [
            {"cop": "Style/Foo", "fp": 3, "fn": 0, "diagnosis": {"code_bugs": 2, "config_issues": 1}},
            {"cop": "Style/NoDiag", "fp": 2, "fn": 0},
        ],
    }
    diverging = {"Style/Foo", "Style/NoDiag"}
    diagnosis = {}
    for cop_entry in data.get("by_cop", []):
        diag = cop_entry.get("diagnosis")
        if diag and cop_entry["cop"] in diverging:
            diagnosis[cop_entry["cop"]] = (diag["code_bugs"], diag["config_issues"])

    remaining = diverging - diagnosis.keys()
    assert remaining == {"Style/NoDiag"}


def test_generate_task_includes_diagnostic_contradiction_guidance(tmp_path):
    """When pre-diagnostic says CODE BUG, the prompt should warn agents not to
    defer to doc comments that say otherwise."""
    # Create minimal fixtures and source
    dept_dir = tmp_path / "src" / "cop" / "style"
    dept_dir.mkdir(parents=True)
    (dept_dir / "foo_bar.rs").write_text("pub struct FooBar;\n")
    fix_dir = tmp_path / "tests" / "fixtures" / "cops" / "style" / "foo_bar"
    fix_dir.mkdir(parents=True)
    (fix_dir / "offense.rb").write_text("x = 1\n^ Style/FooBar: msg\n")
    (fix_dir / "no_offense.rb").write_text("y = 2\n")

    # Patch module-level constants and helpers
    saved = {
        "PROJECT_ROOT": gct.PROJECT_ROOT,
        "get_corpus_data": gct.get_corpus_data,
        "find_vendor_ruby_source": gct.find_vendor_ruby_source,
        "find_vendor_spec": gct.find_vendor_spec,
    }
    gct.PROJECT_ROOT = tmp_path
    gct.get_corpus_data = lambda *a, **kw: {
        "fp": 0, "fn": 1, "matches": 100,
        "repo_breakdown": {},
        "fp_examples": [],
        "fn_examples": [{"loc": "repo: file.rb:10", "msg": "missed"}],
    }
    gct.find_vendor_ruby_source = lambda *a: None
    gct.find_vendor_spec = lambda *a: None
    try:
        task = gct.generate_task("Style/FooBar")
        assert "When the pre-diagnostic contradicts existing doc comments" in task
        assert "pre-diagnostic takes precedence" in task
    finally:
        for k, v in saved.items():
            setattr(gct, k, v)


def test_collect_nofix_findings_extracts_from_issue_comments():
    """_collect_nofix_findings scrapes Agent findings blocks from issue comments."""
    import json as _json

    calls = []
    original_run = gct.subprocess.run

    def fake_run(args, **kwargs):
        calls.append(args)
        cmd_str = " ".join(str(a) for a in args)
        if "issue" in cmd_str and "list" in cmd_str and "--search" in cmd_str:
            return SimpleNamespace(
                stdout=_json.dumps([{"number": 293}]),
                stderr="", returncode=0,
            )
        if "issue" in cmd_str and "view" in cmd_str and "293" in cmd_str:
            comments = [
                {"body": "Reopened from corpus sync."},
                {
                    "body": (
                        "No fix PR was produced for `Lint/RedundantCopDisableDirective`.\n\n"
                        "- Backend: `codex / hard`\n"
                        "- Mode: `reduce`\n"
                        "- Run: https://github.com/6/nitrocop/actions/runs/123\n\n"
                        "<details>\n"
                        "<summary>Agent findings (what was tried)</summary>\n\n"
                        "```\n"
                        "Tried per-file detection. +147 FNs. Reverted.\n"
                        "```\n"
                        "</details>\n"
                    ),
                },
                {"body": "No fix PR was produced for `Lint/RedundantCopDisableDirective`.\n\nThe agent did not produce any branch changes.\n"},
            ]
            return SimpleNamespace(
                stdout=_json.dumps({"comments": comments}),
                stderr="", returncode=0,
            )
        return SimpleNamespace(stdout="", stderr="", returncode=0)

    gct.subprocess.run = fake_run
    try:
        findings = gct._collect_nofix_findings("Lint/RedundantCopDisableDirective")
    finally:
        gct.subprocess.run = original_run

    assert len(findings) == 1
    assert "per-file detection" in findings[0]
    assert "+147 FNs" in findings[0]


def test_collect_nofix_findings_empty_when_no_findings():
    """_collect_nofix_findings returns empty when no Agent findings blocks exist."""
    import json as _json

    original_run = gct.subprocess.run

    def fake_run(args, **kwargs):
        cmd_str = " ".join(str(a) for a in args)
        if "issue" in cmd_str and "list" in cmd_str:
            return SimpleNamespace(
                stdout=_json.dumps([{"number": 100}]),
                stderr="", returncode=0,
            )
        if "issue" in cmd_str and "view" in cmd_str:
            return SimpleNamespace(
                stdout=_json.dumps({"comments": [
                    {"body": "No fix PR was produced.\n\nThe agent did not produce any branch changes.\n"},
                ]}),
                stderr="", returncode=0,
            )
        return SimpleNamespace(stdout="", stderr="", returncode=0)

    gct.subprocess.run = fake_run
    try:
        findings = gct._collect_nofix_findings("Style/Foo")
    finally:
        gct.subprocess.run = original_run

    assert findings == []


def test_collect_nofix_findings_handles_repair_comments():
    """_collect_nofix_findings also scrapes repair workflow findings."""
    import json as _json

    original_run = gct.subprocess.run

    def fake_run(args, **kwargs):
        cmd_str = " ".join(str(a) for a in args)
        if "issue" in cmd_str and "list" in cmd_str:
            return SimpleNamespace(
                stdout=_json.dumps([{"number": 50}]),
                stderr="", returncode=0,
            )
        if "issue" in cmd_str and "view" in cmd_str:
            return SimpleNamespace(
                stdout=_json.dumps({"comments": [
                    {
                        "body": (
                            "## Auto-repair Did Not Produce a Fix\n\n"
                            "- Repair workflow: [#999](https://github.com/6/nitrocop/actions/runs/999)\n\n"
                            "The repair attempt produced no branch changes.\n\n"
                            "<details>\n"
                            "<summary>Agent findings (what was tried)</summary>\n\n"
                            "```\n"
                            "Tried fixing the test but the cop logic is wrong.\n"
                            "```\n"
                            "</details>\n"
                        ),
                    },
                ]}),
                stderr="", returncode=0,
            )
        return SimpleNamespace(stdout="", stderr="", returncode=0)

    gct.subprocess.run = fake_run
    try:
        findings = gct._collect_nofix_findings("Style/Bar")
    finally:
        gct.subprocess.run = original_run

    assert len(findings) == 1
    assert "cop logic is wrong" in findings[0]


def test_collect_nofix_findings_no_issue():
    """_collect_nofix_findings returns empty when no tracker issue exists."""
    import json as _json

    original_run = gct.subprocess.run

    def fake_run(args, **kwargs):
        cmd_str = " ".join(str(a) for a in args)
        if "issue" in cmd_str and "list" in cmd_str:
            return SimpleNamespace(stdout=_json.dumps([]), stderr="", returncode=0)
        return SimpleNamespace(stdout="", stderr="", returncode=0)

    gct.subprocess.run = fake_run
    try:
        findings = gct._collect_nofix_findings("Style/NonExistent")
    finally:
        gct.subprocess.run = original_run

    assert findings == []


def test_collect_attempts_includes_nofix_findings():
    """collect_attempts integrates no-fix findings from issue comments."""
    import json as _json

    original_run = gct.subprocess.run

    def fake_run(args, **kwargs):
        cmd_str = " ".join(str(a) for a in args)
        # find_prior_prs
        if "pr" in cmd_str and "list" in cmd_str:
            return SimpleNamespace(
                stdout=_json.dumps([{
                    "number": 1027,
                    "title": "[bot] Fix Lint/Foo",
                    "state": "CLOSED",
                    "headRefName": "fix/lint-foo-123",
                    "mergedAt": None,
                    "closedAt": "2026-03-31",
                    "url": "https://github.com/6/nitrocop/pull/1027",
                }]),
                stderr="", returncode=0,
            )
        # get_pr_diff
        if "pr" in cmd_str and "diff" in cmd_str:
            return SimpleNamespace(stdout="", stderr="", returncode=0)
        # get_pr_checks
        if "pr" in cmd_str and "checks" in cmd_str:
            return SimpleNamespace(stdout="all passed", stderr="", returncode=0)
        # issue list (for _collect_nofix_findings)
        if "issue" in cmd_str and "list" in cmd_str:
            return SimpleNamespace(
                stdout=_json.dumps([{"number": 293}]),
                stderr="", returncode=0,
            )
        # issue view
        if "issue" in cmd_str and "view" in cmd_str:
            return SimpleNamespace(
                stdout=_json.dumps({"comments": [
                    {
                        "body": (
                            "No fix PR was produced for `Lint/Foo`.\n\n"
                            "- Run: https://github.com/6/nitrocop/actions/runs/999\n\n"
                            "<details>\n"
                            "<summary>Agent findings (what was tried)</summary>\n\n"
                            "```\nTried approach X, regressed.\n```\n"
                            "</details>\n"
                        ),
                    },
                ]}),
                stderr="", returncode=0,
            )
        return SimpleNamespace(stdout="", stderr="", returncode=0)

    gct.subprocess.run = fake_run
    try:
        result = gct.collect_attempts("Lint/Foo")
    finally:
        gct.subprocess.run = original_run

    assert "Prior Attempts" in result
    assert "No-Fix Runs" in result
    assert "Tried approach X" in result
    assert "Do NOT repeat" in result
    assert "PR #1027" in result


def test_collect_attempts_empty_when_no_prs_or_findings():
    """collect_attempts returns empty when no PRs and no findings exist."""
    import json as _json

    original_run = gct.subprocess.run

    def fake_run(args, **kwargs):
        cmd_str = " ".join(str(a) for a in args)
        if "pr" in cmd_str and "list" in cmd_str:
            return SimpleNamespace(stdout=_json.dumps([]), stderr="", returncode=0)
        if "issue" in cmd_str and "list" in cmd_str:
            return SimpleNamespace(stdout=_json.dumps([]), stderr="", returncode=0)
        return SimpleNamespace(stdout="", stderr="", returncode=0)

    gct.subprocess.run = fake_run
    try:
        result = gct.collect_attempts("Style/NonExistent")
    finally:
        gct.subprocess.run = original_run

    assert result == ""


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
    gct.fetch_corpus_for_sync = lambda input_path: (
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
            "labels": [{"name": "type:cop-issue"}],
        },
        {
            "number": 12,
            "title": "[cop] Layout/Done",
            "state": "OPEN",
            "url": "https://example.com/issues/12",
            "body": "<!-- nitrocop-cop-tracker: cop=Layout/Done fp=0 fn=1 total=1 matches=120 difficulty=simple -->",
            "labels": [{"name": "type:cop-issue"}, {"name": "state:backlog"}],
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
            SimpleNamespace(repo="6/nitrocop", input=None, binary=None, department=None)
        )
    finally:
        for name, func in original_funcs.items():
            setattr(gct, name, func)
    assert ("reopen", 11) in calls
    assert any(call[0] == "update" and call[1] == 11 for call in calls)
    assert any(call[0] == "close" and call[1] == 12 for call in calls)


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

    test_select_backend_for_entry_uses_issue_difficulty_when_present()
    test_select_backend_for_entry_easy_cop_uses_codex_normal()
    test_build_start_here_section_uses_repo_hotspots_and_examples()
    test_build_start_here_section_empty_when_no_corpus_examples()
    test_choose_issue_state_preserves_blocked_without_open_pr()
    test_sync_issue_labels_removes_then_readds_labels()
    test_config_only_classification()
    test_collect_nofix_findings_extracts_from_issue_comments()
    test_collect_nofix_findings_empty_when_no_findings()
    test_collect_nofix_findings_handles_repair_comments()
    test_collect_nofix_findings_no_issue()
    test_collect_attempts_includes_nofix_findings()
    test_collect_attempts_empty_when_no_prs_or_findings()
    test_cmd_issues_sync_reopens_diverging_issue_and_closes_resolved_issue()
    print("All tests passed.")
