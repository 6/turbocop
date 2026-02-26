# Corpus FP Investigation Notes

Findings from the 2026-02-25 `/fix-cops` session investigating high-FP cops.
The corpus oracle (run 22412389672) uses `bench/corpus/baseline_rubocop.yml`
which overrides per-repo configs — both tools get identical config. So all FPs
are genuine cop-logic differences, not config-handling issues.

## Style/DocumentationMethod — 13,884 FP (99.7% match)

**Hypothesis: visibility tracking is too weak.**

The cop at `src/cop/style/documentation_method.rs` calls `is_private_or_protected()`
(from `src/cop/util.rs`) to skip non-public methods when `RequireForNonPublicMethods: false`
(the default). nitrocop fires on 13,884 methods that RuboCop skips.

RuboCop uses `non_public?(node)` from the `DefNode` mixin, which walks up the AST
to check for `private`/`protected` visibility modifiers. Key cases RuboCop handles:

1. **Block-style visibility**: `private` on its own line, affecting all subsequent methods
2. **Inline visibility**: `private def foo` — method wrapped in a `send` node
3. **`module_function`**: Also marks methods as non-public for this cop
4. **`private_class_method`**: For `def self.foo` methods

nitrocop's `is_private_or_protected()` likely uses line-scanning heuristics rather
than AST-aware visibility tracking, which would miss edge cases like:
- Methods defined after a `private` block that's inside a nested module
- `module_function` (RuboCop's cop has a special `modifier_node?` matcher for this)
- Methods defined via `private def foo` inline syntax (Prism AST differs from Parser)

Also notable: RuboCop checks `documentation_comment?(node)` from the
`DocumentationComment` mixin, which looks for contiguous `#` comment lines above
the node. nitrocop only checks the single line immediately above `def` (line 66-73),
missing multi-line comments with blank lines or decorators between comment and def.

**To fix**: Compare `is_private_or_protected()` against RuboCop's `non_public?()`.
Add `module_function` handling. Improve comment detection to match
`DocumentationComment` mixin behavior.

**Files**: `src/cop/style/documentation_method.rs`, `src/cop/util.rs` (is_private_or_protected)

## Style/MissingElse — 5,914 FP (92.2% match, symmetric FP/FN)

**Hypothesis: cop checks wrong node type or has logic inversion.**

The cop at `src/cop/style/missing_else.rs` defaults to `EnforcedStyle: both`
(matching RuboCop). The near-symmetric FP/FN per repo (e.g., canvas-lms: 694 FP /
721 FN, discourse: 476/476) suggests the cop fires on the WRONG instances while
missing the RIGHT ones — same quantity, wrong targets.

The code logic for style branching looks correct on paper:
```rust
if self.style == "if" || self.style == "both" { /* check if */ }
if self.style == "case" || self.style == "both" { /* check case */ }
```

With `EnforcedStyle: both` (the baseline default), both blocks execute, which
should match RuboCop. The symmetry is hard to explain from style mismatch alone.

Possible root causes to investigate:
1. **Different node matching** — nitrocop might flag `if` nodes that RuboCop skips
   (e.g., ternaries, modifier ifs, `unless` with else, one-liner ifs)
2. **Missing guard clauses** — RuboCop's `on_normal_if_unless` skips ternaries and
   modifier forms; does nitrocop skip those?
3. **`case` vs `case/in` distinction** — Prism has `CaseNode` and `CaseMatchNode`;
   does nitrocop handle both? RuboCop handles `case/in` (pattern matching) separately
4. **Empty else detection** — RuboCop checks for `Style/EmptyElse` interaction;
   methods with empty `else` clauses may be handled differently

**To fix**: Pick a specific corpus repo with symmetric FP/FN (e.g., discourse at
476/476). Run both tools on a single file that has both an FP and FN. Compare the
exact offenses to see what nitrocop flags that RuboCop doesn't, and vice versa.

**Files**: `src/cop/style/missing_else.rs`

## Style/StringLiterals — 140 FP (99.9% match)

**Hypothesis: subtle string content parsing difference.**

140 FPs across only 2 repos (one_gadget: 76, jruby: 64). Earlier investigation
incorrectly attributed this to per-repo config differences, but the corpus oracle
uses the uniform baseline config.

The cop at `src/cop/style/string_literals.rs` has a `needs_double_quotes()` function
that correctly handles `\"` (treats it as convertible, which matches RuboCop's regex
`(?<!\\)\\{2}*\\(?![\\"])`). Manual testing confirmed RuboCop DOES flag strings with
`\"` for single-quote conversion.

Since the escape handling logic appears correct, the 140 FPs may come from:
1. **String interpolation detection** — strings with `#{}` need double quotes;
   maybe nitrocop misparses some strings as having interpolation when they don't
2. **Heredoc vs string confusion** — some string nodes might be heredocs that
   shouldn't be checked
3. **`?x` character literals** — single-character strings have special syntax
4. **Encoding/binary strings** — `\x` hex escapes or other encoding-specific escapes
5. **Adjacent string concatenation** — `"foo" "bar"` implicit concatenation

The one_gadget FPs are all in `lib/one_gadget/builds/*.rb` files with strings like
`"execve(\"/bin/sh\", esp+0x28, environ)"`. The jruby FPs need investigation to
see if they follow a different pattern.

**To fix**: Run nitrocop and RuboCop on a single FP file from the corpus and diff
the offense lists to find the exact strings that diverge.

**Files**: `src/cop/style/string_literals.rs`

## Style/PercentLiteralDelimiters — 1,008 FN → 3,449 FP (reverted)

**Attempted fix reverted in 310d878. Original fix was 03bd6b3.**

The cop had 1,008 FN (96.2% match) — mostly `%r(...)` patterns where the preferred
delimiter is `{}`. The root cause was identified correctly: the content-contains-preferred-
delimiter check scanned raw source bytes, so `#{...}` interpolation syntax falsely matched
`{`/`}` as preferred delimiter characters, causing the cop to suppress legitimate offenses.

The fix iterated over Prism `.parts()` for interpolated nodes and only checked `StringNode`
children (literal text segments), skipping `EmbeddedStatementsNode` children. This correctly
fixed the interpolation issue but was too aggressive overall — it went from 1,054 total
divergence (46 FP + 1,008 FN) to 3,449 FP + 0 FN. The fix produced ~3,400 new false
positives, far worse than the original state.

**Why the fix regressed**: The interpolation-aware check was necessary but insufficient.
The cop likely also needs to handle:
1. **Nested delimiters in literal content** — `%r(foo(bar))` where `()` in the content
   are balanced and part of the regex, not delimiters. RuboCop may have logic to detect
   balanced delimiter pairs within content.
2. **Escaped delimiters** — `%r(foo\(bar)` where `\(` shouldn't count as containing
   the preferred delimiter.
3. **Different percent literal types** — `%Q`, `%q`, `%s`, `%x`, `%I`, `%W` may have
   different preferred delimiter defaults than what nitrocop assumes.
4. **Config reading** — nitrocop may not be reading the `PreferredDelimiters` config
   correctly for all percent literal types.

**To fix properly**: Compare nitrocop's `PreferredDelimiters` defaults against RuboCop's.
Run both tools on a small set of FP files (e.g., from slim-template/slim which had 316
excess) to find the exact patterns that diverge. The interpolation fix is correct but
additional guards are needed before re-applying it.

**Files**: `src/cop/style/percent_literal_delimiters.rs`

## General Notes

- The corpus oracle baseline (`bench/corpus/baseline_rubocop.yml`) explicitly enables
  ALL disabled-by-default cops. It uses `--config` to override per-repo configs.
  Both tools get identical config, so FPs are pure implementation differences.
- `check-cop.py --verbose --rerun` re-runs nitrocop locally; the RuboCop baseline
  comes from CI. Discrepancies between local check-cop and CI triage numbers are
  expected when the local nitrocop binary differs from the CI build.
- The `--only CopName` flag produces a different cache session hash, so results
  may differ from full-scan mode. Always clear cache (`rm -rf ~/.cache/nitrocop`)
  before verification runs.
