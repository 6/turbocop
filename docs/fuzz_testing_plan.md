# Fuzz Testing Plan

## Motivation

In April 2026, a single UTF-8 boundary bug in `Style/DocumentationMethod` caused 139 corpus repos (including rails/rails) to panic, inflating FN by ~4.1M and dropping conformance from 88% to 5.6%. The bug was a `text[..kw.len()]` slice landing inside a multi-byte character. Fuzz testing would have caught this in seconds.

## What to Fuzz

### Priority 1: Comment/string parsers in cops

Any cop that indexes into source text by byte offset is vulnerable. Key targets:

- `is_annotation_comment()` in `style/documentation_method.rs` (the bug that motivated this)
- `Lint/RedundantCopDisableDirective` directive parsing in `linter.rs`
- Any cop using `text[..n]`, `&text[n..]`, or manual byte-offset slicing on user content

Inputs: arbitrary UTF-8 strings with mixed ASCII, multi-byte (2-4 byte) characters, and emoji.

### Priority 2: Config/YAML parsing

Feed malformed or adversarial `.rubocop.yml` content to config resolution. Target: crash-free behavior on any input.

### Priority 3: Full-file parsing pipeline

Feed arbitrary `.rb` file content through the full `check_source` path for each cop. This catches panics from unexpected AST shapes, not just string handling.

## Approach

Use [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz) with `libFuzzer`:

```bash
cargo install cargo-fuzz
cargo fuzz init
```

### Example fuzz target for comment annotation scanning

```rust
// fuzz/fuzz_targets/annotation_comment.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    // Call the annotation comment checker with arbitrary UTF-8 input.
    // This should never panic regardless of input.
    let _ = nitrocop::cop::style::documentation_method::is_annotation_comment(data);
});
```

### Example fuzz target for full cop pipeline

```rust
// fuzz/fuzz_targets/cop_check_source.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    // Parse as Ruby and run all cops. Should never panic.
    let _ = nitrocop::check_source(data, &default_config());
});
```

## Running

```bash
# Run a specific target for 5 minutes
cargo fuzz run annotation_comment -- -max_total_time=300

# Run with a corpus of real-world Ruby comments as seeds
cargo fuzz run annotation_comment corpus/comments/
```

## CI Integration

Consider a nightly or weekly CI job that runs fuzz targets for a bounded time (e.g., 10 minutes each). Any crash artifacts should be converted into regression test fixtures.

## Rules for New Cops

When a cop does manual string slicing or byte-offset arithmetic on source text:

1. Use `str::get()` instead of direct indexing (`text[..n]`)
2. Add the relevant function as a fuzz target
3. Seed the fuzzer with multi-byte UTF-8 strings: `"Δ"`, `"←"`, `"🦀"`, `"日本語"`
