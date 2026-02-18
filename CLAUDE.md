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
- `src/cop/` — `Cop` trait (`check_lines`/`check_node`/`check_source`), `CopRegistry`, department modules (`layout/`, `lint/`, `metrics/`, `naming/`, `performance/`, `rails/`, `rspec/`, `style/`)
- `src/testutil.rs` — `#[cfg(test)]` fixture parser (annotations, `# rblint-expect:`, `# rblint-filename:`) + assertion helpers
- `src/config/` — `.rubocop.yml` loading with `inherit_from`, `inherit_gem`, `inherit_mode`, auto-discovery
- `src/fs/` — File discovery via `ignore` crate (.gitignore-aware)
- `src/linter.rs` — Parallel orchestration (parse per-thread since ParseResult is !Send)
- `src/formatter/` — Text (RuboCop-compatible) and JSON output
- `src/cli.rs` — Clap args
- `src/lib.rs` — `run()` wiring; `src/main.rs` — entry point

## Key Constraints

- `ruby_prism::ParseResult` is `!Send + !Sync` — parsing MUST happen inside each rayon worker thread
- Cop trait is `Send + Sync`; cops needing mutable visitor state create a temporary `Visit` struct internally
- Edition 2024 (Rust 1.85+)

## Plugin Cop Version Awareness

rblint compiles ALL cops into the binary, including cops from plugin gems (rubocop-rspec, rubocop-rails, rubocop-performance). But target projects may use older gem versions that don't include newer cops. The vendor submodules pin the latest versions we support — they are NOT the versions the target project uses.

When rblint processes `require: [rubocop-rspec]`, it runs `bundle info --path rubocop-rspec` in the target project to find the *installed* gem version, then loads that gem's `config/default.yml`. Plugin cops not mentioned in the installed gem's `config/default.yml` should be treated as non-existent (disabled), because the target project's gem version doesn't include them. This matches RuboCop's behavior where only cops that exist in the installed gem are registered.

## Keeping in Sync with RuboCop

RuboCop is a moving target — new cops, changed behavior, and evolving NodePattern definitions. The vendor submodules (`vendor/rubocop`, `vendor/rubocop-rails`, etc.) pin specific release tags. **Submodules must always point to a proper release tag** (e.g., `v1.84.2`, `v2.34.3`), never arbitrary commits on `master`.

### Updating vendor submodules

```bash
cd vendor/rubocop && git fetch --tags && git checkout v1.XX.0    # repeat for each plugin
cd vendor/rubocop-rails && git fetch --tags && git checkout v2.XX.0
cd vendor/rubocop-rspec && git fetch --tags && git checkout v3.XX.0
cd vendor/rubocop-performance && git fetch --tags && git checkout v1.XX.0
```

### Updating bench repo dependencies

After updating submodules, update the bench repos to use the same gem versions:

```bash
ruby bench/update_rubocop_deps.rb          # update all bench repos
ruby bench/update_rubocop_deps.rb --dry-run # preview changes
```

This script reads `version.rb` from each vendor submodule, pins those versions in bench repo Gemfiles, and runs `bundle update`. It also verifies submodules are on proper release tags.

### Verification after updates

1. `cargo test config_audit -- --nocapture` — reports YAML config keys that cops don't read yet
2. `cargo test prism_pitfalls -- --nocapture` — flags cops missing `KeywordHashNode` or `ConstantPathNode` handling
3. Fix flagged cops, add test coverage, re-run `cargo run --release --bin bench_rblint -- conform` to verify FP counts

## Common Prism Pitfalls

These are the most frequent sources of false negatives (45% of historical bugs):
- `const` in Parser gem splits into `ConstantReadNode` (simple `Foo`) AND `ConstantPathNode` (qualified `Foo::Bar`) — must handle both
- `begin` is overloaded: explicit `begin..end` → `BeginNode`, implicit method body → `StatementsNode`
- `hash` splits into `HashNode` (literal `{}`) and `KeywordHashNode` (keyword args `foo(a: 1)`)
- `send`/`csend` merge into `CallNode` — check `.call_operator()` for safe-navigation `&.`
- `nil?` in NodePattern means "child is absent" (`receiver().is_none()`), NOT a `NilNode` literal

See `docs/node_pattern_analysis.md` for the full Parser→Prism mapping table.

## Quality Checks

Two zero-tolerance integration tests enforce implementation completeness:

- **`cargo test config_audit`** — cross-references vendor YAML config keys against `config.get_str/get_usize/get_bool/get_string_array/get_string_hash` calls in Rust source. Fails if any key is missing from the cop's source.
- **`cargo test prism_pitfalls`** — scans cop source for `as_hash_node` without `keyword_hash_node` and `as_constant_read_node` without `constant_path_node`. Fails if any cop handles one node type but not the other.

Both tests require **zero gaps** — any new cop or config key must be fully implemented before tests pass.

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

## Vendor Fixture Extraction Process

To add a new cop department from a RuboCop plugin (e.g., rubocop-rspec, rubocop-performance), extract test fixtures from the vendor specs:

1. **Read the vendor spec** at `vendor/rubocop-{plugin}/spec/rubocop/cop/{dept}/{cop_name}_spec.rb`
2. **Extract `expect_offense` blocks** — these contain inline Ruby with `^` annotation markers:
   ```ruby
   expect_offense(<<~RUBY)
     User.where(id: x).take
          ^^^^^^^^^^^^^^^^^ Use `find_by` instead of `where.take`.
   RUBY
   ```
3. **Convert to rblint format** — strip the heredoc wrapper, prepend the department/cop prefix to annotations, write to `testdata/cops/{dept}/{cop_name}/offense.rb`
4. **Extract `expect_no_offenses` blocks** — combine clean Ruby snippets into `no_offense.rb` (≥5 non-empty lines)
5. **Adapt annotations** — vendor specs use just the message after `^`; rblint requires `Department/CopName: message` format:
   - Vendor: `^^^ Use find_by instead of where.take.`
   - rblint: `^^^ Rails/FindBy: Use find_by instead of where.take.`
6. **Handle edge cases**:
   - Vendor specs with interpolation (`#{method}`) — pick concrete examples
   - Vendor specs testing config variations — use default config for fixtures, test variations inline
   - Cops that fire once per file — use `offense/` scenario directory layout
7. **Validate** — `cargo test` enforces ≥3 offense annotations and ≥5 no_offense lines per cop

## Benchmarking

```
cargo run --release --bin bench_rblint                # full run: setup + bench + conform + report
cargo run --release --bin bench_rblint -- setup        # clone benchmark repos only
cargo run --release --bin bench_rblint -- bench        # timing benchmarks (hyperfine)
cargo run --release --bin bench_rblint -- conform      # conformance comparison → bench/conform.json + bench/results.md
cargo run --release --bin bench_rblint -- report       # regenerate results.md from cached data
```

Results are written to `bench/results.md` (checked in). Conformance data is also written to `bench/conform.json` (gitignored) as structured data for the coverage table. Benchmark repos are cloned to `bench/repos/` (gitignored).

## Coverage Reporting

```
cargo run --bin coverage_table                                  # print to stdout
cargo run --bin coverage_table -- --show-missing                # include missing cop lists
cargo run --bin coverage_table -- --output docs/coverage.md     # write to file (checked in)
```

Generates `docs/coverage.md` with:
- **Cop coverage table** — counts per department from vendor YAML vs registry
- **Missing cops** — which vendor cops aren't implemented yet (with `--show-missing`)
- **Conformance table** — FP/FN rates per bench repo (reads `bench/conform.json` if available)

Pipeline: `bench_rblint conform` → `bench/conform.json` → `coverage_table` → `docs/coverage.md`

PROGRESS.md links to `docs/coverage.md` instead of maintaining inline tables.

## Rules

- Keep [PROGRESS.md](PROGRESS.md) up to date when completing milestone tasks. Check off items as done and update milestone status.
- See [PLAN.md](PLAN.md) for full roadmap, cop batching strategy, and technical design decisions.
- After adding a new cop, ensure `cargo test` passes — the `all_cops_have_minimum_test_coverage` integration test enforces that every cop has at least 3 offense fixture cases and 5+ non-empty lines in no_offense.rb. There are zero exemptions; use `offense/` scenario directories and `# rblint-expect:` annotations to handle cops that can't use the standard single-file format.
- **Every cop fix or false-positive fix must include test coverage.** When fixing a false positive, add the previously-false-positive case to the cop's `no_offense.rb` fixture. When fixing a missed detection, add it to `offense.rb`. This prevents regressions and documents the expected behavior.
