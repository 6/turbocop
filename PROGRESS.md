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

## Completed: M2 — Token / simple-pattern cops

17 new cops (25 total), CodeMap infrastructure, `check_source` cop method, CopWalker extraction.

### M2 Infrastructure

- [x] **src/cop/walker.rs** — Extracted CopWalker from linter.rs (shared by linter + test harness)
- [x] **src/parse/codemap.rs** — CodeMap: sorted non-code byte ranges for O(log n) `is_code()` lookups
- [x] **src/cop/mod.rs** — Added `check_source` method to Cop trait (default no-op)
- [x] **src/linter.rs** — Builds CodeMap per file, calls `check_source` between check_lines and check_node
- [x] **src/testutil.rs** — Full-pipeline test helpers: `run_cop_full`, `assert_cop_offenses_full`, etc.
- [x] **src/cop/lint/mod.rs** — New Lint department with `register_all()`

### M2 Cops — Layout (11 new)

- [x] Layout/EmptyLines — consecutive blank lines > Max (line-based)
- [x] Layout/SpaceAfterComma — check_source + CodeMap
- [x] Layout/SpaceAfterSemicolon — check_source + CodeMap
- [x] Layout/SpaceBeforeComma — check_source + CodeMap
- [x] Layout/SpaceAroundEqualsInParameterDefault — AST (OptionalParameterNode)
- [x] Layout/SpaceAfterColon — AST (AssocNode shorthand hash)
- [x] Layout/SpaceInsideParens — AST (ParenthesesNode)
- [x] Layout/SpaceInsideHashLiteralBraces — AST (HashNode, configurable)
- [x] Layout/SpaceInsideBlockBraces — AST (BlockNode)
- [x] Layout/SpaceInsideArrayLiteralBrackets — AST (ArrayNode)
- [x] Layout/SpaceBeforeBlockBraces — AST (BlockNode)

### M2 Cops — Lint (2 new)

- [x] Lint/Debugger — AST (CallNode: binding.pry, debugger, byebug, binding.irb)
- [x] Lint/LiteralAsCondition — AST (IfNode/WhileNode/UntilNode with literal predicate)

### M2 Cops — Style (4 new)

- [x] Style/StringLiterals — AST (StringNode, configurable EnforcedStyle)
- [x] Style/RedundantReturn — AST (DefNode last statement is ReturnNode)
- [x] Style/NumericLiterals — AST (IntegerNode, configurable MinDigits)
- [x] Style/Semicolon — check_source + CodeMap

### M2 Summary

- [x] **src/cop/registry.rs** — `default_registry()` registers all 25 cops
- [x] **testdata/cops/** — 34 new fixture files (offense + no_offense for each new cop)
- [x] 175 tests passing (159 unit + 16 integration)

## Next: M3 — AST single-node cops

See [PLAN.md § Batch 2](PLAN.md#batch-2-ast-walking-cops-single-node-patterns).

## Upcoming Milestones

| Milestone | Cops | Status |
|-----------|------|--------|
| **M0**: Skeleton | 0 | **Done** |
| **M1**: Line-based cops | 8 | **Done** |
| **M2**: Token/simple-pattern cops | 25 | **Done** |
| **M3**: AST single-node | 70 | Pending — [PLAN.md § Batch 2](PLAN.md#batch-2-ast-walking-cops-single-node-patterns) |
| **M4**: Performance cops | 40 | Pending — [PLAN.md § Batch 3](PLAN.md#batch-3-rubocop-performance-cops-all) |
| **M5**: Complex core cops | 50 | Pending — [PLAN.md § Batch 4](PLAN.md#batch-4-complex-core-cops--remaining-core) |
| **M6**: bin/lint + --rubocop-only | 0 new | Pending |
| **M7**: Autocorrect | +30 fixes | Pending |
| **M8**: rubocop-rspec | 80 | Pending |
| **M9**: rubocop-rails | 70 | Pending |
