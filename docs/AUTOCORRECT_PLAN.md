# Autocorrect Implementation Plan

## Overview

turbocop currently only detects offenses — it cannot fix them. RuboCop's `-a` (safe autocorrect) and `-A` (all autocorrect) flags are among its most-used features. This document covers the architecture, difficulty assessment, phased implementation roadmap, and conformance testing strategy for adding autocorrect to turbocop.

For the complete catalog of every autocorrectable cop (664 cops across 4 gems, with safety classifications and extraction scripts), see **[AUTOCORRECT_COPS.md](AUTOCORRECT_COPS.md)**.

### Scope

| Gem | Total Cops | Autocorrectable | % |
|-----|-----------|----------------|---|
| rubocop (core) | 593 | 458 | 77% |
| rubocop-performance | 52 | 45 | 87% |
| rubocop-rails | 148 | 101 | 68% |
| rubocop-rspec | 114 | 60 | 53% |
| **Total** | **907** | **664** | **73%** |

---

## 1. How RuboCop's Autocorrect Works

### 1.1 Corrector

Each cop gets a `Corrector` instance (extends `Parser::Source::TreeRewriter`) with operations: `replace`, `remove`, `insert_before`, `insert_after`, `wrap`, `swap`.

### 1.2 add_offense with Correction Block

```ruby
add_offense(range, message: 'Prefer single quotes') do |corrector|
  corrector.replace(range, "'hello'")
end
```

The block is only evaluated if autocorrect is requested. Cops must `extend AutoCorrector`.

### 1.3 Safety Model

| Flag | Behavior |
|------|----------|
| `-a` / `--autocorrect` | Safe only: skips cops with `SafeAutoCorrect: false` or `Safe: false` |
| `-A` / `--autocorrect-all` | All corrections, including unsafe ones |

Per-cop config keys: `AutoCorrect` (`'always'`/`'contextual'`/`'disabled'`), `Safe` (bool), `SafeAutoCorrect` (bool).

### 1.4 Correction Merging and Conflicts

Each cop produces corrections independently. RuboCop merges them per-file using these rules:

- The `Corrector` (extends `Parser::Source::TreeRewriter`) is configured with:
  - `different_replacements: :raise` — raises `ClobberingError` if two corrections target the same range with different replacements
  - `swallowed_insertions: :raise` — raises if an insertion would be deleted by another edit
  - `crossing_deletions: :accept` — overlapping deletions are allowed (merged)
- `ClobberingError` is caught and silently ignored in `Team#suppress_clobbering` — the conflicting correction is simply dropped
- Corrections merge in **cop registration order** — first cop merged wins when there's a conflict
- Cops can declare `autocorrect_incompatible_with` — after a cop's correction is merged, all cops it declares incompatible are **skipped** for that iteration (not just for overlapping ranges — skipped entirely)

### 1.5 Iteration Loop

Re-inspects after applying corrections, up to **200 iterations**, until source stabilizes. Detects infinite loops via source checksum comparison.

### 1.6 File Writing

Corrected source written to disk, or to stdout with `--stdin`.

---

## 2. Difficulty Assessment

### 2.1 Overall Difficulty: Moderate-High

The autocorrect system has two distinct layers of difficulty:

**Infrastructure (the hard part):** The correction framework, iteration loop, safety model, and walker integration touch the core linting pipeline. This is architecturally involved because it interacts with rayon parallelism, the `!Send` `ParseResult` constraint, the config system, the formatter, and the cache. However, the design is well-understood — we're replicating RuboCop's proven architecture, not inventing something new.

**Per-cop corrections (the long tail):** Each cop needs its own correction logic. Difficulty varies wildly — from trivial (delete trailing whitespace) to genuinely hard (restructure `if`/`unless` with proper precedence handling). The good news: each cop is independent, so they can be added incrementally without blocking anything.

### 2.2 Difficulty by Component

| Component | Difficulty | Why |
|-----------|-----------|-----|
| `Correction` + `CorrectionSet` data types | **Easy** | Simple structs, well-defined algorithm (sorted non-overlapping edits). Mostly unit-testable in isolation. |
| `SourceFile::line_col_to_offset()` | **Easy** | Inverse of existing `offset_to_line_col`. The `line_starts` array already exists. |
| CLI flags (`-a`, `-A`) | **Easy** | Clap args + enum. Trivial. |
| `CopConfig` extension (Safe/SafeAutoCorrect/AutoCorrect) | **Easy** | Reading 3 more YAML keys. Config system already handles arbitrary keys via `options` HashMap. |
| `Diagnostic.corrected` field | **Easy** | Add one bool field, update Display impl and JSON serializer. |
| Cop trait `corrections` parameter | **Easy** | Add `Option<&mut Vec<Correction>>` param to existing `check_*` methods. All 915 cops get a mechanical signature update; no behavioral change. |
| `BatchedCopWalker` corrections buffer | **Moderate** | Pass `Option<&mut Vec<Correction>>` through to cop dispatch. Must not regress the hot path when autocorrect is off (`None` path). |
| Linter iteration loop | **Moderate** | Re-parse + re-lint per iteration inside rayon workers. Must handle convergence detection, max iterations, and the `!Send` ParseResult constraint. Conceptually straightforward but touches the most performance-sensitive code. |
| File writing | **Easy-Moderate** | `std::fs::write` for the simple case. `--stdin` mode needs stdout output. Edge cases: file permissions, symlinks, encoding. |
| Formatter changes | **Easy** | `[Corrected]` prefix, corrected count. Mechanical. |
| Cache interaction | **Easy** | Disable cache when autocorrect active. One conditional. |
| Correction conflict resolution | **Moderate** | Overlapping edits need deterministic resolution. Sorting by offset + dropping overlaps is simple in theory; edge cases around insertions at the same point need care. |
| Conformance testing harness | **Moderate-Hard** | Copying bench repos, running both tools, diffing output trees, reporting per-cop match rates. Not algorithmically hard, but lots of plumbing. |

### 2.3 Difficulty by Cop Category

| Category | Count | Difficulty | Example |
|----------|-------|-----------|---------|
| Byte-range deletion | ~15 | **Trivial** | `Layout/TrailingWhitespace`: delete bytes from offset A to B |
| Byte-range insertion | ~5 | **Trivial** | `Style/FrozenStringLiteralComment`: insert string at offset 0 |
| Simple token replacement | ~40 | **Easy** | `Style/StringLiterals`: replace `"` with `'`; `Lint/UnifiedInteger`: replace `Fixnum` with `Integer` |
| AST node replacement | ~80 | **Easy-Moderate** | `Style/NilComparison`: replace `x == nil` with `x.nil?` — need node byte ranges |
| Multi-edit corrections | ~50 | **Moderate** | `Style/SymbolProc`: delete block, insert `(&:method)` — two edits that must be coordinated |
| Structural transforms | ~30 | **Moderate-Hard** | `Style/IfUnlessModifier`: rewrite multi-line if to modifier form — need to understand indentation context |
| Operator precedence-aware | ~10 | **Hard** | `Style/AndOr`: `and`/`or` → `&&`/`||` with parenthesization when precedence changes semantics |
| Layout/indentation cops | ~99 | **Hard** | The entire Layout department. Each cop's correction is conceptually simple (add/remove spaces) but must match RuboCop's exact indentation calculation, which involves context-dependent rules. Getting 100% conformance here is the hardest part of the entire project. |
| Config-dependent behavior | ~40 | **Moderate** | Cops with `EnforcedStyle` that changes the correction direction (e.g., `Style/HashSyntax` with `ruby19` vs `hash_rockets`) |

### 2.4 What Makes This Tractable

1. **Incremental.** Infrastructure lands first, then cops are added one at a time. Each cop is independently useful.
2. **No new algorithms.** We're replicating RuboCop's well-documented behavior, not designing from scratch.
3. **Existing detection logic.** Every cop already identifies the offense location. The correction is usually "the obvious fix" for what was detected.
4. **Byte offsets from Prism.** Prism nodes provide `start_offset()` / `end_offset()`, so we don't need a separate range-mapping layer.
5. **Conformance testing.** We can measure correctness against RuboCop at every step.

### 2.5 What Makes This Hard

1. **Layout cops.** RuboCop's indentation/spacing corrections are the most complex autocorrections and account for 99 cops. Matching their behavior exactly requires understanding RuboCop's indentation calculation internals.
2. **Multi-cop interaction.** When multiple cops correct the same file, corrections can conflict or cascade. RuboCop handles this with its iteration loop, which we must replicate exactly.
3. **The long tail.** 664 cops is a lot. Even at 10 cops/day, full coverage is months of work. Prioritization matters.
4. **Performance.** The iteration loop re-parses files multiple times. With rayon this should still be fast, but it's a meaningful change to the hot path.

---

## 3. Architecture Design

### 3.1 Correction Data Structure

```rust
/// A single source-level edit: replace byte range [start..end) with replacement.
#[derive(Debug, Clone)]
pub struct Correction {
    pub start: usize,        // byte offset, inclusive
    pub end: usize,          // byte offset, exclusive
    pub replacement: String, // replacement text (empty = delete)
    pub cop_name: &'static str,
}
```

**Design decision: byte offsets.** Prism provides `start_offset()`/`end_offset()`. Avoids lossy line:col round-trips.

### 3.2 CorrectionSet

```rust
pub struct CorrectionSet {
    corrections: Vec<Correction>, // sorted by start, non-overlapping
}
```

Apply algorithm — single O(n) linear scan:
```
cursor = 0
for each correction c (sorted by start):
    copy source[cursor..c.start]
    copy c.replacement
    cursor = c.end
copy source[cursor..]
```

**Design decision: sorted-edits, not tree-rewriter.** Simpler than RuboCop's `TreeRewriter` since we collect all corrections upfront.

**Conflict resolution rules** (matching RuboCop's "first merged wins" behavior):
- Sort corrections by `start` offset ascending
- When two corrections overlap (second's `start` < first's `end`), drop the second
- When two corrections start at the same offset, the one from the earlier cop in registry order wins
- Registry order is stable (deterministic cop registration in `default_registry()`)

### 3.3 Cop Trait Extension

```rust
pub trait Cop: Send + Sync {
    // existing methods unchanged...

    fn supports_autocorrect(&self) -> bool { false }
    fn safe_autocorrect(&self) -> bool { true }

    fn check_lines(
        &self,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<Correction>>,  // NEW param
    );
    fn check_source(
        &self, ...,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<Correction>>,  // NEW param
    );
    fn check_node(
        &self, ...,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<Correction>>,  // NEW param
    );
}
```

**Why `Option` parameter instead of dual methods?** Adding `*_with_corrections` variants would mean 3 new trait methods with 915 default impls that just delegate. Instead, we add one `Option<&mut Vec<Correction>>` parameter to the existing methods. Callers pass `None` when autocorrect is off — cops that don't support autocorrect simply ignore the parameter. This keeps the trait surface flat and avoids duplicating every walker call site.

### 3.4 Other Changes

- **Diagnostic**: Add `corrected: bool` field
- **CLI**: `-a`/`-A` flags, `AutocorrectMode` enum
- **CopConfig**: Read `Safe`, `SafeAutoCorrect`, `AutoCorrect` keys
- **BatchedCopWalker**: Add `corrections: Vec<Correction>` buffer, conditional dispatch
- **Linter**: Iteration loop (max 200), file writing
- **Formatters**: `[Corrected]` prefix, corrected count summary, JSON fields
- **Cache**: Disabled when autocorrect active

### 3.5 Linter Iteration Loop

```
for each file (parallel via rayon):
    source = read file
    loop (max 200 iterations):
        (diagnostics, corrections) = lint_with_corrections(source)
        if corrections.is_empty(): break
        source = corrections.apply(source)
        if source unchanged: break
    write corrected source to disk
    collect remaining diagnostics
```

Files are independent — rayon parallelism at file level. Per-file iterations are sequential. `ParseResult` being `!Send` is fine since we re-parse inside the same rayon worker.

---

## 4. Phased Implementation Roadmap

### Phase 0: Infrastructure ✅ COMPLETE

**Goal:** The autocorrect framework works end-to-end with zero cops. Running `turbocop -a .` accepts the flag, applies no corrections (since no cop implements `supports_autocorrect`), and exits cleanly.

**Status:** Implemented in commit `b370fa5`. 935 files changed (1507 insertions, 145 deletions). All tests pass.

| File | Change | Status |
|------|--------|--------|
| `src/correction.rs` | **NEW**: `Correction`, `CorrectionSet` + 13 unit tests | ✅ |
| `src/diagnostic.rs` | Add `corrected: bool` with `#[serde(default)]` | ✅ |
| `src/cop/mod.rs` | Add `supports_autocorrect`, `safe_autocorrect`; add `corrections` param; add config helpers (`is_safe`, `is_safe_autocorrect`, `autocorrect_setting`, `should_autocorrect`) | ✅ |
| `src/cop/**/*.rs` | All 915 cop signatures updated with `_corrections` parameter | ✅ |
| `src/cli.rs` | Add `-a`, `-A` flags, `AutocorrectMode` enum | ✅ |
| `src/cop/walker.rs` | Pass `None` through `CopWalker` and `BatchedCopWalker` dispatch | ✅ |
| `src/cop/mod.rs` (CopConfig) | Read `Safe`, `SafeAutoCorrect`, `AutoCorrect` keys | ✅ |
| `src/linter.rs` | `corrected_count` in `LintResult`, cache disabled during autocorrect | ✅ |
| `src/lib.rs` | Autocorrect mode debug output, stdin+autocorrect warning | ✅ |
| `src/formatter/text.rs` | Corrected count in summary when > 0 | ✅ |
| `src/formatter/progress.rs` | Corrected count in summary when > 0 | ✅ |
| `src/formatter/pacman.rs` | Corrected count in summary when > 0 | ✅ |
| `src/formatter/json.rs` | `corrected` field on offenses, `corrected_count` in metadata | ✅ |
| `src/parse/source.rs` | `line_col_to_offset()` with roundtrip proptest | ✅ |
| `src/testutil.rs` | Updated all call sites with `None` | ✅ |
| `src/cache.rs` | Updated `test_args()` and `to_diagnostic()` | ✅ |
| `tests/integration.rs` | Updated `default_args()` and Diagnostic literals | ✅ |

**Not yet implemented** (deferred to when first cops produce corrections):
- Linter iteration loop (re-parse + re-lint until convergence)
- File writing after correction
- `BatchedCopWalker` corrections buffer (currently passes `None`; will pass `Some(&mut vec)` when autocorrect is active)

### Phase 1: Trivial Cops (byte-range deletions and insertions)

**Goal:** First cops that actually correct files. Validates the full pipeline: detection → correction → file write → re-lint convergence.

**Difficulty: Easy.** Each cop is 5-15 lines of correction code on top of existing detection logic.

| Cop | Correction | Notes |
|-----|-----------|-------|
| `Layout/TrailingWhitespace` | Delete trailing whitespace bytes | Simplest possible: `Correction::remove(start, end)` |
| `Layout/TrailingEmptyLines` | Add/remove trailing newlines | Slightly tricky: may need to insert `\n` or remove multiple |
| `Layout/LeadingEmptyLines` | Remove leading blank lines | Delete from offset 0 |
| `Style/FrozenStringLiteralComment` | Insert `# frozen_string_literal: true\n` at top | `Correction::insert_before(0, ...)` |
| `Layout/IndentationStyle` | Replace tabs with spaces | Per-line byte replacement |
| `Style/Encoding` | Remove/add encoding comment | Line-level insert/delete |

**Verification:** `corrected.rb` fixtures for each cop. Manual test on a real repo.

### Phase 2: Simple Token Replacements

**Goal:** Cops that replace one token/string with another. Builds confidence in AST-based corrections.

**Difficulty: Easy.** Simple `Correction::replace(node.start_offset(), node.end_offset(), "new_text")`.

| Cop | Correction |
|-----|-----------|
| `Style/StringLiterals` | Replace `"str"` with `'str'` (when no interpolation) |
| `Style/StringLiteralsInInterpolation` | Replace quote style inside `#{}` |
| `Lint/UnifiedInteger` | `Fixnum`/`Bignum` → `Integer` |
| `Style/NilComparison` | `x == nil` → `x.nil?` |
| `Style/Not` | `not x` → `!x` |
| `Style/ColonMethodCall` | `Foo::bar` → `Foo.bar` |
| `Style/DefWithParentheses` | Remove empty parens from `def foo()` |
| `Style/EmptyLiteral` | `Array.new` → `[]`, `Hash.new` → `{}` |
| `Style/Proc` | `Proc.new` → `proc` |
| `Style/Attr` | `attr` → `attr_reader` |
| `Style/RedundantSelf` | Remove `self.` prefix |
| `Lint/RedundantStringCoercion` | Remove `.to_s` in interpolation |
| `Style/SymbolLiteral` | Remove unnecessary symbol quotes |
| `Style/CharacterLiteral` | `?c` → `'c'` |

**Verification:** `corrected.rb` fixtures. Run on bench repos, diff against RuboCop.

### Phase 3: Conformance Testing Harness

**Goal:** Automated way to measure how well turbocop's autocorrect matches RuboCop, per-cop and per-file.

**Difficulty: Moderate.** Not algorithmically hard, but substantial plumbing (temp dirs, running both tools, diffing, reporting).

- Add `autocorrect-conform` subcommand to `bench_turbocop`
- For each bench repo: copy to two temp dirs, run `rubocop -A` on one and `turbocop -A` on the other, diff results
- Report per-cop match rate and overall file-level match rate
- Integrate into the conformance pipeline alongside existing detection conformance

**Verification:** Running `cargo run --release --bin bench_turbocop -- autocorrect-conform` produces a report.

### Phase 4: AST-Based Corrections (Moderate)

**Goal:** Cops that require understanding AST structure for correct replacement. Covers the high-value middle ground.

**Difficulty: Easy-Moderate.** Need node byte ranges and sometimes parent context.

| Cop | Correction |
|-----|-----------|
| `Style/SymbolProc` | `{ \|x\| x.foo }` → `(&:foo)` |
| `Style/NegatedIf` / `NegatedUnless` | Flip condition, swap if/unless keyword |
| `Style/AndOr` | `and`/`or` → `&&`/`\|\|` (with parenthesization) |
| `Style/HashSyntax` | Convert between `ruby19`/`hash_rockets` styles |
| `Style/BlockDelimiters` | Switch between `do..end` and `{..}` |
| `Style/Lambda` | `lambda { }` ↔ `-> { }` |
| `Style/RedundantReturn` | Remove `return` keyword |
| `Style/RedundantBegin` | Remove redundant `begin..end` |
| `Style/RedundantParentheses` | Remove unnecessary parens |
| `Style/WhenThen` | `when x then` → `when x\n` |
| `Style/Semicolon` | Remove unnecessary semicolons |

**Verification:** `corrected.rb` fixtures + autocorrect-conform runs.

### Phase 5: Layout Spacing Cops

**Goal:** The ~30 most common spacing cops. This is the highest-impact but hardest-to-match category.

**Difficulty: Moderate-Hard.** Each individual correction is "insert/remove a space", but matching RuboCop's exact behavior across all edge cases is challenging. RuboCop's layout corrections account for the majority of real-world autocorrect usage.

| Category | Cops | Approach |
|----------|------|----------|
| Space around operators | `SpaceAroundOperators`, `SpaceAroundKeyword`, etc. | Insert/remove spaces at known byte offsets |
| Space inside brackets | `SpaceInsideBlockBraces`, `SpaceInsideHashLiteralBraces`, `SpaceInsideParens` | Insert/remove spaces after open / before close |
| Empty lines | `EmptyLines`, `EmptyLineBetweenDefs`, `EmptyLinesAround*` | Insert/delete newline bytes |
| Indentation | `IndentationWidth`, `IndentationConsistency` | Rewrite leading whitespace — context-dependent |

**Verification:** Autocorrect-conform runs are critical here. Layout cops are where conformance divergence is most likely.

### Phase 6: Long Tail + Full Conformance

**Goal:** Remaining cops, driven by conformance data. Prioritize cops that cause the most conformance failures in bench repos.

**Difficulty: Varies.** Some are trivial cops that just weren't prioritized earlier. Some are genuinely complex (e.g., `Style/IfUnlessModifier`, `Style/ConditionalAssignment`, `Style/GuardClause`).

- Use autocorrect-conform data to identify highest-impact missing corrections
- Implement `autocorrect_incompatible_with` mechanism for cop conflicts
- Target specific conformance rates per bench repo
- Consider `--disable-uncorrectable` support

---

## 5. Conformance Testing Strategy

### 5.1 Unit Tests: corrected.rb Fixtures

Extend the existing fixture format:

```
testdata/cops/style/string_literals/
  offense.rb       # source with ^ annotations (existing)
  no_offense.rb    # clean source (existing)
  corrected.rb     # expected output after autocorrect (NEW)
```

New macro `cop_autocorrect_fixture_tests!` in `src/cop/mod.rs`:
- Strips `^` annotations from `offense.rb` to get clean source
- Runs the cop in autocorrect mode (`corrections: Some(&mut vec)`)
- Applies `CorrectionSet` to produce corrected source
- Asserts output matches `corrected.rb` byte-for-byte

If `corrected.rb` doesn't exist, the macro silently skips (backward-compatible). This means autocorrect tests are opt-in per cop — just add `corrected.rb`.

### 5.2 Integration Tests: autocorrect-conform

**Single-cop isolation approach.** Rather than comparing full `turbocop -A` vs `rubocop -A` output (which conflates all cops and makes it hard to attribute failures), the harness tests autocorrect conformance **one cop at a time**:

```bash
cargo run --release --bin bench_turbocop -- autocorrect-conform
```

For each bench repo, for each cop that `supports_autocorrect`:

1. Copy the repo to a temp directory
2. Run `rubocop -A --only Department/CopName --format json` on the copy
3. Record which files were corrected and the corrected content
4. Reset the copy (restore originals)
5. Run `turbocop -A --only Department/CopName` on the same copy
6. Diff the corrected files

**Why single-cop?**
- No conflict resolution noise — isolates each cop's behavior
- Easy to attribute failures to specific cops
- Matches how conformance testing works for detection (per-cop stats)
- Slow but thorough — can run overnight, cache results

**Output:** `bench/autocorrect_conform.json` with per-cop, per-repo stats:
```json
{
  "mastodon": {
    "Style/StringLiterals": {
      "files_corrected_rubocop": 42,
      "files_corrected_turbocop": 42,
      "files_match": 41,
      "files_differ": 1,
      "match_rate": 97.6
    }
  }
}
```

### 5.3 Golden-File Tests (from RuboCop specs)

For each autocorrectable cop, extract `expect_correction` blocks from vendor RuboCop specs:

```
testdata/cops/style/string_literals/
  corrected.rb     # derived from vendor spec expect_correction blocks
```

RuboCop specs contain explicit before/after pairs:
```ruby
expect_offense(<<~RUBY)
  x = "hello"
      ^^^^^^^ Prefer single-quoted strings
RUBY

expect_correction(<<~RUBY)
  x = 'hello'
RUBY
```

Extraction process:
1. Parse vendor spec for `expect_correction` blocks
2. Combine with corresponding `expect_offense` blocks to get input → expected output pairs
3. Write as `corrected.rb` (the expected output after correction)

This gives us RuboCop's own test expectations as our ground truth.

### 5.4 Conflict Resolution Tests

Test that overlapping corrections from multiple cops are handled identically to RuboCop:
- Files that trigger corrections from 2+ cops on overlapping ranges
- Verify the same corrections win/lose in both tools
- Multi-cop golden-file tests in `testdata/autocorrect/multi_cop/` with input.rb + expected.rb

---

## 6. Open Questions

1. **Iteration limit**: Match RuboCop's 200, or start lower (e.g., 10) as a safety measure?

2. **Atomic writes**: Write corrected files atomically (temp + rename)? RuboCop doesn't, but it's safer.

3. **`--disable-uncorrectable`**: Defer to a later phase? It adds `# rubocop:todo` comments, which is useful but orthogonal to the core autocorrect flow.

4. **Priority ordering**: When bench conformance data is available, should we prioritize (a) most-triggered cops in bench repos, (b) simplest corrections, or (c) highest conformance impact? Likely (c).

5. ~~**Layout cop strategy**~~: Resolved — implement from scratch per Phase 5. The phased approach allows measuring conformance incrementally and deprioritizing cops where RuboCop's behavior is too complex to match exactly.

---

## 7. Files Modified Summary

| File | Change |
|------|--------|
| `src/correction.rs` | **NEW**: `Correction`, `CorrectionSet` types |
| `src/diagnostic.rs` | Add `corrected: bool` field |
| `src/cop/mod.rs` | Add `supports_autocorrect`, `safe_autocorrect`; add `corrections: Option<&mut Vec<Correction>>` param to `check_*` methods |
| `src/cop/walker.rs` | Pass `Option<&mut Vec<Correction>>` through `BatchedCopWalker` dispatch |
| `src/cli.rs` | Add `-a`, `-A` flags, `AutocorrectMode` |
| `src/config/mod.rs` | Read `Safe`, `SafeAutoCorrect`, `AutoCorrect` |
| `src/linter.rs` | Correction-aware lint path, iteration loop, file writing |
| `src/lib.rs` | Branch on autocorrect mode |
| `src/formatter/text.rs` | `[Corrected]` prefix, corrected count |
| `src/formatter/json.rs` | `corrected`/`correctable` fields |
| `src/parse/source.rs` | `line_col_to_offset()` helper |
| Per-cop files | Use `corrections` param in `check_*` methods to produce `Correction` values |
