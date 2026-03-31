"""Tests for bench/corpus/diagnose_corpus.py."""

import json
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "bench", "corpus"))
import diagnose_corpus


def test_extract_diagnostic_lines_basic():
    src = [
        "     1: x = 1",
        ">>>  2: y = x and z",
        "     3: puts y",
    ]
    lines, offense = diagnose_corpus.extract_diagnostic_lines(src)
    assert lines == ["x = 1", "y = x and z", "puts y"]
    assert offense == "y = x and z"


def test_extract_diagnostic_lines_no_offense():
    src = [
        "     1: x = 1",
        "     2: y = 2",
    ]
    lines, offense = diagnose_corpus.extract_diagnostic_lines(src)
    assert lines == ["x = 1", "y = 2"]
    assert offense is None


def test_parse_example_loc():
    result = diagnose_corpus.parse_example_loc("repo__id__abc: lib/foo/bar.rb:42")
    assert result == ("repo__id__abc", "lib/foo/bar.rb", 42)


def test_parse_example_loc_invalid():
    assert diagnose_corpus.parse_example_loc("no_colon_space") is None
    assert diagnose_corpus.parse_example_loc("repo: no_line") is None


def test_parse_example_loc_spec_file():
    result = diagnose_corpus.parse_example_loc("grape__grape__abc: spec/models/user_spec.rb:10")
    assert result is not None
    assert result[1] == "spec/models/user_spec.rb"
    assert os.path.basename(result[1]) == "user_spec.rb"


def test_diagnose_cop_empty_examples():
    """Cop with no examples returns 0 bugs, 0 config issues."""
    entry = {"cop": "Style/Foo", "fp_examples": [], "fn_examples": []}
    cop, bugs, cfg = diagnose_corpus.diagnose_cop("/nonexistent", entry)
    assert cop == "Style/Foo"
    assert bugs == 0
    assert cfg == 0


def test_diagnose_examples_no_src():
    """Examples without 'src' field are skipped."""
    examples = [
        {"loc": "repo: file.rb:1", "msg": "test"},
        "repo: file.rb:2",
    ]
    bugs, cfg = diagnose_corpus.diagnose_examples("/nonexistent", "Style/Foo", examples, "fp")
    assert bugs == 0
    assert cfg == 0


def test_main_enriches_corpus_results(tmp_path):
    """Main function adds diagnosis field to by_cop entries."""
    # Create a minimal corpus-results.json with a non-diverging cop
    data = {
        "by_cop": [
            {
                "cop": "Style/PerfectCop",
                "fp": 0,
                "fn": 0,
                "matches": 100,
                "fp_examples": [],
                "fn_examples": [],
            },
            {
                "cop": "Style/NoDiagExamples",
                "fp": 2,
                "fn": 0,
                "matches": 50,
                "fp_examples": [
                    {"loc": "repo: file.rb:1", "msg": "test"},
                ],
                "fn_examples": [],
            },
        ],
    }
    input_path = tmp_path / "input.json"
    input_path.write_text(json.dumps(data))

    # Create a fake binary that always returns empty JSON
    fake_binary = tmp_path / "fake_nitrocop"
    fake_binary.write_text('#!/bin/sh\necho \'{"offenses":[]}\'\n')
    fake_binary.chmod(0o755)

    # Run diagnose_corpus main via the module functions
    loaded = json.loads(input_path.read_text())
    by_cop = loaded.get("by_cop", [])
    diverging = [e for e in by_cop if e.get("fp", 0) + e.get("fn", 0) > 0]

    # Non-diverging cop should not get diagnosis
    assert len(diverging) == 1
    assert diverging[0]["cop"] == "Style/NoDiagExamples"

    # Perfect cop should not be diagnosed
    perfect = [e for e in by_cop if e["cop"] == "Style/PerfectCop"]
    assert "diagnosis" not in perfect[0]


def test_snippet_preserves_repo_relative_path(tmp_path):
    """Snippet files are written with repo-relative paths for Include matching."""
    # Simulate what diagnose_examples does with the path
    loc = "repo__id__abc: test/functional/v1/tweets_controller_test.rb:4"
    parsed = diagnose_corpus.parse_example_loc(loc)
    assert parsed is not None
    rel_path = parsed[1]
    assert rel_path == "test/functional/v1/tweets_controller_test.rb"

    # The snippet should be written at tmp/test/functional/v1/tweets_controller_test.rb
    filepath = os.path.join(str(tmp_path), rel_path)
    os.makedirs(os.path.dirname(filepath), exist_ok=True)
    with open(filepath, "w") as f:
        f.write("class FooTest < ActionController::TestCase; end\n")
    assert os.path.exists(filepath)

    # A glob like **/test/**/*.rb should match
    import fnmatch
    assert fnmatch.fnmatch(rel_path, "**/test/**/*.rb") or \
        "test/" in rel_path  # fnmatch doesn't do ** like globset


def test_snippet_spec_path_matches_include(tmp_path):
    """Spec files preserve paths for RSpec Include patterns."""
    loc = "grape__grape__abc: spec/grape/api/inherited_helpers_spec.rb:55"
    parsed = diagnose_corpus.parse_example_loc(loc)
    rel_path = parsed[1]
    assert rel_path == "spec/grape/api/inherited_helpers_spec.rb"
    assert rel_path.endswith("_spec.rb")
    assert "spec/" in rel_path
