# rblint — Progress Tracker

See [PLAN.md](PLAN.md) for the full roadmap and technical design.

## Completed: M0 — Skeleton

Parse Ruby files with Prism, basic config loading, parallel cop execution framework.
All files compile, binary runs, produces "0 offenses detected."

### M0 Tasks

- [x] **Cargo.toml** — Dependencies, edition 2024, Prism C FFI build validated
- [x] **src/diagnostic.rs** — Diagnostic, Location, Severity types
- [x] **src/parse/source.rs** — SourceFile with byte-offset-to-line:col translation
- [x] **src/parse/mod.rs** — Prism parser wrapper (`parse_source()`)
- [x] **src/cop/mod.rs** — Cop trait + CopConfig
- [x] **src/cop/registry.rs** — CopRegistry (empty at M0, registration mechanism works)
- [x] **src/config/mod.rs** — Stub config loader (reads .rubocop.yml, no inheritance)
- [x] **src/fs/mod.rs** — File discovery with `ignore` crate, .gitignore + AllCops.Exclude
- [x] **src/linter.rs** — Linter orchestration, parallel cop execution via rayon
- [x] **src/formatter/text.rs** — RuboCop-compatible text output
- [x] **src/formatter/json.rs** — JSON formatter
- [x] **src/formatter/mod.rs** — Formatter trait + factory
- [x] **src/cli.rs** — Clap CLI args
- [x] **src/lib.rs** + **src/main.rs** — Entry point wiring
- [x] **Verify**: `cargo run -- .` parses .rb files, reports 0 offenses
- [x] Unit tests for SourceFile, config loading, file discovery, formatters

## Completed: M1 — Line-based cops + Test harness

8 line-based cops, annotation-based test harness, RuboCop git submodule for reference.

### M1 Tasks

- [x] **vendor/rubocop** — Git submodule (shallow clone) for reference specs
- [x] **src/testutil.rs** — Annotation-based fixture parser + assertion helpers
- [x] **src/cop/layout/** — 6 layout cops:
  - [x] Layout/TrailingWhitespace
  - [x] Layout/LineLength (configurable Max, default 120)
  - [x] Layout/TrailingEmptyLines
  - [x] Layout/LeadingEmptyLines
  - [x] Layout/EndOfLine
  - [x] Layout/InitialIndentation
- [x] **src/cop/style/** — 2 style cops:
  - [x] Style/FrozenStringLiteralComment (shebang/encoding-aware)
  - [x] Style/Tab
- [x] **src/cop/registry.rs** — `default_registry()` registers all 8 cops
- [x] **testdata/cops/** — Fixture files with offense/no_offense cases
- [x] **.gitattributes** — Whitespace preservation for test fixtures
- [x] 96 tests passing (43 from M0 + 53 new)

## Next: M2 — Token / simple-pattern cops

See [PLAN.md § Batch 1](PLAN.md#batch-1-tokensimple-pattern-cops-minimal-ast).

- [ ] Layout/EmptyLineBetweenDefs (basic)
- [ ] Style/MagicComment alignment
- [ ] Additional line-based cops from Batch 0 not yet implemented
- [ ] Token-based cops from Batch 1

## Upcoming Milestones

| Milestone | Cops | Status |
|-----------|------|--------|
| **M0**: Skeleton | 0 | **Done** |
| **M1**: Line-based cops | 8 | **Done** |
| **M2**: Token cops | 18 | Pending — [PLAN.md § Batch 1](PLAN.md#batch-1-tokensimple-pattern-cops-minimal-ast) |
| **M3**: AST single-node | 70 | Pending — [PLAN.md § Batch 2](PLAN.md#batch-2-ast-walking-cops-single-node-patterns) |
| **M4**: Performance cops | 40 | Pending — [PLAN.md § Batch 3](PLAN.md#batch-3-rubocop-performance-cops-all) |
| **M5**: Complex core cops | 50 | Pending — [PLAN.md § Batch 4](PLAN.md#batch-4-complex-core-cops--remaining-core) |
| **M6**: bin/lint + --rubocop-only | 0 new | Pending |
| **M7**: Autocorrect | +30 fixes | Pending |
| **M8**: rubocop-rspec | 80 | Pending |
| **M9**: rubocop-rails | 70 | Pending |
