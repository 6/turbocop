# Plan: Systematic Testing of All Config Style Variants (All Departments)

## Problem

Nitrocop has **160 cops across all departments** with configurable style parameters (e.g., `EnforcedStyle`, `EnforcedStyleForMultiline`). Of the **172 total style parameters**, only **76 have tests covering non-default values**. The remaining **96 are tested exclusively with the default config — spanning Style (42), Layout (23), RSpec (7), Naming (4), FactoryBot (4), Rails (3), Gemspec (2), Bundler (2), and Lint (2).**

The corpus oracle runs all 5,500 repos against a single `baseline_rubocop.yml` that uses **only default parameter values**. A cop can be "perfect" in the corpus while having broken logic for non-default styles like `comma`, `consistent`, or `semantic`. This is a cross-cutting problem, not limited to any single department.

**Concrete example:** `Style/TrailingCommaInHashLiteral` is corpus-perfect with `no_comma` (default) but produces false positives with `comma` (used by standardrb, shopify, and other popular style gems). The `comma` code path was never exercised by any test — neither fixture nor corpus.

## Scope

96 untested style parameter variants across all departments. Full list in Appendix A.

## Testing Layers

This plan adds style-variant coverage at **three layers**, each catching different classes of bugs:

| Layer | What it catches | Cost |
|-------|----------------|------|
| **Fixture tests** (per-style `.rb` files) | Implementation logic bugs in non-default code paths | Cheap, runs in `cargo test` |
| **Corpus style-overlay runs** (subset of repos with non-default configs) | Real-world edge cases that fixture authors didn't think of | Medium, ~50 repos × N overlays |
| **Corpus native-config spot checks** (repos using their own `.rubocop.yml`) | Config resolution bugs + end-to-end integration | Expensive, manual/on-demand |

## Part 1: Fixture-Level Testing

### Step 1: Add `# nitrocop-config:` directive to fixture parser

**File:** `src/testutil.rs`

Add a new directive recognized by `parse_fixture()`:

```
# nitrocop-config: EnforcedStyleForMultiline: comma
```

When present, the fixture test runner builds a `CopConfig` with the specified key-value pairs in its `options` HashMap, instead of using `CopConfig::default()`.

**Parsing rules:**
- Must appear at the top of the file (before any Ruby code), like `# nitrocop-filename:`.
- Multiple config lines are allowed (one per line):
  ```
  # nitrocop-config: EnforcedStyle: consistent
  # nitrocop-config: IndentationWidth: 4
  ```
- Values are parsed as YAML scalars: strings stay strings, integers become integers, booleans become booleans. This matches how `CopConfig.options` stores values (`serde_yml::Value`).
- The directive line is stripped from the source before parsing (same as other directives).

**Changes to `parse_fixture()`:**
```rust
// In ParsedFixture struct, add:
pub config: CopConfig,

// In parse_fixture(), collect config directives:
let mut config_options: HashMap<String, serde_yml::Value> = HashMap::new();
// For each line matching `# nitrocop-config: key: value`:
//   config_options.insert(key, parse_yaml_value(value));
// Return ParsedFixture { ..., config: CopConfig { options: config_options, ..CopConfig::default() } }
```

**Changes to `assert_cop_offenses_full()`:**

Currently:
```rust
pub fn assert_cop_offenses_full(cop: &dyn Cop, fixture_bytes: &[u8]) {
    assert_cop_offenses_full_with_config(cop, fixture_bytes, CopConfig::default());
}
```

Change to use the parsed config:
```rust
pub fn assert_cop_offenses_full(cop: &dyn Cop, fixture_bytes: &[u8]) {
    let parsed = parse_fixture(fixture_bytes);
    assert_cop_offenses_full_with_config(cop, fixture_bytes, parsed.config);
}
```

This is backwards-compatible: fixtures without `# nitrocop-config:` directives produce `CopConfig::default()`, same as today.

**Important:** `assert_cop_offenses_full_with_config` currently calls `parse_fixture` internally too (at line 385). Refactor so parsing happens once — either the caller parses and passes the result, or the function accepts pre-parsed data.

### Step 2: Extend `cop_fixture_tests!` macro to discover style variant fixtures

**File:** `src/cop/mod.rs`

Currently, `cop_fixture_tests!` generates two tests from `offense.rb` and `no_offense.rb`. Extend it to also discover and test style-variant fixture files.

**Convention for style-variant fixtures:**

```
tests/fixtures/cops/style/trailing_comma_in_hash_literal/
  offense.rb                              # default style (no_comma)
  no_offense.rb                           # default style
  offense.comma.rb                        # EnforcedStyleForMultiline: comma
  no_offense.comma.rb                     # EnforcedStyleForMultiline: comma
  offense.consistent_comma.rb             # EnforcedStyleForMultiline: consistent_comma
  no_offense.consistent_comma.rb          # (optional — only if there are cases to test)
```

Each variant file MUST include a `# nitrocop-config:` directive specifying its config. The `.stylename.` in the filename is just for human readability and test naming — the actual config comes from the directive inside the file.

**Macro changes:**

The macro can't do filesystem discovery at compile time (no `include_dir!` in stable Rust). Two options:

**Option A (recommended): Explicit variant declaration in the macro call.**

```rust
crate::cop_fixture_tests!(
    TrailingCommaInHashLiteral,
    "cops/style/trailing_comma_in_hash_literal",
    variants: ["comma", "consistent_comma"]
);
```

This generates:
- `offense_fixture()` and `no_offense_fixture()` (default, as today)
- `offense_fixture_comma()` and `no_offense_fixture_comma()`
- `offense_fixture_consistent_comma()` and `no_offense_fixture_consistent_comma()`

Each variant test loads `offense.{variant}.rb` / `no_offense.{variant}.rb` and uses the `# nitrocop-config:` directive from the file.

**Option B: Build script generates test code.**

A `build.rs` script scans `tests/fixtures/cops/` for `*.variant.rb` files and generates a Rust source file with test functions. This is more automatic but adds build complexity.

**Recommendation:** Option A. It's explicit, requires no build script, and the list of variants per cop serves as documentation. The downside (manually listing variants) is actually an upside: it forces the developer to think about which styles to test.

### Step 3: Add a CI audit script

**File:** `scripts/audit_style_coverage.py`

A Python script that:

1. Scans all `src/cop/**/*.rs` files for `config.get_str("Enforced*", "default")` calls.
2. Looks up the cop in `vendor/rubocop/config/default.yml` to find `SupportedStyles` / `SupportedStylesForMultiline`.
3. For each supported style value that is NOT the default, checks whether a corresponding fixture file exists (`offense.{style}.rb` or `no_offense.{style}.rb`) OR a `with_config` test referencing that style exists in the Rust test section.
4. Outputs a report of untested style variants.

**Output format:**
```
Style/TrailingCommaInHashLiteral:
  EnforcedStyleForMultiline: no_comma (default) ✓ fixture
  EnforcedStyleForMultiline: comma              ✗ NO TEST
  EnforcedStyleForMultiline: consistent_comma   ✗ NO TEST
  EnforcedStyleForMultiline: diff_comma         ✗ NO TEST

Layout/DotPosition:
  EnforcedStyle: leading (default)  ✓ fixture
  EnforcedStyle: trailing           ✗ NO TEST
```

**CI integration:** Add to the pre-commit or CI pipeline as a non-blocking warning. Over time, as coverage improves, it can become blocking.

### Step 4: Write fixture files for high-priority cops

Start with the cops that caused the real-world FPs, plus the most commonly configured cops in popular open-source style gems (standardrb, shopify/ruby-style-guide, thoughtbot/guides).

**Priority 1 — Cops that caused FPs on real projects (fix immediately):**

| Cop | Untested style | Used by |
|-----|---------------|---------|
| `Style/TrailingCommaInHashLiteral` | `comma`, `consistent_comma` | standardrb forks, shopify |
| `Style/TrailingCommaInArguments`* | already has inline tests, but no fixture coverage for `comma` multiline-detection edge cases | various |
| `Layout/FirstArgumentIndentation` | `consistent` — already tested via `with_config`, but the `consistent` path has a detection bug | standardrb |
| `Layout/FirstHashElementIndentation` | `consistent` — already tested, but detection bug | standardrb |

*TrailingCommaInArguments has 9 inline tests for `comma`/`consistent_comma`, but these are narrow cases. The multiline detection bug seen on real code (all elements on one line, braces on separate lines) isn't covered.

**Priority 2 — Most commonly overridden styles in popular gems (high exposure):**

| Cop | Non-default style | Used by |
|-----|------------------|---------|
| `Style/BlockDelimiters` | `semantic` | thoughtbot, various |
| `Style/StringLiterals` | `double_quotes` | airbnb |
| `Style/HashSyntax` | `ruby19_no_mixed_keys` | many |
| `Layout/MultilineMethodCallIndentation` | `indented` | standardrb |
| `Style/EmptyElse` | `empty`, `nil` | various |
| `Style/FormatString` | `percent`, `sprintf` | various |
| `Style/Lambda` | `literal`, `lambda` | standardrb |
| `Style/AndOr` | `always` | standardrb |
| `Style/RescueStandardError` | `explicit` | various |
| `Layout/DotPosition` | `trailing` | various |

**Priority 3 — Everything else (96 total, work through over time).**

Full list in Appendix A.

### Step 5: Fix the detection bugs found during fixture creation

For each priority-1 cop, writing the fixture file will likely reveal the exact detection bug. Fix it, add the fixture, and verify with `check_cop.py --rerun` that corpus results don't regress (since the corpus uses the default style, fixes to non-default paths should be safe).

## Part 2: Corpus-Level Style-Variant Testing

Fixture tests catch implementation bugs in non-default code paths. But they only test patterns the fixture author thought of. The corpus catches real-world edge cases — but currently only for default configs. This section extends the corpus workflow to exercise non-default styles.

### Step 6: Create style-overlay baseline configs

**Directory:** `bench/corpus/style_overlays/`

Create a small set of baseline config overlays representing the most common non-default style bundles in the wild. Each overlay inherits from the main baseline and overrides specific parameters:

**`bench/corpus/style_overlays/standardrb.yml`** — mirrors standardrb's non-default choices:
```yaml
inherit_from: ../baseline_rubocop.yml

Layout/FirstArgumentIndentation:
  EnforcedStyle: consistent
Layout/FirstHashElementIndentation:
  EnforcedStyle: consistent
Layout/FirstArrayElementIndentation:
  EnforcedStyle: consistent
Layout/MultilineMethodCallIndentation:
  EnforcedStyle: indented
Style/AndOr:
  EnforcedStyle: always
Style/Lambda:
  EnforcedStyle: literal
Style/BlockDelimiters:
  EnforcedStyle: semantic
Style/StringLiterals:
  Enabled: false
Style/StringLiteralsInInterpolation:
  Enabled: false
```

**`bench/corpus/style_overlays/trailing_comma.yml`** — the most common non-default trailing comma config:
```yaml
inherit_from: ../baseline_rubocop.yml

Style/TrailingCommaInArguments:
  EnforcedStyleForMultiline: comma
Style/TrailingCommaInArrayLiteral:
  EnforcedStyleForMultiline: comma
Style/TrailingCommaInHashLiteral:
  EnforcedStyleForMultiline: comma
```

**`bench/corpus/style_overlays/shopify.yml`** — mirrors shopify/ruby-style-guide choices:
```yaml
inherit_from: ../baseline_rubocop.yml

Style/TrailingCommaInArguments:
  EnforcedStyleForMultiline: comma
Style/TrailingCommaInArrayLiteral:
  EnforcedStyleForMultiline: comma
Style/TrailingCommaInHashLiteral:
  EnforcedStyleForMultiline: comma
Layout/DotPosition:
  EnforcedStyle: trailing
Style/RescueStandardError:
  EnforcedStyle: explicit
```

Additional overlays can be added over time. Each overlay should represent a real, popular style gem — not arbitrary combinations.

### Step 7: Add `--style-overlay` flag to `check_cop.py`

**File:** `scripts/check_cop.py`

Add a `--style-overlay <path>` flag that:

1. Uses the specified overlay config instead of `baseline_rubocop.yml` when running nitrocop.
2. Also runs rubocop with the same overlay config to get the expected counts.
3. Compares the two, same as today.

The key difference: both tools use the same non-default config, so the expected offense counts change (more offenses for `comma` style, fewer for cops disabled in the overlay, etc.).

**Implementation:**

In `run_nitrocop.py`, `resolve_repo_config()` currently returns the baseline or a per-repo overlay. Extend it to accept an optional `style_overlay` parameter:

```python
def run_nitrocop(
    repo_dir: str,
    *,
    cop: str | None = None,
    binary: str | None = None,
    timeout: int = 120,
    cwd: str | None = None,
    style_overlay: str | None = None,  # NEW
) -> dict:
```

When `style_overlay` is set, `gen_repo_config.py` merges the style overlay on top of the baseline (and the per-repo overlay if any) before writing the final config.

For rubocop: add a parallel `run_rubocop()` helper (or extend the existing one in `check_cop.py`) that runs rubocop with the same overlay config. This gives us a ground-truth count for the non-default style.

**Usage:**

```bash
# Check TrailingCommaInHashLiteral with comma style on diverging repos
python3 scripts/check_cop.py Style/TrailingCommaInHashLiteral \
    --style-overlay bench/corpus/style_overlays/trailing_comma.yml \
    --rerun --clone --sample 30

# Check FirstArgumentIndentation with standardrb style
python3 scripts/check_cop.py Layout/FirstArgumentIndentation \
    --style-overlay bench/corpus/style_overlays/standardrb.yml \
    --rerun --clone --sample 30
```

**Caching:** Style-overlay runs use a separate cache key (`(binary_mtime, cop_name, repo_id, overlay_hash)`) so they don't collide with baseline results.

### Step 8: Add `check_cop_styles.py` for batch style-variant validation

**File:** `scripts/check_cop_styles.py`

A convenience wrapper that, for a given cop:

1. Reads the cop's `SupportedStyles` from `vendor/rubocop/config/default.yml`.
2. For each non-default style, creates a temporary overlay config.
3. Runs `check_cop.py --style-overlay` with each overlay on a sample of repos.
4. Reports per-style conformance.

**Output:**
```
Style/TrailingCommaInHashLiteral:
  no_comma (default):      50/50 repos match (baseline)
  comma:                   47/50 repos match (3 diverge: repo_a, repo_b, repo_c)
  consistent_comma:        50/50 repos match
  diff_comma:              50/50 repos match
```

**Usage:**
```bash
python3 scripts/check_cop_styles.py Style/TrailingCommaInHashLiteral --sample 50
python3 scripts/check_cop_styles.py Layout/FirstArgumentIndentation --sample 30
```

This is the primary tool for validating that a cop works correctly across all its supported styles against real-world code.

### Step 9: CI integration for style-overlay runs

**File:** `.github/workflows/` (new or existing workflow)

Add an optional CI job (not blocking initially) that runs style-overlay checks for priority-1 and priority-2 cops:

```yaml
style-variant-check:
  runs-on: ubuntu-latest
  strategy:
    matrix:
      overlay: [standardrb, trailing_comma, shopify]
  steps:
    - run: |
        python3 scripts/check_cop_styles.py \
          --overlay bench/corpus/style_overlays/${{ matrix.overlay }}.yml \
          --sample 30 --cops-from-overlay
```

The `--cops-from-overlay` flag means: only check cops whose parameters are overridden in the overlay (no point checking `Lint/Void` against a trailing-comma overlay).

Start as a non-blocking weekly job. Promote to blocking once coverage stabilizes.

## Part 3: Native-Config Spot Checks

The most realistic test: run repos using their own `.rubocop.yml` instead of the baseline. This exercises the full config resolution pipeline — `inherit_gem`, `inherit_from`, `require:`, `plugins:`, `DisabledByDefault`, per-cop Exclude, etc.

### Step 10: Add `--native-config` mode to `check_cop.py`

**File:** `scripts/check_cop.py`

Add a `--native-config` flag that:

1. Uses each repo's own `.rubocop.yml` instead of the baseline.
2. Runs rubocop on the repo with its native config to get expected counts.
3. Runs nitrocop on the same repo with its native config.
4. Compares the two.

**Constraints:**
- Only works on repos that have a valid `.rubocop.yml` and a working `bundle install`.
- Much slower than baseline runs (rubocop must actually run, not just use cached oracle results).
- Not suitable for the full 5,500-repo corpus — use on a curated subset.

**Implementation:**

```python
def run_nitrocop_native(repo_dir: str, *, cop: str | None = None, ...) -> dict:
    """Run nitrocop using the repo's own config (no --config flag)."""
    cmd = [binary, "--preview", "--format", "json", "--no-cache"]
    if cop:
        cmd += ["--only", cop]
    cmd.append(".")
    # Run FROM the repo dir so config resolution works naturally
    result = subprocess.run(cmd, cwd=repo_dir, ...)
```

For rubocop:
```python
def run_rubocop_native(repo_dir: str, *, cop: str | None = None, ...) -> dict:
    """Run rubocop using the repo's own config."""
    cmd = ["bundle", "exec", "rubocop", "--format", "json"]
    if cop:
        cmd += ["--only", cop]
    cmd.append(".")
    result = subprocess.run(cmd, cwd=repo_dir, env=build_native_env(repo_dir), ...)
```

**Usage:**
```bash
# Spot-check a specific repo with its native config
python3 scripts/check_cop.py Style/TrailingCommaInHashLiteral \
    --native-config --repo rails/rails --rerun

# Spot-check against a curated list of repos with diverse configs
python3 scripts/check_cop.py Style/TrailingCommaInHashLiteral \
    --native-config --repos-file bench/corpus/diverse_config_repos.txt --rerun
```

### Step 11: Curate a diverse-config repo list

**File:** `bench/corpus/diverse_config_repos.txt`

A manually curated list of ~30-50 corpus repos that use non-default style configurations. Selected by scanning corpus repos' `.rubocop.yml` for `EnforcedStyle` overrides, `inherit_gem:` from popular style gems, and `DisabledByDefault: true`.

**Selection criteria:**
- Repos that `inherit_gem: standard` (standardrb users)
- Repos that set `EnforcedStyleForMultiline: comma` on trailing comma cops
- Repos that use `DisabledByDefault: true` with selective enablement
- Repos with `inherit_from: .rubocop_todo.yml` that have per-cop Exclude patterns
- At least one repo per major style gem (standardrb, shopify, airbnb, thoughtbot)

**Script to generate initial list:**

```bash
python3 scripts/find_diverse_config_repos.py --min-styles 3 --output bench/corpus/diverse_config_repos.txt
```

This script (new) scans corpus repos for `.rubocop.yml` files, counts non-default style parameters, and outputs the most diverse ones.

## Part 4: `docs/corpus.md` Per-Variant Reporting

`docs/corpus.md` is auto-generated by the corpus oracle workflow via `bench/corpus/diff_results.py --output-md`. Currently it shows one row per cop. For cops with configurable styles, it should show one row per (cop, style) combination, since a cop can be perfect for one style and broken for another.

### Step 12: Extend `diff_results.py` to emit per-variant rows

**File:** `bench/corpus/diff_results.py`

When style-overlay corpus runs exist (from Step 9's CI job), `diff_results.py` should merge baseline + overlay results and render per-variant rows in the Diverging Cops table:

**Current format (one row per cop):**
```
| Style/TrailingCommaInHashLiteral | 15,219 | 4 | 0 | 99.9% |
```

**New format (one row per variant):**
```
| Style/TrailingCommaInHashLiteral (no_comma, default) | 15,219 | 4 | 0 | 99.9% |
| Style/TrailingCommaInHashLiteral (comma) | 12,841 | 0 | 0 | 100.0% |
| Style/TrailingCommaInHashLiteral (consistent_comma) | 11,203 | 0 | 0 | 100.0% |
```

**Implementation:**

1. The CI style-overlay jobs (Step 9) produce separate `corpus-results-{overlay}.json` artifacts alongside the baseline `corpus-results.json`.
2. `diff_results.py` gains a `--style-overlay-results` flag that accepts paths to overlay result files.
3. For each cop that appears in an overlay result, the markdown renderer emits one row per variant. Cops without overlay data keep their single row.
4. The detail `<details>` sections also show per-variant FP/FN samples.

**Fallback:** Until style-overlay CI runs are in place, `docs/corpus.md` continues to show the existing single-row format. The per-variant rows appear automatically once overlay results exist — no manual intervention needed.

### Step 13: Update `docs/corpus-workflow.md`

Add a new section documenting the style-variant testing workflow:

```markdown
## Style-Variant Testing

The baseline corpus config (`bench/corpus/baseline_rubocop.yml`) uses default
parameter values for all cops. This means corpus-perfect cops may have bugs
in non-default code paths (e.g., `EnforcedStyleForMultiline: comma`).

### Style overlays

Style overlays (`bench/corpus/style_overlays/*.yml`) represent popular
non-default style bundles. Use them to validate cops against non-default
configs:

    python3 scripts/check_cop.py Department/CopName \
        --style-overlay bench/corpus/style_overlays/trailing_comma.yml \
        --rerun --clone --sample 30

### Batch style-variant check

Check all supported styles for a cop at once:

    python3 scripts/check_cop_styles.py Department/CopName --sample 50

### Native-config spot checks

For end-to-end validation with a repo's own config:

    python3 scripts/check_cop.py Department/CopName \
        --native-config --repo owner/repo --rerun

A curated list of repos with diverse configs is at
`bench/corpus/diverse_config_repos.txt`.

### Audit coverage

To see which style variants lack test coverage:

    python3 scripts/audit_style_coverage.py
```

## Implementation Order

1. **`src/testutil.rs`** — Add `# nitrocop-config:` directive parsing (~30 lines)
2. **`src/cop/mod.rs`** — Extend `cop_fixture_tests!` macro to support `variants:` (~40 lines)
3. **Priority 1 fixtures** — Write `offense.comma.rb` / `no_offense.comma.rb` etc. for the 4 cops listed above. Fix bugs as found.
4. **`scripts/audit_style_coverage.py`** — Audit script (~100 lines)
5. **Style overlay configs** — Create `bench/corpus/style_overlays/{standardrb,trailing_comma,shopify}.yml`
6. **`check_cop.py --style-overlay`** — Extend `run_nitrocop.py` and `check_cop.py` (~80 lines)
7. **`scripts/check_cop_styles.py`** — Batch style-variant checker (~120 lines)
8. **Priority 2 fixtures** — 10 cops × ~2 fixture files each
9. **`check_cop.py --native-config`** — Native-config mode (~60 lines)
10. **`bench/corpus/diverse_config_repos.txt`** + `scripts/find_diverse_config_repos.py` — Curated repo list
11. **`bench/corpus/diff_results.py`** — Per-variant rows in `docs/corpus.md` when overlay results exist
12. **`docs/corpus-workflow.md`** — Document the new workflow
13. **Priority 3 fixtures** — Remaining 82 cops, done incrementally as cops are touched
14. **CI workflow** — Optional weekly style-variant job

## Verification

- `cargo test --release` passes with all new fixtures
- `scripts/audit_style_coverage.py` shows no regressions (untested count only goes down)
- `check_cop.py --rerun` for any cop whose implementation was modified (baseline corpus doesn't regress)
- `check_cop_styles.py` shows 0 divergences for priority-1 cops across all styles
- `check_cop.py --native-config` on diverse-config repos shows 0 divergences for priority-1 cops
- Re-run nitrocop on the project that surfaced the original FPs — the 30 offenses should drop to 0

## Appendix A: Full List of Untested Style Variants (96)

```
bundler/gem_filename: EnforcedStyle (default=Gemfile)
bundler/gem_version: EnforcedStyle (default=required)
factory_bot/association_style: EnforcedStyle (default=implicit)
factory_bot/consistent_parentheses_style: EnforcedStyle (default=require_parentheses)
factory_bot/create_list: EnforcedStyle (default=create_list)
factory_bot/factory_name_style: EnforcedStyle (default=symbol)
gemspec/dependency_version: EnforcedStyle (default=required)
gemspec/development_dependencies: EnforcedStyle (default=Gemfile)
layout/access_modifier_indentation: EnforcedStyle (default=indent)
layout/begin_end_alignment: EnforcedStyleAlignWith (default=start_of_line)
layout/dot_position: EnforcedStyle (default=leading)
layout/empty_lines_around_access_modifier: EnforcedStyle (default=around)
layout/first_parameter_indentation: EnforcedStyle (default=consistent)
layout/line_continuation_spacing: EnforcedStyle (default=space)
layout/line_end_string_concatenation_indentation: EnforcedStyle (default=aligned)
layout/multiline_array_brace_layout: EnforcedStyle (default=symmetrical)
layout/multiline_assignment_layout: EnforcedStyle (default=new_line)
layout/multiline_hash_brace_layout: EnforcedStyle (default=symmetrical)
layout/multiline_method_call_brace_layout: EnforcedStyle (default=symmetrical)
layout/multiline_method_call_indentation: EnforcedStyle (default=aligned)
layout/multiline_method_definition_brace_layout: EnforcedStyle (default=symmetrical)
layout/parameter_alignment: EnforcedStyle (default=with_first_parameter)
layout/space_around_block_parameters: EnforcedStyleInsidePipes (default=no_space)
layout/space_around_equals_in_parameter_default: EnforcedStyle (default=space)
layout/space_around_operators: EnforcedStyleForExponentOperator (default=no_space)
layout/space_around_operators: EnforcedStyleForRationalLiterals (default=no_space)
layout/space_in_lambda_literal: EnforcedStyle (default=require_no_space)
layout/space_inside_array_literal_brackets: EnforcedStyle (default=no_space)
layout/space_inside_block_braces: EnforcedStyle (default=space)
layout/space_inside_reference_brackets: EnforcedStyle (default=no_space)
layout/space_inside_reference_brackets: EnforcedStyleForEmptyBrackets (default=no_space)
layout/space_inside_string_interpolation: EnforcedStyle (default=no_space)
lint/inherit_exception: EnforcedStyle (default=standard_error)
lint/symbol_conversion: EnforcedStyle (default=strict)
naming/block_forwarding: EnforcedStyle (default=anonymous)
naming/heredoc_delimiter_case: EnforcedStyle (default=uppercase)
naming/variable_number: EnforcedStyle (default=normalcase)
rails/action_filter: EnforcedStyle (default=action)
rails/date: EnforcedStyle (default=flexible)
rails/uniq_before_pluck: EnforcedStyle (default=conservative)
rspec/be_nil: EnforcedStyle (default=be_nil)
rspec/class_check: EnforcedStyle (default=be_a)
rspec/example_without_description: EnforcedStyle (default=always_allow)
rspec/it_behaves_like: EnforcedStyle (default=it_behaves_like)
rspec/metadata_style: EnforcedStyle (default=symbol)
rspec/not_to_not: EnforcedStyle (default=not_to)
rspec/spec_file_path_format: EnforcedInflector (default=default)
style/access_modifier_declarations: EnforcedStyle (default=group)
style/accessor_grouping: EnforcedStyle (default=grouped)
style/alias: EnforcedStyle (default=prefer_alias)
style/and_or: EnforcedStyle (default=conditionals)
style/bare_percent_literals: EnforcedStyle (default=bare_percent)
style/block_delimiters: EnforcedStyle (default=line_count_based)
style/class_methods_definitions: EnforcedStyle (default=def_self)
style/command_literal: EnforcedStyle (default=backticks)
style/conditional_assignment: EnforcedStyle (default=assign_to_condition)
style/double_negation: EnforcedStyle (default=allowed_in_returns)
style/empty_class_definition: EnforcedStyle (default=class_definition)
style/empty_else: EnforcedStyle (default=both)
style/empty_string_inside_interpolation: EnforcedStyle (default=trailing_conditional)
style/endless_method: EnforcedStyle (default=allow_single_line)
style/exponential_notation: EnforcedStyle (default=scientific)
style/float_division: EnforcedStyle (default=single_coerce)
style/for_cop: EnforcedStyle (default=each)
style/format_string: EnforcedStyle (default=format)
style/format_string_token: EnforcedStyle (default=annotated)
style/hash_as_last_array_item: EnforcedStyle (default=braces)
style/hash_lookup_method: EnforcedStyle (default=brackets)
style/it_block_parameter: EnforcedStyle (default=allow_single_line)
style/lambda: EnforcedStyle (default=line_count_dependent)
style/lambda_call: EnforcedStyle (default=call)
style/magic_comment_format: EnforcedStyle (default=snake_case)
style/method_def_parentheses: EnforcedStyle (default=require_parentheses)
style/missing_else: EnforcedStyle (default=both)
style/mixin_grouping: EnforcedStyle (default=separated)
style/module_function: EnforcedStyle (default=module_function)
style/multiline_memoization: EnforcedStyle (default=keyword)
style/mutable_constant: EnforcedStyle (default=literals)
style/negated_unless: EnforcedStyle (default=both)
style/next: EnforcedStyle (default=skip_modifier_ifs)
style/nil_comparison: EnforcedStyle (default=predicate)
style/numbered_parameters: EnforcedStyle (default=allow_single_line)
style/numeric_literal_prefix: EnforcedOctalStyle (default=zero_with_o)
style/numeric_predicate: EnforcedStyle (default=predicate)
style/object_then: EnforcedStyle (default=then)
style/percent_q_literals: EnforcedStyle (default=lower_case_q)
style/preferred_hash_methods: EnforcedStyle (default=short)
style/quoted_symbols: EnforcedStyle (default=same_as_string_literals)
style/regexp_literal: EnforcedStyle (default=slashes)
style/return_nil: EnforcedStyle (default=return)
style/string_literals_in_interpolation: EnforcedStyle (default=single_quotes)
style/trailing_comma_in_hash_literal: EnforcedStyleForMultiline (default=no_comma)
style/unless_logical_operators: EnforcedStyle (default=forbid_mixed_logical_operators)
```
