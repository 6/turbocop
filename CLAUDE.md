# turbocop

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
cargo run -- -a .    # lint + autocorrect (safe cops only)
cargo run -- -A .    # lint + autocorrect (all cops, including unsafe)
cargo run -- --format json .
cargo run -- --debug .       # phase-level timing breakdown
```

## Before Committing

Always run these before committing to match CI:

```
cargo fmt
cargo clippy --release -- -D warnings
cargo test --release
```

## Performance Profiling

`--debug` prints phase-level timing: bundler shell-outs, config loading, and per-phase linter breakdown (file I/O, Prism parse, CodeMap build, cop execution split into filter+config vs AST walk, disable filtering) using `AtomicU64` counters across rayon threads.

`TURBOCOP_COP_PROFILE=1` enables per-cop timing (requires `--debug`). Re-runs all files single-threaded and reports the top 30 slowest cops broken down by `check_lines`, `check_source`, and `check_node` (AST walk) time. Example:

```
TURBOCOP_COP_PROFILE=1 cargo run --release -- --debug bench/repos/mastodon
```

## Debugging & Benchmarking Tips

- **Isolate a single cop** to measure its cost or debug its behavior:
  ```
  cargo run --release -- --debug --only Style/SymbolProc bench/repos/mastodon
  ```
- **Per-cop profiling** is available via `TURBOCOP_COP_PROFILE=1` (see Performance Profiling above).
- **Comparative benchmarking** with hyperfine:
  ```
  hyperfine --warmup 2 --runs 5 'cargo run --release -- bench/repos/mastodon'
  ```
- **Note on `--debug` timing:** The phase-level timings for filter+config and AST walk show cumulative thread time summed across all rayon workers, not wall time. This means the reported values will exceed wall clock time on multi-core machines.
- **Clear result cache after code changes:** turbocop caches lint results per-file in `~/.cache/turbocop/`. The cache session hash includes version+config+args but NOT cop implementation code. After rebuilding with cop fixes, stale cached results are returned. Clear with `rm -rf ~/.cache/turbocop` before verifying fixes. Using `--only CopName` produces a different session hash and uses a different (empty) cache, which is why `--only` runs may show different results than full scans after code changes.
- **Verify conformance with correct JSON format:** turbocop's JSON output uses `offenses` at the top level (not `files[].offenses[]` like RuboCop). Parse with `d.get('offenses', [])`, not `d.get('files', [])`.

## Architecture

- `src/diagnostic.rs` — Severity, Location, Diagnostic types (includes `corrected: bool` field)
- `src/correction.rs` — `Correction` (byte-offset replacement) and `CorrectionSet` (sort, dedup overlaps, apply)
- `src/parse/` — Prism wrapper + SourceFile (line offsets, byte→line:col, `line_col_to_offset`)
- `src/cop/` — `Cop` trait (`check_lines`/`check_node`/`check_source`), `CopRegistry`, department modules (`layout/`, `lint/`, `metrics/`, `naming/`, `performance/`, `rails/`, `rspec/`, `style/`)
- `src/testutil.rs` — `#[cfg(test)]` fixture parser (annotations, `# turbocop-expect:`, `# turbocop-filename:`) + assertion helpers + autocorrect test helpers
- `src/config/` — `.rubocop.yml` loading with `inherit_from`, `inherit_gem`, `inherit_mode`, auto-discovery
- `src/fs.rs` — File discovery via `ignore` crate (.gitignore-aware)
- `src/linter.rs` — Parallel orchestration (parse per-thread since ParseResult is !Send)
- `src/formatter/` — Text (RuboCop-compatible), JSON, progress, pacman, GitHub, quiet, and files-only output
- `src/cli.rs` — Clap args (`-a`/`-A` autocorrect flags, `AutocorrectMode` enum)
- `src/lib.rs` — `run()` wiring; `src/main.rs` — entry point

## Key Constraints

- `ruby_prism::ParseResult` is `!Send + !Sync` — parsing MUST happen inside each rayon worker thread
- Cop trait is `Send + Sync`; cops needing mutable visitor state create a temporary `Visit` struct internally
- Edition 2024 (Rust 1.85+)

## Autocorrect

Autocorrect infrastructure is in place (Phase 0). The Cop trait methods (`check_lines`, `check_source`, `check_node`) all accept a `corrections: Option<&mut Vec<crate::correction::Correction>>` parameter. Currently all call sites pass `None`; individual cops opt in by overriding `supports_autocorrect() -> bool` and pushing `Correction` structs when `corrections` is `Some`.

**Key types:**
- `Correction` — byte-offset range (`start..end`) + replacement string + cop identity
- `CorrectionSet` — sorts corrections, drops overlapping edits (registration-order wins), applies via linear scan

**CopConfig helpers** for autocorrect decisions:
- `is_safe()` / `is_safe_autocorrect()` — reads `Safe` / `SafeAutoCorrect` YAML keys
- `should_autocorrect(mode)` — combines YAML config with CLI `AutocorrectMode` (Off/Safe/All)

**CLI flags:** `-a` (safe corrections only), `-A` (all corrections including unsafe). Cache is automatically disabled during autocorrect.

## Plugin Cop Version Awareness

turbocop compiles ALL cops into the binary, including cops from plugin gems (rubocop-rspec, rubocop-rails, rubocop-performance). But target projects may use older gem versions that don't include newer cops. The vendor submodules pin the latest versions we support — they are NOT the versions the target project uses.

When turbocop processes `require: [rubocop-rspec]`, it runs `bundle info --path rubocop-rspec` in the target project to find the *installed* gem version, then loads that gem's `config/default.yml`. Plugin cops not mentioned in the installed gem's `config/default.yml` should be treated as non-existent (disabled), because the target project's gem version doesn't include them. This matches RuboCop's behavior where only cops that exist in the installed gem are registered.

## Standardrb Support

turbocop supports projects using standardrb. The config loader recognizes `standard`, `standard-performance`, `standard-rails`, and `standard-custom` as gem families and resolves their version-specific config files (e.g., `config/ruby-3.2.yml`). This handles both pure `.standard.yml` projects and hybrid setups that `require: standard` inside `.rubocop.yml`.

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
3. Fix flagged cops, add test coverage, re-run `cargo run --release --bin bench_turbocop -- conform` to verify FP counts

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

Each cop has a test fixture directory under `tests/fixtures/cops/<dept>/<cop_name>/` with:

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

**Autocorrect layout** (cops that support autocorrect): `offense.rb` + `no_offense.rb` + `corrected.rb`
- Use `cop_autocorrect_fixture_tests!` macro alongside `cop_fixture_tests!` in the cop's test module
- `corrected.rb` contains the expected output after applying autocorrect to `offense.rb` (annotations stripped)
- The test harness strips `^` annotations from `offense.rb`, runs the cop with corrections enabled, applies `CorrectionSet`, and compares byte-for-byte against `corrected.rb`
- Helper functions: `run_cop_autocorrect()` returns corrections, `assert_cop_autocorrect()` validates the full round-trip

**Special directives** (stripped from clean source before running the cop):
- `# turbocop-filename: Name.rb` — first line only; overrides the filename passed to `SourceFile` (used by `Naming/FileName`)
- `# turbocop-expect: L:C Department/CopName: Message` — explicit offense at line L, column C; use when `^` can't be placed (trailing blanks, missing newlines)

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
3. **Convert to turbocop format** — strip the heredoc wrapper, prepend the department/cop prefix to annotations, write to `tests/fixtures/cops/{dept}/{cop_name}/offense.rb`
4. **Extract `expect_no_offenses` blocks** — combine clean Ruby snippets into `no_offense.rb` (≥5 non-empty lines)
5. **Adapt annotations** — vendor specs use just the message after `^`; turbocop requires `Department/CopName: message` format:
   - Vendor: `^^^ Use find_by instead of where.take.`
   - turbocop: `^^^ Rails/FindBy: Use find_by instead of where.take.`
6. **Handle edge cases**:
   - Vendor specs with interpolation (`#{method}`) — pick concrete examples
   - Vendor specs testing config variations — use default config for fixtures, test variations inline
   - Cops that fire once per file — use `offense/` scenario directory layout
7. **Validate** — `cargo test` enforces ≥3 offense annotations and ≥5 no_offense lines per cop

## Benchmarking

```
cargo run --release --bin bench_turbocop                          # full run: setup + bench + conform + report
cargo run --release --bin bench_turbocop -- setup                  # clone benchmark repos only
cargo run --release --bin bench_turbocop -- bench                  # timing benchmarks (hyperfine)
cargo run --release --bin bench_turbocop -- conform                # conformance comparison → bench/conform.json + bench/results.md
cargo run --release --bin bench_turbocop -- report                 # regenerate results.md from cached data
cargo run --release --bin bench_turbocop -- quick                  # quick bench: rubygems.org, cached vs uncached → bench/quick_results.md
cargo run --release --bin bench_turbocop -- autocorrect-conform    # autocorrect conformance: rubocop -A vs turbocop -A file diff
```

Results are written to `bench/results.md` (checked in). Quick bench results go to `bench/quick_results.md`. Conformance data is also written to `bench/conform.json` (gitignored) as structured data for the coverage table. Benchmark repos are cloned to `bench/repos/` (gitignored).

Conformance filters RuboCop offenses to only cops in turbocop's registry (`--list-cops`). Unsupported plugin cops (e.g., minitest, rake, thread_safety) are automatically excluded from comparison — no per-repo handling needed.

**Autocorrect conformance** (`autocorrect-conform`) copies each bench repo, runs `rubocop -A` on one copy and `turbocop -A` on the other, then diffs all `.rb` files. Reports per-repo match/mismatch/error counts. This is the integration-level test that autocorrect output matches RuboCop exactly.

### Private Repo Benchmarking

Private/local repos are configured in `bench/private_repos.json` (gitignored). Each entry has a `name` and a `path` (supports `~/` expansion):

```json
[
  {"name": "my-app", "path": "~/path/to/my-app"}
]
```

The repo must exist and contain a `Gemfile`. To add a new repo, append an entry to the JSON array.

Run benchmarks on private repos:

```
cargo run --release --bin bench_turbocop -- conform --private        # private repos only
cargo run --release --bin bench_turbocop -- conform --all-repos      # public + private
cargo run --release --bin bench_turbocop -- bench --private           # timing only
```

Results go to `bench/private_results.md` and `bench/private_conform.json` (both gitignored), separate from public results.

## RubyGem Distribution

See [docs/rubygem.md](docs/rubygem.md) for the gem build/release pipeline, platform variants, and build scripts.

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

Pipeline: `bench_turbocop conform` → `bench/conform.json` → `coverage_table` → `docs/coverage.md`

## Corpus Fix Loop

After a corpus oracle CI run, use `/fix-cops` to auto-fix a batch of high-FP cops in parallel. It triages, investigates, spawns worktree-isolated teammates to fix each cop, and collects results. See `.claude/skills/fix-cops/SKILL.md`.

Use `/fix-department <gem-name>` to bring all cops in a specific gem to 100% corpus conformance. This is the preferred approach for incremental adoption — completing one gem at a time (e.g., `rubocop-performance`) so users can adopt it with confidence. See `.claude/skills/fix-department/SKILL.md`.

Use `/fix-repo <name>` to improve a specific repo's conformance (e.g., `/fix-repo rails`). It shows the repo's top diverging cops, lets you pick which to fix, and dispatches parallel teammates. See `.claude/skills/fix-repo/SKILL.md`.

Use `/triage` to just view the ranked cop list without fixing. See `.claude/skills/triage/SKILL.md`.

## Corpus Investigation

**IMPORTANT:** `investigate-cop.py` and `investigate-repo.py` auto-download corpus results from the latest CI corpus oracle run. Do NOT manually download artifacts with `gh run download` — just run the scripts directly, they handle fetching. When given a corpus oracle run URL, use these scripts instead of manually downloading.

To investigate a cop's false positives/negatives without re-running turbocop, use `investigate-cop.py`. It reads pre-computed data from `corpus-results.json` (downloaded from CI) and shows all FP/FN locations grouped by repo:

```
python3 scripts/investigate-cop.py Department/CopName                # all FP/FN grouped by repo
python3 scripts/investigate-cop.py Department/CopName --repos-only   # just repo breakdown table
python3 scripts/investigate-cop.py Department/CopName --context      # show source lines from vendor/corpus/
python3 scripts/investigate-cop.py Department/CopName --fp-only      # only false positives
python3 scripts/investigate-cop.py Department/CopName --fn-only      # only false negatives
python3 scripts/investigate-cop.py Department/CopName --input f.json # use local corpus-results.json
```

Use this as the **first step** when investigating a cop — it's instant (reads cached JSON) and shows every FP/FN location with source context from `vendor/corpus/`. No need to re-run turbocop.

To investigate a **repo's** conformance (e.g., "why is rails at 80%?"), use `investigate-repo.py`. It shows the top diverging cops for that repo:

```
python3 scripts/investigate-repo.py rails                    # fuzzy match repo name
python3 scripts/investigate-repo.py rails --fp-only          # only FP-producing cops
python3 scripts/investigate-repo.py rails --fn-only          # only FN-producing cops
python3 scripts/investigate-repo.py rails --limit 10         # top 10 (default 20)
python3 scripts/investigate-repo.py --list                   # list all repos by match rate
python3 scripts/investigate-repo.py --input f.json rails     # use local corpus-results.json
python3 scripts/investigate-repo.py rails --exclude-cops-file fix-cops-done.txt  # skip already-fixed cops
```

Use `/fix-repo <name>` to fix the top diverging cops for a specific repo. See `.claude/skills/fix-repo/SKILL.md`.

## Corpus Regression Testing

After fixing any cop, run the corpus check to verify no FP regression against 500 real-world repos:

```
python3 scripts/check-cop.py Department/CopName                    # aggregate check (re-runs turbocop)
python3 scripts/check-cop.py Department/CopName --verbose           # per-repo breakdown (uses cached data if available)
python3 scripts/check-cop.py Department/CopName --verbose --rerun   # force re-execution after a fix
python3 scripts/check-cop.py Department/CopName --input results.json # use local corpus-results.json
```

The script compares turbocop offense counts against the RuboCop baseline from the latest CI corpus oracle run. `FAIL` means turbocop produces more offenses than RuboCop (false positives). With `--verbose`, it uses enriched per-repo data from `corpus-results.json` when available (instant). Pass `--rerun` to force re-execution of turbocop after making code changes.

## Rules

- **NEVER copy code or identifiers from private repos into this codebase.** When fixing false positives found by running against private/internal repos, write generic test cases that reproduce the same pattern. Use generic names (e.g. `records`, `payload`, `User`, `name`, `role`, `status`) instead of domain-specific names from the private codebase. Do not use variable names, method names, or terminology that originated in private repo source code — even if they seem generic, if you encountered them in a private repo, replace them. Do not reference private repo names or paths in committed files. This applies to test fixtures, comments, commit messages, and any other checked-in files.
- After adding a new cop, ensure `cargo test` passes — the `all_cops_have_minimum_test_coverage` integration test enforces that every cop has at least 3 offense fixture cases and 5+ non-empty lines in no_offense.rb. There are zero exemptions; use `offense/` scenario directories and `# turbocop-expect:` annotations to handle cops that can't use the standard single-file format.
- **Use TDD when fixing cops.** Write the failing test case first (add to `offense.rb` or `no_offense.rb`), verify it fails, then implement the fix and confirm the test passes. This applies to both new detections and false-positive fixes.
- **Every cop fix or false-positive fix must include test coverage.** When fixing a false positive, add the previously-false-positive case to the cop's `no_offense.rb` fixture. When fixing a missed detection, add it to `offense.rb`. This prevents regressions and documents the expected behavior.
- **Don't remove or move test cases unless they are factually incorrect.** Existing offense and no_offense fixtures represent verified correct behavior. If a code change causes existing tests to fail, the change is likely too aggressive and introduces regressions (FPs or FNs on other repos). Fix the approach rather than deleting tests. The exception: if a test case is provably wrong (e.g., turbocop was flagging something RuboCop doesn't flag), it should be moved to the correct fixture file (offense → no_offense or vice versa) with a clear explanation.
- **NEVER use `git stash` or `git stash pop`.** Work has been lost in the past from stash conflicts and forgotten stashes. Instead, commit work-in-progress to a branch, or use a worktree for parallel work. If you need to switch context, commit first with a WIP message.
- **After adding or fixing cops, regenerate coverage docs.** Run the full pipeline to keep docs up to date:
  1. `cargo run --release --bin bench_turbocop -- conform` — regenerate conformance data (`bench/conform.json`)
  2. `cargo run --bin coverage_table -- --show-missing --output docs/coverage.md` — regenerate coverage table
