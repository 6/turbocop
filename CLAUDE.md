# rblint

Fast Ruby linter in Rust targeting RuboCop compatibility. Uses Prism (ruby-prism crate) for parsing, rayon for parallelism.

## Setup

```
git submodule update --init    # fetch vendor/rubocop (reference specs)
```

## Commands

```
cargo check          # fast compile check
cargo build          # full build (includes Prism C FFI)
cargo test           # run all tests
cargo run -- .       # lint current directory
cargo run -- --format json .
cargo run -- --debug .
```

## Architecture

- `src/diagnostic.rs` — Severity, Location, Diagnostic types
- `src/parse/` — Prism wrapper + SourceFile (line offsets, byte→line:col)
- `src/cop/` — `Cop` trait (`check_lines`/`check_node`), `CopRegistry`, department modules (`layout/`, `style/`)
- `src/testutil.rs` — `#[cfg(test)]` fixture parser (annotations, `# rblint-expect:`, `# rblint-filename:`) + assertion helpers
- `src/config/` — `.rubocop.yml` loading (stub: no inherit_from yet)
- `src/fs/` — File discovery via `ignore` crate (.gitignore-aware)
- `src/linter.rs` — Parallel orchestration (parse per-thread since ParseResult is !Send)
- `src/formatter/` — Text (RuboCop-compatible) and JSON output
- `src/cli.rs` — Clap args
- `src/lib.rs` — `run()` wiring; `src/main.rs` — entry point

## Key Constraints

- `ruby_prism::ParseResult` is `!Send + !Sync` — parsing MUST happen inside each rayon worker thread
- Cop trait is `Send + Sync`; cops needing mutable visitor state create a temporary `Visit` struct internally
- Edition 2024 (Rust 1.85+)

## Fixture Format

Each cop has a test fixture directory under `testdata/cops/<dept>/<cop_name>/` with:

**Standard layout** (most cops): `offense.rb` + `no_offense.rb`
- Use `cop_fixture_tests!` macro in the cop's test module
- Annotate offenses with `^` markers after the offending source line:
  ```
  x = 1
       ^^ Layout/TrailingWhitespace: Trailing whitespace detected.
  ```

**Scenario layout** (cops that fire once per file or can't use `^`): `offense/` directory + `no_offense.rb`
- Use `cop_scenario_fixture_tests!` macro, listing each scenario file
- The `offense/` directory contains multiple `.rb` files, each with ≥1 offense
- Annotations across all files are summed for coverage (≥3 total required)

**Special directives** (stripped from clean source before running the cop):
- `# rblint-filename: Name.rb` — first line only; overrides the filename passed to `SourceFile` (used by `Naming/FileName`)
- `# rblint-expect: L:C Department/CopName: Message` — explicit offense at line L, column C; use when `^` can't be placed (trailing blanks, missing newlines)

## Rules

- Keep [PROGRESS.md](PROGRESS.md) up to date when completing milestone tasks. Check off items as done and update milestone status.
- See [PLAN.md](PLAN.md) for full roadmap, cop batching strategy, and technical design decisions.
- After adding a new cop, ensure `cargo test` passes — the `all_cops_have_minimum_test_coverage` integration test enforces that every cop has at least 3 offense fixture cases and 5+ non-empty lines in no_offense.rb. There are zero exemptions; use `offense/` scenario directories and `# rblint-expect:` annotations to handle cops that can't use the standard single-file format.
