# rblint

Fast Ruby linter in Rust targeting RuboCop compatibility. Uses Prism (ruby-prism crate) for parsing, rayon for parallelism.

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
- `src/cop/` — `Cop` trait (`check_lines`/`check_node`), `CopRegistry`
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

## Rules

- Keep [PROGRESS.md](PROGRESS.md) up to date when completing milestone tasks. Check off items as done and update milestone status.
- See [PLAN.md](PLAN.md) for full roadmap, cop batching strategy, and technical design decisions.
