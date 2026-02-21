# turbocop — Phase 1 Progress

> For pre-Phase-1 history (M0–M20, 915 cops, 12/12 bench repos at 100%), see
> `docs/archive/PROGRESS_v1.md`. For the full plan, see `docs/PLAN.md`.

## Phase 1: Adoption + Safety

### Tiering & skip classification

- [x] `resources/tiers.json` — embedded tier data (schema 1, default stable, curated preview overrides)
- [x] `src/cop/tiers.rs` — `Tier` enum, `TierMap`, `SkipSummary` struct
- [x] `--preview` CLI flag — enables preview-tier cops
- [x] `--quiet-skips` CLI flag — suppresses skip summary notice
- [x] Skip classification in `build_cop_filters()` — preview tier gating
- [x] `compute_skip_summary()` — classifies enabled cops into preview-gated / unimplemented / outside-baseline
- [x] Grouped skip notice in `run()` — one-line stderr summary at end of run
- [x] JSON output includes `skipped` field with 3 categories + total
- [x] Unit tests for TierMap, SkipSummary, JSON formatter skip output
- [x] All integration tests updated for new signatures

### Exit code contract

- [x] `--strict[=SCOPE]` flag — `coverage` (default), `implemented-only`, `all`
- [x] Exit code 2 for strict coverage failure (skipped cops violate scope)
- [x] Exit code 3 for internal errors (panic, IO, config parse)
- [x] Tests for exit code semantics

### `migrate` command

- [ ] `turbocop migrate [PATH]` — config analysis, no linting
- [ ] Text output: grouped counts + top examples per category
- [ ] JSON output: baseline versions + per-cop status
- [ ] Suggested CI command line in output

### `doctor` command

- [ ] `turbocop doctor` — debug/support output
- [ ] Baseline versions (vendored RuboCop + plugin versions)
- [ ] Config root + inheritance chain
- [ ] Gem version mismatch warnings
- [ ] Skip summary (same 4 categories as `check`)

### `rules` command

- [ ] `turbocop rules [--tier <stable|preview>] [--format <table|json>]`
- [ ] Output: name, tier, implemented?, baseline presence, default enabled?

### NodePattern verifier

- [ ] Finish interpreter (adapt `node_pattern_codegen.rs` lexer/parser)
- [ ] Verifier harness: compiled vs interpreted matching on bench-repo AST nodes
- [ ] `cargo test verifier` in CI
- [ ] Gate merges that modify matching logic

### Autocorrect

- [x] `-a`/`-A` flags working for existing cops (Phase 0 infrastructure)
- [ ] `resources/autocorrect_safe_allowlist.json` — initial safe cop list
- [ ] `-a` restricted to allowlisted cops only

### `verify` command (optional but recommended)

- [ ] `turbocop verify [PATH]` — oracle mode, requires Ruby
- [ ] Runs both tools, normalizes JSON, diffs per cop
- [ ] Exit codes: 0 (match), 1 (diffs), 3 (tool error)

### Doc consolidation

- [x] Archived old docs to `docs/archive/`
- [x] Created consolidated `docs/PLAN.md`
- [x] Created fresh `PROGRESS.md` (this file)

## Cop Coverage & Conformance

See **[docs/coverage.md](docs/coverage.md)** for the auto-generated coverage table and bench conformance results.

| Stat | Value |
|------|-------|
| Cops registered | 915 |
| Bench repos at 100% | 12/12 |
| Departments at 100% | 14 |
