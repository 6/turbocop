# Lint Cleanup Plan

> Get `cargo clippy --release -- -D warnings` passing with zero `#![allow]` suppressions.
>
> Current state: 60 crate-level `#![allow(clippy::...)]` directives in `src/lib.rs` +
> 5 in `bench/bench.rs`, 2 in binaries. **528 warnings** across **300 files** when allows
> are removed. All introduced by Rust 1.93 stable (clippy added new default lints).

## Inventory

### Mechanical fixes (452 warnings, 87% — safe to apply without review)

These are rote transformations where clippy's suggestion is always correct.

| Count | Lint | Fix |
|------:|------|-----|
| 82 | `manual_contains` | `iter().any(\|x\| x == y)` → `.contains(&y)` |
| 73 | `unnecessary_map_or` | Simplify `.map_or(default, f)` patterns |
| 51 | `collapsible_if` | Merge `if a { if b { } }` → `if a && b { }` |
| 36 | `needless_borrow` | Remove unnecessary `&` borrows |
| 32 | `needless_return` | Remove explicit `return` at end of function |
| 26 | `needless_borrows_for_generic_args` | Remove `&` on args that impl the trait |
| 22 | `string_from_utf8_as_bytes` | `from_utf8(x.as_bytes())` → use `x` directly |
| 17 | `useless_asref` | Remove no-op `.as_ref()` calls |
| 16 | `manual_map` | `match opt { Some(x) => Some(f(x)), None => None }` → `.map(f)` |
| 13 | `borrow_deref_ref` | Remove redundant `&*` patterns |
| 13 | `manual_strip` | Use `strip_prefix`/`strip_suffix` |
| 9 | `useless_conversion` | Remove `.into()` on same type |
| 6 | `collapsible_else_if` | `else { if }` → `else if` |
| 6 | `manual_pattern_char_comparison` | Use char pattern instead of closure |
| 6 | `needless_range_loop` | `for i in 0..v.len()` → `for item in &v` |
| 5 | `byte_char_slices` | `&[b'\r']` → `b"\r"` |
| 5 | `for_kv_map` | `.iter()` → `.values()` / `.keys()` |
| 3 | `question_mark` | `match opt { None => return None, .. }` → `opt?` |
| 3 | `needless_bool` | `if cond { true } else { false }` → `cond` |
| 3 | `unnecessary_unwrap` | Remove `.unwrap()` after `.is_some()` guard |
| 2 | `wildcard_in_or_patterns` | Remove `_` from or-patterns |
| 2 | `useless_format` | `format!("literal")` → `"literal".to_string()` |
| 2 | `manual_range_contains` | Use `(a..b).contains(&x)` |
| 2 | `redundant_closure` | `\|x\| f(x)` → `f` |
| 1 each | `int_plus_one`, `redundant_field_names`, `unnecessary_filter_map`, `unnecessary_sort_by`, `needless_option_as_deref`, `needless_bool_assign`, `unused_enumerate_index`, `manual_ignore_case_cmp`, `manual_filter`, `manual_flatten`, `while_let_loop`, `manual_unwrap_or_default`, `manual_unwrap_or`, `map_identity`, `no_effect_replace`, `collapsible_str_replace`, `match_single_binding`, `print_literal` | Various one-off mechanical fixes |

### Needs-review fixes (43 warnings, 8% — safe but verify intent)

| Count | Lint | What to check |
|------:|------|---------------|
| 18 | `if_same_then_else` | Both branches do the same thing — may be copy-paste bug or intentional for clarity. Review each: collapse if safe, add comment if intentional. |
| 6 | `map_entry` | Use `entry()` API on HashMap instead of `contains_key` + `insert`. |
| 6 | `too_many_arguments` | Functions with >7 args. Options: introduce a params struct, or add local `#[allow]` with a comment explaining why. |
| 6 | `nonminimal_bool` | Boolean expression can be simplified. Verify the simplification preserves intent. |
| 3 | `type_complexity` | Type is very long. Add a `type Alias = ...` or local `#[allow]`. |
| 1 | `collapsible_match` | Nested match → single match. Verify readability. |
| 1 | `derivable_impls` | Default impl can use `#[derive(Default)]`. |
| 1 | `only_used_in_recursion` | Parameter only forwarded to recursive call — may be a bug. Investigate. |
| 1 | `single_match` | `match` with one arm + wildcard → `if let`. |

### Keep as `#[allow]` (8 warnings — intentional or not worth changing)

These are the **only** lints that should remain suppressed, and they should use **local** `#[allow]` on the specific item, not crate-level.

| Count | Lint | Reason |
|------:|------|--------|
| 2 | `new_without_default` | Adding `Default` trait would change the public API contract. |
| 2 | `doc_lazy_continuation` | Doc formatting, not worth restructuring comments. |
| 2 | `never_loop` | Loop with early-return pattern is intentional for control flow. |
| 1 | `should_implement_trait` | `from_str` naming — not intended to be `FromStr` trait impl. |
| 1 | `enum_variant_names` | Variant prefix is intentional (e.g. `NeedsFoo`, `NeedsBar`). |
| 1 | `needless_lifetimes` | Explicit lifetime aids readability in this context. |
| 1 | `cloned_ref_to_slice_refs` | New 1.93 lint, suggestion is less readable. |

### Dead code (23 warnings — clean up)

| Type | Action |
|------|--------|
| Unused imports | Delete them (already fixed 3 in initial commit). |
| Unused variables | Prefix with `_` if intentionally unused, otherwise delete. |
| Unused functions/structs/fields | Delete if truly dead. If kept for future use, add `#[allow(dead_code)]` locally with a `// TODO: used by <future feature>` comment. |

## Execution plan

Work in **5 batches**, one commit per batch. Each batch removes the relevant `#![allow]` lines from `lib.rs` (and bin crates) so CI enforces they stay fixed.

### Batch 1: Mechanical — borrow/reference lints (75 fixes)

Remove allows for: `needless_borrow`, `needless_borrows_for_generic_args`, `borrow_deref_ref`, `useless_asref`, `useless_conversion`, `redundant_closure`, `cloned_ref_to_slice_refs`.

These are the safest — just removing unnecessary `&`, `*`, `.as_ref()`, `.into()`.

### Batch 2: Mechanical — control flow lints (110 fixes)

Remove allows for: `collapsible_if`, `collapsible_else_if`, `collapsible_str_replace`, `needless_return`, `manual_map`, `manual_filter`, `manual_flatten`, `manual_strip`, `manual_unwrap_or`, `manual_unwrap_or_default`, `manual_range_contains`, `question_mark`, `while_let_loop`, `match_single_binding`, `needless_bool`, `needless_bool_assign`, `wildcard_in_or_patterns`, `int_plus_one`, `collapsible_match`, `single_match`.

### Batch 3: Mechanical — idiom lints (185 fixes)

Remove allows for: `manual_contains`, `unnecessary_map_or`, `string_from_utf8_as_bytes`, `byte_char_slices`, `for_kv_map`, `needless_range_loop`, `manual_pattern_char_comparison`, `useless_format`, `unused_enumerate_index`, `manual_ignore_case_cmp`, `map_identity`, `no_effect_replace`, `print_literal`, `redundant_field_names`, `unnecessary_filter_map`, `unnecessary_sort_by`, `needless_option_as_deref`, `unnecessary_unwrap`.

### Batch 4: Review — logic and structure lints (43 fixes)

Remove allows for: `if_same_then_else`, `nonminimal_bool`, `map_entry`, `too_many_arguments`, `type_complexity`, `derivable_impls`, `only_used_in_recursion`.

Each fix needs human review. For `too_many_arguments` and `type_complexity`, use local `#[allow]` where refactoring isn't warranted.

### Batch 5: Cleanup — dead code + local allows (31 fixes)

- Remove `#![allow(dead_code, unused_variables, unused_assignments)]` from `lib.rs`.
- Delete genuinely dead code.
- Prefix intentionally-unused variables with `_`.
- Move remaining 8 intentional suppressions to **local** `#[allow(...)]` on the specific items.
- Remove all `#![allow]` from `bench/bench.rs`, `src/bin/coverage_table.rs`, `src/bin/node_pattern_codegen.rs` (fix or localize their lints too).

## End state

- `src/lib.rs`: zero `#![allow]` directives
- `bench/bench.rs`, bins: zero `#![allow]` directives
- `cargo clippy --release -- -D warnings`: passes clean
- ~8 **local** `#[allow(clippy::...)]` with comments, on specific items only

## Verification

After each batch:

```
cargo fmt -- --check
cargo clippy --release -- -D warnings
cargo test --release
```
