# nitrocop

Fast Ruby linter in Rust targeting RuboCop compatibility. Uses Prism (ruby-prism crate) for parsing, rayon for parallelism.

## Setup

```
git submodule update --init    # fetch vendor/rubocop (reference specs)
```

## Dual-Platform Development

This repo is developed simultaneously on macOS (native) and Linux (devcontainer). To prevent cross-platform binary conflicts in the shared workspace, the devcontainer sets `CARGO_TARGET_DIR=target-linux` so each platform builds to a separate directory (`target/` for macOS, `target-linux/` for Linux). Both are gitignored.

Python scripts in `scripts/` resolve the binary path via `os.environ.get("CARGO_TARGET_DIR", "target")` so they work on both platforms automatically.

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

Run these checks before committing **when your change touches Rust code** (`src/**/*.rs`, `bench/**/*.rs`, `Cargo.toml`, `Cargo.lock`):

```
cargo fmt -- src/path/to/changed_file.rs    # format only the files you modified
cargo clippy --release -- -D warnings       # incremental — fast when few files changed
cargo test --release
```

If the change is non-Rust only (for example `docs/`, fixtures under `tests/fixtures/`, or skill markdown/scripts), these Rust checks are optional and should not be run by default.

**Important:** Run `cargo fmt` on the specific Rust files you edited (not `cargo fmt` with no args, which formats everything). Run `cargo clippy` which leverages incremental compilation and is fast when few files changed. Do NOT use `git diff` to discover changed files — multiple agents may be working on main concurrently.

### Fast Iteration (during development)

For iterative cop development, use **debug tests with a filter** instead of `cargo test --release`:

```
cargo test --lib -- cop::style::while_until_modifier    # single cop, ~8s first run, <1s cached
cargo test --lib -- cop::style::                        # all Style cops
```

`--lib` skips integration tests (avoids compiling a second test binary). Debug mode has much faster incremental compilation than release. Reserve `cargo test --release` for the final pre-commit check only.

## Performance Profiling

`--debug` prints phase-level timing: bundler shell-outs, config loading, and per-phase linter breakdown (file I/O, Prism parse, CodeMap build, cop execution split into filter+config vs AST walk, disable filtering) using `AtomicU64` counters across rayon threads.

`NITROCOP_COP_PROFILE=1` enables per-cop timing (requires `--debug`). Re-runs all files single-threaded and reports the top 30 slowest cops broken down by `check_lines`, `check_source`, and `check_node` (AST walk) time. Example:

```
NITROCOP_COP_PROFILE=1 cargo run --release -- --debug bench/repos/mastodon
```

## Debugging & Benchmarking Tips

- **Isolate a single cop** to measure its cost or debug its behavior:
  ```
  cargo run --release -- --debug --only Style/SymbolProc bench/repos/mastodon
  ```
- **Per-cop profiling** is available via `NITROCOP_COP_PROFILE=1` (see Performance Profiling above).
- **Comparative benchmarking** with hyperfine:
  ```
  hyperfine --warmup 2 --runs 5 'cargo run --release -- bench/repos/mastodon'
  ```
- **Note on `--debug` timing:** The phase-level timings for filter+config and AST walk show cumulative thread time summed across all rayon workers, not wall time. This means the reported values will exceed wall clock time on multi-core machines.
- **Corpus validation (current workflow):** use `python3 scripts/check-cop.py Department/Cop --input <local corpus-results.json> --verbose --rerun` for aggregate count deltas. `--rerun` forces fresh per-repo execution, auto-rebuilds a stale release binary, and clears nitrocop's file cache before running. Prefer this over ad-hoc `--only` comparisons when validating per-cop count regressions. For exact known FP/FN locations, also run `python3 scripts/verify-cop-locations.py Department/Cop`.
- **Corpus bundle setup:** `check-cop.py --rerun` and `--corpus-check` mode require the corpus bundle to be installed for the active Ruby version (set in `mise.toml`). The bundle lives at `bench/corpus/vendor/bundle/`. If `bundle info rubocop` fails (wrong Ruby version or missing gems), config resolution falls back to hardcoded defaults, producing wildly incorrect offense counts (often 5-10x higher than expected). Fix with: `cd bench/corpus && BUNDLE_PATH=vendor/bundle bundle install`. The CI corpus oracle uses `ruby/setup-ruby@v1` which reads `mise.toml` — keep your local Ruby version in sync with `mise.toml` to match CI. The Python scripts (`check-cop.py`, `verify-cop-locations.py`) will warn if the bundle is missing.
- **Manual cache clear (fallback):** if you are running ad-hoc commands outside `check-cop.py`, clear `~/.cache/nitrocop/` to avoid stale per-file cache reuse.
- **Verify conformance with correct JSON format:** nitrocop's JSON output uses `offenses` at the top level (not `files[].offenses[]` like RuboCop). Parse with `d.get('offenses', [])`, not `d.get('files', [])`.

## Architecture

- `src/diagnostic.rs` — Severity, Location, Diagnostic types (includes `corrected: bool` field)
- `src/correction.rs` — `Correction` (byte-offset replacement) and `CorrectionSet` (sort, dedup overlaps, apply)
- `src/parse/` — Prism wrapper + SourceFile (line offsets, byte→line:col, `line_col_to_offset`)
- `src/cop/` — `Cop` trait (`check_lines`/`check_node`/`check_source`), `CopRegistry`, department modules (`layout/`, `lint/`, `metrics/`, `naming/`, `performance/`, `rails/`, `rspec/`, `style/`)
- `src/testutil.rs` — `#[cfg(test)]` fixture parser (annotations, `# nitrocop-expect:`, `# nitrocop-filename:`) + assertion helpers + autocorrect test helpers
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

## Disabled-by-Default Cops

Cops with `Enabled: false` in vendor `config/default.yml` (e.g., `Naming/InclusiveLanguage`, `Bundler/GemComment`, `Style/AsciiComments`) **must** override `fn default_enabled(&self) -> bool { false }` in their `impl Cop` block. Without this override, the cop defaults to enabled when `bundle info rubocop` fails (no vendored config loaded), causing false positives on projects without rubocop in their Gemfile.

**Corpus conformance note:** The corpus oracle and `check-cop.py` use `bench/corpus/baseline_rubocop.yml`, which explicitly sets `Enabled: true` for all disabled-by-default cops. This means `default_enabled()` has **no effect on corpus FP/FN numbers** — the config layer overrides it. The `default_enabled()` override matters only for real-world usage where the vendor defaults are loaded (or fail to load).

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

nitrocop compiles ALL cops into the binary, including cops from plugin gems (rubocop-rspec, rubocop-rails, rubocop-performance). But target projects may use older gem versions that don't include newer cops. The vendor submodules pin the latest versions we support — they are NOT the versions the target project uses.

When nitrocop processes `require: [rubocop-rspec]`, it runs `bundle info --path rubocop-rspec` in the target project to find the *installed* gem version, then loads that gem's `config/default.yml`. Plugin cops not mentioned in the installed gem's `config/default.yml` should be treated as non-existent (disabled), because the target project's gem version doesn't include them. This matches RuboCop's behavior where only cops that exist in the installed gem are registered.

## Standardrb Support

nitrocop supports projects using standardrb. The config loader recognizes `standard`, `standard-performance`, `standard-rails`, and `standard-custom` as gem families and resolves their version-specific config files (e.g., `config/ruby-3.2.yml`). This handles both pure `.standard.yml` projects and hybrid setups that `require: standard` inside `.rubocop.yml`.

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
3. Fix flagged cops, add test coverage, re-run `cargo run --release --bin bench_nitrocop -- conform` to verify FP counts

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
- `# nitrocop-filename: Name.rb` — first line only; overrides the filename passed to `SourceFile` (used by `Naming/FileName`)
- `# nitrocop-expect: L:C Department/CopName: Message` — explicit offense at line L, column C; use when `^` can't be placed (trailing blanks, missing newlines)

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
3. **Convert to nitrocop format** — strip the heredoc wrapper, prepend the department/cop prefix to annotations, write to `tests/fixtures/cops/{dept}/{cop_name}/offense.rb`
4. **Extract `expect_no_offenses` blocks** — combine clean Ruby snippets into `no_offense.rb` (≥5 non-empty lines)
5. **Adapt annotations** — vendor specs use just the message after `^`; nitrocop requires `Department/CopName: message` format:
   - Vendor: `^^^ Use find_by instead of where.take.`
   - nitrocop: `^^^ Rails/FindBy: Use find_by instead of where.take.`
6. **Handle edge cases**:
   - Vendor specs with interpolation (`#{method}`) — pick concrete examples
   - Vendor specs testing config variations — use default config for fixtures, test variations inline
   - Cops that fire once per file — use `offense/` scenario directory layout
7. **Validate** — `cargo test` enforces ≥3 offense annotations and ≥5 no_offense lines per cop

## Benchmarking

```
cargo run --release --bin bench_nitrocop                          # full run: setup + bench + conform + report
cargo run --release --bin bench_nitrocop -- setup                  # clone benchmark repos only
cargo run --release --bin bench_nitrocop -- bench                  # timing benchmarks (hyperfine)
cargo run --release --bin bench_nitrocop -- conform                # conformance comparison → bench/conform.json + bench/results.md
cargo run --release --bin bench_nitrocop -- report                 # regenerate results.md from cached data
cargo run --release --bin bench_nitrocop -- quick                  # quick bench: rubygems.org, cached vs uncached → bench/quick_results.md
cargo run --release --bin bench_nitrocop -- autocorrect-conform    # autocorrect conformance: rubocop -A vs nitrocop -A file diff
```

Results are written to `bench/results.md` (checked in). Quick bench results go to `bench/quick_results.md`. Conformance data is also written to `bench/conform.json` (gitignored) as structured data for the coverage table. Benchmark repos are cloned to `bench/repos/` (gitignored).

Conformance filters RuboCop offenses to only cops in nitrocop's registry (`--list-cops`). Unsupported plugin cops (e.g., minitest, rake, thread_safety) are automatically excluded from comparison — no per-repo handling needed.

**Autocorrect conformance** (`autocorrect-conform`) copies each bench repo, runs `rubocop -A` on one copy and `nitrocop -A` on the other, then diffs all `.rb` files. Reports per-repo match/mismatch/error counts. This is the integration-level test that autocorrect output matches RuboCop exactly.

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
cargo run --release --bin bench_nitrocop -- conform --private        # private repos only
cargo run --release --bin bench_nitrocop -- conform --all-repos      # public + private
cargo run --release --bin bench_nitrocop -- bench --private           # timing only
```

Results go to `bench/private_results.md` and `bench/private_conform.json` (both gitignored), separate from public results.

### Synthetic Corpus

The synthetic corpus at `bench/synthetic/` exercises 55 cops that have zero activity in the 1,000-repo corpus oracle. It contains handcrafted Ruby files in a Rails-like project layout designed to trigger each cop, then compares nitrocop vs RuboCop output.

```
python3 bench/synthetic/run_synthetic.py           # summary
python3 bench/synthetic/run_synthetic.py --verbose  # per-cop breakdown
```

See [`bench/synthetic/README.md`](bench/synthetic/README.md) for full details, including research findings on `railties` gem requirements, Include pattern path matching, and Ruby version gates.

## RubyGem Distribution

See [docs/rubygem.md](docs/rubygem.md) for the gem build/release pipeline, platform variants, and build scripts.

## Corpus Fix Loop

Use `/fix-department <gem-name>` to bring all cops in a specific gem to 100% corpus conformance. This is the preferred approach for incremental adoption — completing one gem at a time (e.g., `rubocop-performance`) so users can adopt it with confidence. See `.claude/skills/fix-department/SKILL.md`.

`/fix-department --loop` is a standing task, not a one-turn command. In Codex,
skill activation is turn-scoped, so later turns like "status?", "commit to
main", "rebase", "push", or similar git maintenance can accidentally look like
standalone tasks even though they are checkpoints inside the same loop. For
this repo, treat those turns as intermediate sub-steps of the active loop:
satisfy the request, then resume the prior `/fix-department --loop` workflow
unless the user explicitly pauses, stops, or redirects. If you cannot safely
resume in the same turn, say so explicitly and tell the user to run
`/fix-continue --loop`.

Use `/triage` to just view the ranked cop list without fixing. See `.claude/skills/triage/SKILL.md`.

### Remote Agent Dispatch

Use `/dispatch-cops` to parallelize cop fixes at scale via AI agents running in GitHub Actions. Each agent fixes one cop and opens a PR validated by CI. Two backends available:
- **Claude Code + MiniMax** (~$0.03/cop) — [docs/agent-dispatch-minimax.md](docs/agent-dispatch-minimax.md)
- **Codex** ($200/mo flat rate) — [docs/agent-dispatch.md](docs/agent-dispatch.md)

See `.claude/skills/dispatch-cops/SKILL.md` for the `/dispatch-cops` skill.

```
/dispatch-cops              # start from triage
/dispatch-cops pilot        # 10-cop pilot
/dispatch-cops tier1        # batch dispatch Tier 1 (~319 cops)
/dispatch-cops retry        # retry failed cops with stronger model
/dispatch-cops status       # check PR status
/dispatch-cops validate     # trigger corpus oracle
```

## Corpus Investigation

**IMPORTANT:** `investigate-cop.py` and `investigate-repo.py` auto-download corpus results from the latest CI corpus oracle run. Do NOT manually download artifacts with `gh run download` — just run the scripts directly, they handle fetching. When given a corpus oracle run URL, use these scripts instead of manually downloading.

To investigate a cop's false positives/negatives without re-running nitrocop, use `investigate-cop.py`. It reads pre-computed data from `corpus-results.json` (downloaded from CI) and shows all FP/FN locations grouped by repo:

```
python3 scripts/investigate-cop.py Department/CopName                # all FP/FN grouped by repo
python3 scripts/investigate-cop.py Department/CopName --repos-only   # just repo breakdown table
python3 scripts/investigate-cop.py Department/CopName --context      # show source lines from vendor/corpus/
python3 scripts/investigate-cop.py Department/CopName --fp-only      # only false positives
python3 scripts/investigate-cop.py Department/CopName --fn-only      # only false negatives
python3 scripts/investigate-cop.py Department/CopName --input f.json # use local corpus-results.json
python3 scripts/investigate-cop.py Department/CopName --extended     # use extended corpus (5k+ repos)
```

Use this as the **first step** when investigating a cop — it's instant (reads cached JSON) and shows every FP/FN location with source context from `vendor/corpus/`. No need to re-run nitrocop.

To investigate a **repo's** conformance (e.g., "why is rails at 80%?"), use `investigate-repo.py`. It shows the top diverging cops for that repo:

```
python3 scripts/investigate-repo.py rails                    # fuzzy match repo name
python3 scripts/investigate-repo.py rails --fp-only          # only FP-producing cops
python3 scripts/investigate-repo.py rails --fn-only          # only FN-producing cops
python3 scripts/investigate-repo.py rails --limit 10         # top 10 (default 20)
python3 scripts/investigate-repo.py --list                   # list all repos by match rate
python3 scripts/investigate-repo.py --input f.json rails     # use local corpus-results.json
python3 scripts/investigate-repo.py rails --no-git-exclude       # skip auto-exclusion of fixed cops
python3 scripts/investigate-repo.py rails --extended             # use extended corpus (5k+ repos)
```

To **reduce a corpus mismatch to a minimal reproduction**, use `reduce-mismatch.py`. It takes a specific FP/FN example (from `investigate-cop.py` output) and automatically shrinks the source file using delta debugging until only the triggering pattern remains:

```
python3 scripts/reduce-mismatch.py Department/CopName repo_id filepath:line           # reduce FP (default)
python3 scripts/reduce-mismatch.py Department/CopName repo_id filepath:line --type fn  # reduce FN
python3 scripts/reduce-mismatch.py Department/CopName repo_id filepath:line --verbose  # show each step
```

Use this when `investigate-cop.py --context` shows an FP/FN in a large file and the root cause isn't obvious. The reduced output (typically 5–20 lines) makes the triggering pattern clear and can be pasted directly into test fixtures. See `docs/delta_reducer_plan.md` for design details.

## Corpus Regression Testing

After fixing any cop, run the corpus count check to verify no aggregate regression against the real-world repo corpus:

```
python3 scripts/check-cop.py Department/CopName                              # aggregate check (re-runs nitrocop)
python3 scripts/check-cop.py Department/CopName --verbose                     # per-repo breakdown (uses cached data if available)
python3 scripts/check-cop.py Department/CopName --verbose --rerun             # force re-execution after a fix (uses batch mode)
python3 scripts/check-cop.py Department/CopName --verbose --rerun --quick     # fast iteration: only repos with baseline activity
python3 scripts/check-cop.py Department/CopName --input results.json          # use local corpus-results.json
python3 scripts/check-cop.py Department/CopName --extended                    # use extended corpus (5k+ repos)
```

`check-cop.py` compares aggregate nitrocop offense counts against the RuboCop baseline from the latest CI corpus oracle run. It reports two things:

1. **vs RuboCop counts** — the aggregate offense-count delta: how many excess offenses nitrocop reports and how many RuboCop offenses nitrocop is still missing. Zero count delta is necessary, but exact locations may still differ.
2. **PASS/FAIL** — regression check: whether the current code is WORSE than the CI baseline. `PASS: no new excess vs CI nitrocop baseline` means you haven't introduced new aggregate excess offenses, but existing location mismatches or missing offenses may still remain. `PASS: aggregate offense count matches RuboCop for this cop` means the totals match for that cop, not that the exact locations are proven identical.

**Important:** `check-cop.py` is count-only. Use `python3 scripts/verify-cop-locations.py Department/CopName` to verify the known oracle FP/FN locations for a cop. For `/fix-department`, the final completion gate is regenerated `README.md` / `docs/corpus.md` via `cargo run --release --bin bench_nitrocop -- conform`.

With `--verbose`, it uses enriched per-repo data from `corpus-results.json` when available (instant). Pass `--rerun` to force re-execution of nitrocop after making code changes. `--rerun` automatically uses batch `--corpus-check` mode (single process) when available, falling back to per-repo subprocesses. Add `--quick` to skip repos with zero baseline activity (3-10x faster, may miss new FPs on zero-baseline repos).

**WARNING — "file-drop noise" is not an excuse for FN gaps.** `check-cop.py` reports a "file-drop noise" metric: the total offense count for repos where RuboCop's parser crashes on some files (e.g., jruby). This metric is subtracted from excess to avoid penalizing nitrocop for files RuboCop can't parse. However, agents frequently misinterpret this as explaining away large FN counts ("the ~41k missing offenses are just file-drop noise"). The file-drop noise number is the TOTAL offense count across all cops for affected repos — it is NOT the FN attributable to file drops for any specific cop. When `check-cop.py` reports `PASS: no new excess vs CI nitrocop baseline` with thousands of missing offenses, that means the cop has real implementation gaps, not noise. Always investigate the actual root cause of FN: use `investigate-cop.py` to see per-repo breakdowns, examine specific examples, and compare nitrocop's behavior against RuboCop's AST semantics. Do not declare a cop "done" or attribute FN to noise without concrete evidence.

## Rules

- **NEVER copy code or identifiers from private repos into this codebase.** When fixing false positives found by running against private/internal repos, write generic test cases that reproduce the same pattern. Use generic names (e.g. `records`, `payload`, `User`, `name`, `role`, `status`) instead of domain-specific names from the private codebase. Do not use variable names, method names, or terminology that originated in private repo source code — even if they seem generic, if you encountered them in a private repo, replace them. Do not reference private repo names or paths in committed files. This applies to test fixtures, comments, commit messages, and any other checked-in files.
- After adding a new cop, ensure `cargo test` passes — the `all_cops_have_minimum_test_coverage` integration test enforces that every cop has at least 3 offense fixture cases and 5+ non-empty lines in no_offense.rb. There are zero exemptions; use `offense/` scenario directories and `# nitrocop-expect:` annotations to handle cops that can't use the standard single-file format.
- **Use TDD when fixing cops.** Write the failing test case first (add to `offense.rb` or `no_offense.rb`), verify it fails, then implement the fix and confirm the test passes. This applies to both new detections and false-positive fixes.
- **Every cop fix or false-positive fix must include test coverage.** When fixing a false positive, add the previously-false-positive case to the cop's `no_offense.rb` fixture. When fixing a missed detection, add it to `offense.rb`. This prevents regressions and documents the expected behavior.
- **Don't remove or move test cases unless they are factually incorrect.** Existing offense and no_offense fixtures represent verified correct behavior. If a code change causes existing tests to fail, the change is likely too aggressive and introduces regressions (FPs or FNs on other repos). Fix the approach rather than deleting tests. The exception: if a test case is provably wrong (e.g., nitrocop was flagging something RuboCop doesn't flag), it should be moved to the correct fixture file (offense → no_offense or vice versa) with a clear explanation.
- **NEVER use `git stash` or `git stash pop`.** Work has been lost in the past from stash conflicts and forgotten stashes. Instead, commit work-in-progress to a branch, or use a worktree for parallel work. If you need to switch context, commit first with a WIP message.
- **Commit early and often.** Multiple agents frequently work on the same branch without worktree isolation. Uncommitted changes can be lost at any time — another agent's `git checkout`, a rebase, or a crash will destroy unstaged work. Commit after every meaningful step (tests added, fix implemented, even partial progress). Use WIP commit messages if the work isn't complete. Uncommitted work is unprotected work.
- **Do not pause for unrelated working-tree changes.** If you see modified files unrelated to your current task, continue the task, do not edit those files, do not stage/commit them, and do not revert them. Treat unrelated changes as off-limits unless explicitly asked to work on them.
- **NEVER run `git checkout -- .` or `git restore .` on a shared branch.** Other agents may have uncommitted work in the working tree. Resetting all files destroys their progress. Only checkout/restore specific files that you own. This includes broad directory restores like `git checkout HEAD -- src/` or `git checkout HEAD -- tests/` — these are equivalent to `git checkout -- .` for those directories and will wipe out other agents' uncommitted work across the entire directory tree. Always target individual files: `git checkout HEAD -- src/cop/lint/specific_file.rs`.
- **Document all cop investigation findings** as `///` doc comments on the cop's struct. This applies to all cop investigation work, not just `/fix-department` runs. Document root causes, fixes applied, remaining gaps, and whether issues are cop logic bugs vs config resolution problems. This prevents future investigators from repeating the same analysis.
- **Do not run local benchmark regeneration by default during cop-fix loops.**
  Use per-cop corpus gates (`scripts/check-cop.py ... --verbose [--rerun]`) as the default count-based acceptance check, and `scripts/verify-cop-locations.py` when you need location-level confirmation.
  Only run `cargo run --release --bin bench_nitrocop -- conform` when explicitly requested or when closing out `/fix-department`, since it rewrites `bench/results.md`.
- **When editing a skill, check for related skills that share conventions or interact with it.** Skills in `.claude/skills/` often form workflows (e.g., fix-department creates branches, land-branch-commits lands them). Changes to shared conventions (commit message format, Co-Authored-By trailers, branch naming) must be applied consistently across all related skills.
