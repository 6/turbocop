#!/usr/bin/env python3
"""Tests for prepopulate_fixtures.py."""
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[2] / "scripts" / "ci"))
import prepopulate_fixtures


TASK_WITH_FP = """
# Fix Lint/AmbiguousRange — 4 FP, 0 FN

## Pre-diagnostic Results

### FP #1: `repo: file.rb:10`
**CONFIRMED false positive — CODE BUG**
nitrocop incorrectly flags this pattern in isolation.
Fix the detection logic to not flag this.

Full source context (add relevant parts to no_offense.rb):
```ruby
1.. ..1
```

### FP #2: `repo: file.rb:20`
**CONFIRMED false positive — CODE BUG**
nitrocop incorrectly flags this pattern in isolation.
Fix the detection logic to not flag this.

Full source context (add relevant parts to no_offense.rb):
```ruby
def get_text(start)
  @string[start..@pos-1]
end
```
"""

TASK_WITH_FN = """
# Fix Style/Foo — 0 FP, 2 FN

## Pre-diagnostic Results

### FN #1: `repo: file.rb:5`
**NOT DETECTED — CODE BUG**
The cop fails to detect this pattern. Fix the detection logic.

Ready-made test snippet (add to offense.rb, adjust `^` count):
```ruby
super { |x| x.foo }
      ^^^^^^^^^^^^^^^^^ Style/Foo: Pass `&:foo` as an argument.
```
"""

TASK_WITH_CONFIG_ONLY = """
# Fix Style/Bar — 0 FP, 1 FN

## Pre-diagnostic Results

### FN #1: `repo: file.rb:5`
**DETECTED in isolation — CONFIG/CONTEXT issue**
The cop correctly detects this pattern with default config.
"""

TASK_MIXED = """
# Fix Lint/Baz — 1 FP, 1 FN

## Pre-diagnostic Results

### FN #1: `repo: file.rb:5`
**NOT DETECTED — CODE BUG**
The cop fails to detect this pattern. Fix the detection logic.

Ready-made test snippet (add to offense.rb, adjust `^` count):
```ruby
some_pattern
^ Lint/Baz: Bad pattern.
```

### FP #1: `repo: file.rb:10`
**CONFIRMED false positive — CODE BUG**
nitrocop incorrectly flags this pattern in isolation.
Fix the detection logic to not flag this.

Full source context (add relevant parts to no_offense.rb):
```ruby
safe_pattern_here
```
"""

TASK_WITH_NOISY_BOUNDARIES = """
# Fix Style/MixinUsage — 1 FP, 0 FN

## Pre-diagnostic Results

### FP #1: `repo: file.rb:10`
**CONFIRMED false positive — CODE BUG**
nitrocop incorrectly flags this pattern in isolation.

Full source context (add relevant parts to no_offense.rb):
```ruby
#

BEGIN {
  include UtilityFunctions
}

#
```
"""


def make_fixtures(tmp: Path):
    """Create minimal fixture files."""
    (tmp / "offense.rb").write_text("# existing offenses\nfoo\n")
    (tmp / "no_offense.rb").write_text("# existing no-offenses\nbar\nbaz\nqux\nquux\nquuz\n")


def test_fp_appended_to_no_offense():
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        make_fixtures(tmp)
        task = tmp / "task.md"
        task.write_text(TASK_WITH_FP)
        result = prepopulate_fixtures.prepopulate(task, "Lint/AmbiguousRange", tmp)
        assert result["fp_added"] == 2
        assert result["fn_added"] == 0
        content = (tmp / "no_offense.rb").read_text()
        assert "1.. ..1" in content
        assert "@pos-1" in content
        assert "Pre-populated from corpus" not in content


def test_fn_appended_to_offense():
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        make_fixtures(tmp)
        task = tmp / "task.md"
        task.write_text(TASK_WITH_FN)
        result = prepopulate_fixtures.prepopulate(task, "Style/Foo", tmp)
        assert result["fn_added"] == 1
        assert result["fp_added"] == 0
        content = (tmp / "offense.rb").read_text()
        assert "super { |x| x.foo }" in content
        assert "Style/Foo" in content
        assert "Pre-populated from corpus" not in content


def test_config_only_no_changes():
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        make_fixtures(tmp)
        task = tmp / "task.md"
        task.write_text(TASK_WITH_CONFIG_ONLY)
        result = prepopulate_fixtures.prepopulate(task, "Style/Bar", tmp)
        assert result["fp_added"] == 0
        assert result["fn_added"] == 0


def test_mixed_fp_and_fn():
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        make_fixtures(tmp)
        task = tmp / "task.md"
        task.write_text(TASK_MIXED)
        result = prepopulate_fixtures.prepopulate(task, "Lint/Baz", tmp)
        assert result["fp_added"] == 1
        assert result["fn_added"] == 1
        assert "safe_pattern_here" in (tmp / "no_offense.rb").read_text()
        assert "some_pattern" in (tmp / "offense.rb").read_text()


def test_empty_task():
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        make_fixtures(tmp)
        task = tmp / "task.md"
        task.write_text("# Nothing here")
        result = prepopulate_fixtures.prepopulate(task, "Style/Foo", tmp)
        assert result["fp_added"] == 0
        assert result["fn_added"] == 0


def test_boundary_noise_is_trimmed_from_snippets():
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        make_fixtures(tmp)
        task = tmp / "task.md"
        task.write_text(TASK_WITH_NOISY_BOUNDARIES)
        result = prepopulate_fixtures.prepopulate(task, "Style/MixinUsage", tmp)
        assert result["fp_added"] == 1
        content = (tmp / "no_offense.rb").read_text()
        assert "\n#\n\nBEGIN {" not in content
        assert "BEGIN {\n  include UtilityFunctions\n}" in content
        assert not content.rstrip().endswith("#")
        assert "Pre-populated from corpus" not in content


if __name__ == "__main__":
    test_fp_appended_to_no_offense()
    test_fn_appended_to_offense()
    test_config_only_no_changes()
    test_mixed_fp_and_fn()
    test_empty_task()
    test_boundary_noise_is_trimmed_from_snippets()
    print("All tests passed.")
