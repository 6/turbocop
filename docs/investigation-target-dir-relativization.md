# Investigation: target_dir relativization for cop Include patterns

**Status:** Understood, not fixable in oracle without upstream RuboCop changes
**Date:** 2026-03-26 (Sessions 1-3)

## Problem

When nitrocop runs via the corpus runner with an overlay config from a temp
directory, cop-level Include patterns fail to match files. This affects 20 Rails
cops whose Include patterns don't start with `**/` (e.g., `db/**/*.rb`).
Cops with `**/`-prefixed patterns (54 of 74 total) are unaffected.

The corpus runner invokes:
```
nitrocop --config /tmp/nitrocop_corpus_configs/overlay.yml /path/to/repo
# with cwd=/tmp
```

This sets:
- `config_dir` = `/tmp/nitrocop_corpus_configs/` (config file's parent)
- `base_dir` = `/tmp` (CWD, because config filename isn't `.rubocop*`)
- File paths are absolute: `/path/to/repo/app/controllers/foo.rb`

In `is_cop_match()` (src/config/mod.rs:294-343), file paths are relativized
against `config_dir` and `base_dir` via `strip_prefix`. Both fail because the
file isn't under either directory. Include patterns like `**/*.rb` compiled with
`literal_separator(true)` don't match the raw absolute path either.

Result: cops whose Include patterns don't start with `**/` are silently skipped.
Patterns starting with `**/` (e.g., `**/app/models/**/*.rb`) still match because
`**` consumes the absolute path prefix. This affects 20 cops, all in rubocop-rails
— not the originally estimated 47. See Session 3 for the `**/` prefix analysis.

Multiple agent investigations independently discovered this as the root cause
of their FN: Rails/Delegate (#202), Rails/EnvironmentVariableAccess (#216),
Security/Open (#228).

## What Was Tried

Added `target_dir` (the CLI positional argument, e.g., `/path/to/repo`) as a
fifth relativization attempt in `is_cop_match()`, `is_cop_excluded()`, and
`is_path_matched_by_cop_config()`. For `/path/to/repo/lib/foo.rb` with
`target_dir=/path/to/repo`, `strip_prefix` produces `lib/foo.rb` — which
matches Include patterns.

Changes (all in `src/config/mod.rs`, now reverted):
- Added `target_dir: Option<PathBuf>` to `ResolvedConfig` and `CopFilterSet`
- Populated from `load_config()`'s existing `target_dir` parameter
- Added `rel_to_target` to Include/Exclude checks in three functions
- 3 new unit tests, all 4,388 existing tests passed
- Validated with `check_cop.py --rerun --clone --sample 30` for Rails/Delegate
  and Rails/EnvironmentVariableAccess — both showed 0 new FP / 0 new FN

## What Went Wrong

The fix caused a massive FP regression in the full corpus oracle run:

| Metric | Before | After |
|--------|--------|-------|
| Conformance | 98.5% | 97.2% |
| FP total | 39,063 | 57,988 |
| FN total | 411,172 | 770,784 |
| Rails FP | 92 | 19,071 |
| 100% match repos | 441 | 266 |

Cops that were previously silently disabled (0 matches, 0 FP, 0 FN) started
running and produced thousands of FP:

| Cop | New FP | Notes |
|-----|--------|-------|
| Rails/ThreeStateBooleanColumn | 6,988 | Migration-only cop |
| Rails/ReversibleMigration | 4,589 | Migration-only cop |
| Rails/CreateTableWithTimestamps | 3,507 | Migration-only cop |
| Rails/Output | 1,425 | |
| Rails/ReversibleMigrationMethodDefinition | 829 | Migration-only cop |
| Rails/I18nLocaleAssignment | 783 | |
| Rails/NotNullColumn | 287 | Migration-only cop |
| Rails/TimeZoneAssignment | 277 | |
| Rails/AddColumnIndex | 205 | Migration-only cop |
| Rails/DangerousColumnNames | 171 | Migration-only cop |

## Why the Pre-merge Validation Missed This

`check_cop.py --rerun --clone --sample 30` was run for two cops and showed
"0 new FP / 0 new FN". This check compares per-repo counts against the old
oracle baseline. For cops with 0 baseline matches (like ThreeStateBooleanColumn),
adding thousands of FP shows as "0 new FP" because the baseline has no per-repo
data to compare against. The check declared victory; a full oracle run was needed
to catch the regression.

## Underlying Issues (resolved — see Sessions 2-3)

The `target_dir` relativization made Include patterns match correctly, which
exposed a deeper problem: nitrocop runs cops that RuboCop wouldn't, because
other gating mechanisms differ between them:

1. **Corpus config resolution mismatch**: The corpus runner uses a shared
   `baseline_rubocop.yml` that enables all cops. RuboCop's own per-repo config
   resolution may disable cops via the project's `.rubocop.yml`, `inherit_from`,
   or gem-level config that the overlay doesn't replicate.

2. **Migration cops gating**: Cops like `Rails/ReversibleMigration` are meant to
   only run on migration files. **Resolved**: The gating is via Include patterns
   (`db/**/*.rb`), not `MigratedSchemaVersion`. Both tools fail to resolve these
   patterns in the oracle → both skip them → symmetric. The "FP" in the target_dir
   run were actually correct offenses that RuboCop couldn't see.

3. **Corpus runner CWD**: `run_nitrocop.py` uses `cwd=/tmp` to "avoid .gitignore
   interference." **Resolved**: CWD only affects `base_dir` for config resolution,
   not file discovery (`WalkBuilder` walks from the target dir). Changing CWD to
   repo_dir fixes nitrocop's `base_dir` but NOT RuboCop's in the oracle (which has
   its own CWD). The in-repo config approach was tried (Session 2) and reverted
   because RuboCop ignores cop Exclude patterns regardless of `base_dir`.

## Possible Directions (status as of Session 3)

- **Fix the corpus runner CWD** — TRIED, insufficient. Fixes nitrocop's
  `base_dir` but not RuboCop's. Creates asymmetry.

- **Narrow the target_dir fix** — NOT TRIED. Could work in theory but the
  asymmetry problem (Session 2) means even correctly-scoped cops would diverge
  because RuboCop ignores cop Exclude patterns when `base_dir` is wrong.

- **Investigate why those cops produce FP** — RESOLVED. They weren't FP.
  The "FP" were real offenses that RuboCop also can't see (symmetric failure).
  See Session 2.

- **In-repo `.rubocop_corpus.yml` config** — TRIED, reverted. Fixed Include
  for both tools but exposed Exclude asymmetry (nitrocop correctly applies
  Exclude, RuboCop doesn't). Dropped conformance to 94.1%. See Session 2.

- **Match RuboCop's lenient behavior in nitrocop** — RESOLVED, no action
  needed. Nitrocop already matches RuboCop's behavior when `base_dir` can't
  resolve: Include patterns fail → cop skipped, Exclude patterns fail → cop
  runs. See Session 3.

## Investigation Session 2 (2026-03-26)

### Key Discovery: RuboCop has the same bug (symmetric failure)

The FP regression was NOT caused by migration cops having bad implementations.
It was caused by an **asymmetric fix**: the target_dir change only fixed
nitrocop, but the oracle's RuboCop invocation has the identical Include matching
failure.

In the corpus oracle workflow (`.github/workflows/corpus-oracle.yml:284-298`):
```
bundle exec rubocop --config "$REPO_CONFIG" ... "$ABS_DEST"
```

- `$REPO_CONFIG` is either `bench/corpus/baseline_rubocop.yml` or
  `/tmp/nitrocop_corpus_configs/corpus_config_xxx.yml`
- Neither starts with `.rubocop`, so RuboCop's `base_dir = Dir.pwd` = CI workspace
- For `repos/REPO_ID/db/migrate/xxx.rb`, RuboCop relativizes to
  `repos/REPO_ID/db/migrate/xxx.rb` (includes `repos/` prefix)
- Include pattern `db/**/*.rb` does NOT match `repos/REPO_ID/db/migrate/xxx.rb`

Both tools are symmetrically broken — 0 offenses for Include-gated cops. The
target_dir fix broke this symmetry: nitrocop found thousands of offenses that
RuboCop couldn't, all counted as FP.

### CWD does not affect file discovery

Confirmed that `WalkBuilder::new(dir)` in `src/fs.rs` walks from the target
directory, not CWD. The `.gitignore` concern in `run_nitrocop.py`'s `/tmp` CWD
is about config resolution (`base_dir`), not file discovery. Changing CWD to
the repo dir would fix `base_dir` for nitrocop but NOT for RuboCop in the oracle
(which has its own CWD).

### Recommended fix: in-repo config with `.rubocop*` name

Both tools have identical `base_dir` logic: if config filename starts with
`.rubocop`, then `base_dir = dirname(config_path)`. By writing the overlay as
`<repo_dir>/.rubocop_corpus.yml`:

1. `base_dir = repo_dir` for **both** tools
2. `strip_prefix(repo_dir)` succeeds for all repo files
3. Include patterns match correctly in both tools
4. No Rust code changes needed — fix is entirely in Python/CI layer
5. FP/FN delta reflects real implementation gaps, not config artifacts

### Oracle run #162 produced 0% conformance (pre-existing workflow bug)

The first oracle run after the config fix (run #162, PR #230) showed 0%
conformance with all 5,590 repos erroring: "No rubocop JSON output file".

Root cause: commit 8774941b ("Free disk in corpus collect-results") added
`rm -rf all-results/results/rubocop` BEFORE the diff step that reads from
`--rubocop-dir all-results/results/rubocop`. This pre-existing bug was masked
because the previous oracle run (#161) happened to run before that commit
landed. Fix: moved the cleanup to after the diff step.

### Oracle run #163 (PR #231): 94.1% conformance, down from 98.5%

After fixing the `rm -rf` bug (run #162 was 0% due to deleted rubocop results),
run #163 produced real data. Key numbers:

| Metric | Old | New | Delta |
|--------|-----|-----|-------|
| Match rate | 98.5% | 94.1% | -4.4% |
| Matches | 29,571,533 | 28,313,268 | -1,258,265 |
| FP | 39,063 | 40,168 | +1,105 |
| FN | 411,172 | 1,707,967 | +1,296,795 |
| 100% repos | 441 | 224 | -217 |
| Exact cops | 371 | 110 | -261 |

Department breakdown (FN changes):
- Style: 215,547 → 1,031,471 (+815,924)
- Layout: 173,726 → 408,407 (+234,681)
- Lint: 16,151 → 168,323 (+152,172)
- Metrics: 3,270 → 31,105 (+27,835)
- RSpec: 333 → 20,215 (+19,882)
- Naming: 1,221 → 19,830 (+18,609)
- Rails: 444 → 16,301 (+15,857, FP: 92 → 7,661)
- Performance: 416 → 6,679 (+6,263)
- Gemspec: 0 → 3,901
- Bundler: 0 → 743

FP is mostly flat (+1,105 overall), confirming both tools are symmetrically
fixed. But Rails FP jumped +7,569 (expected — migration cops now fire).

The massive FN increase affects ALL departments, including cops WITHOUT Include
patterns (Layout, Style, Lint, Metrics, Naming). This means the `base_dir`
change affects more than just cop-level Include patterns — it changes how
AllCops.Exclude and cop-level Exclude patterns resolve too. Resolved in
Session 3: RuboCop ignores cop Exclude patterns when `base_dir` is wrong,
creating asymmetry with nitrocop which correctly applies them.

### Key finding: RuboCop ignores cop Exclude patterns regardless of base_dir

Tested on dotenv: RuboCop produces identical offenses (2267) with either
config (baseline OR overlay). Its `base_dir_for_path_parameters` changes
(`/workspace` → `/tmp/dotenv_test`), but cop-level Exclude patterns like
`spec/**/*` on `Style/DocumentationMethod` have NO effect on RuboCop's output.

Meanwhile, nitrocop's offense count drops (2258 → 2244) with the overlay
because its `is_cop_match()` now correctly applies Exclude patterns.

This means:
- OLD oracle (baseline config): Both tools broken on patterns → both run cops
  on all files → offenses match → **inflated conformance**
- NEW oracle (overlay config): Only nitrocop fixed → it correctly excludes
  spec/test files → RuboCop still reports them → **asymmetric = FN increase**

The "symmetric fix" hypothesis was wrong. RuboCop's cop-level Include/Exclude
resolution works differently than nitrocop's — it doesn't use
`base_dir_for_path_parameters` for cop Exclude patterns.

### Resolution: reverted in-repo overlay config (2026-03-26)

Reverted the `.rubocop_corpus.yml` overlay approach. The config change only
fixed nitrocop's pattern resolution while RuboCop's was unaffected, creating
an asymmetry that dropped conformance from 98.5% to 94.1%. The 98.5% was
correct from the "does nitrocop match RuboCop" perspective — both tools ran
all cops on all files, and results mostly agreed.

The 20 Include-gated cops (originally estimated at 47 — see Session 3 for the
`**/` prefix analysis) have no corpus data. Nitrocop already matches RuboCop's
lenient behavior (Session 3), so no Rust changes are needed. The gap is purely
in corpus measurement, not in correctness for real-world usage.

Commits reverted: 3cc8bd0e, 9c7d3102, c19e6e8e.
Commits kept: acdb591e (oracle rm -rf bug fix), d11399d4 (cleanup removal).

## Investigation Session 3 (2026-03-26): RuboCop `relevant_file?` analysis

### Root cause: RuboCop's cop Exclude patterns silently fail

In `vendor/rubocop/lib/rubocop/cop/base.rb:286-297`, `relevant_file?`:
```ruby
def relevant_file?(file)
  return false unless target_satisfies_all_gem_version_requirements?
  return true unless @config.clusivity_config_for_badge?(self.class.badge)
  file == RuboCop::AST::ProcessedSource::STRING_SOURCE_NAME ||
    (file_name_matches_any?(file, 'Include', true) &&
      !file_name_matches_any?(file, 'Exclude', false))
end
```

When `base_dir` doesn't contain the file (the corpus oracle's situation):
1. `path_relative_to_config(file)` in `path_util.rb:25-26` catches `ArgumentError`
   and returns the unchanged absolute path
2. Relative Include patterns (e.g., `db/migrate/**/*.rb`) don't match absolute paths
   → `file_name_matches_any?` returns `true` (the default_result for Include) only
   when no patterns exist; when patterns exist but don't match → returns `false` → cop skipped
3. Relative Exclude patterns (e.g., `spec/**/*`) don't match absolute paths
   → `file_name_matches_any?` returns `false` (default) → `!false = true` → cop runs

**Nitrocop already matches this behavior.** In `src/config/mod.rs:294-343`, when
`strip_prefix` fails for all bases, only the raw absolute path is tried against
patterns. Relative patterns don't match it → same outcome as RuboCop. No Rust
code changes needed.

### Not all Include patterns are broken: `**/` prefix matters

Of 74 cops with Include patterns, only 20 have zero corpus activity. The
differentiator is the `**/` prefix:

- `**/app/models/**/*.rb` → `**` matches any path prefix including
  `/path/to/repo/` → WORKS even without relativization → 54 cops have data
- `db/**/*.rb` → requires path to start with `db/` → FAILS against absolute
  paths → 20 cops have zero data

All 20 zero-activity cops are in rubocop-rails (migration cops, test cops,
controller cops with non-`**/` Include patterns).

### Per-cop validation with `cwd=repo_dir`

Added `--repo-cwd` flag to `check_cop.py` (auto-enabled for Include-gated cops
with zero baseline). This passes `cwd=repo_dir` to `run_nitrocop.py`, making
`base_dir = repo_dir` so `strip_prefix` succeeds and Include patterns resolve.

Also modified `relevant_repos_for_cop()` to sample from the full manifest when a
cop has zero baseline data (since there are no "relevant" repos to filter to).

Validated with sample runs:
- `Rails/ReversibleMigration`: 1 offense in 15 repos (migration files)
- `Rails/ThreeStateBooleanColumn`: 3 offenses in 20 repos (migration files)
- `Rails/CreateTableWithTimestamps`: 2 offenses in 20 repos (migration files)
- `Rails/HttpPositionalArguments`: 0 offenses in 20 repos (expected — deprecated pattern)

This does NOT change the oracle workflow or conformance numbers. It provides a
separate validation path for the 20 Include-gated cops.

### Tooling added

- `scripts/list_include_gated_cops.py` — Lists all cops with Include patterns,
  cross-referenced against corpus data to show which have zero activity
- `scripts/check_cop.py --repo-cwd` — Runs nitrocop with `cwd=repo_dir` for
  correct Include pattern resolution

### Why this doesn't matter much in practice

These 20 cops work correctly in normal usage. When a user runs `nitrocop .`
from their project root with a `.rubocop.yml`, `base_dir` resolves to the
project root and all Include patterns match. The gap is **only** in corpus
measurement — and it's unfixable in the oracle without upstream RuboCop changes,
because RuboCop's `relevant_file?` ignores cop Exclude patterns when `base_dir`
can't resolve (creating asymmetry with any fix we apply to nitrocop).

The 98.5% conformance number is genuine from the "does nitrocop match RuboCop"
perspective. Both tools are symmetrically broken on these 20 cops — neither
runs them, so they contribute zero to both FP and FN.

### Recommended next step: parallel Include-gated comparison in the oracle workflow

A **parallel step in the corpus oracle workflow** for these 20 cops IS feasible,
unlike fixing the main comparison. The reason all previous main oracle fixes
failed was the **Exclude asymmetry** — fixing `base_dir` activated cop Exclude
patterns in nitrocop but not RuboCop, affecting 400+ cops across all departments.

These 20 cops are different: **19 of 20 have NO Exclude patterns.** Only
`Rails/CreateTableWithTimestamps` has one (narrow ActiveStorage exclusion). This
means fixing Include resolution for these cops won't trigger Exclude asymmetry.

#### How it integrates into the corpus oracle

The existing pipeline is:

```
oracle-repo jobs (batched)
  → per-repo nitrocop + rubocop JSON
    → diff_results.py
      → corpus-results.json  (central aggregation)
        ├→ gen_tiers.py       → tiers.json
        ├→ update_readme.py   → README.md tables
        ├→ cop_coverage.py    → docs/cop_coverage.md
        └→ corpus-results.md  → docs/corpus.md
```

The Include-gated comparison runs **in parallel** with the main oracle-repo
jobs, using the same cloned repos, and its results are **merged into
corpus-results.json** before the downstream scripts run. This means the 20
cops automatically appear in docs/corpus.md, README conformance tables, tier
classification, and cop coverage — no changes needed to downstream scripts.

```
oracle-repo jobs (batched, ALL cops)  ←── existing, unchanged
  → main-results.json

include-gated job (20 cops only)      ←── NEW parallel job
  → include-gated-results.json

merge step                             ←── NEW
  → corpus-results.json                    (replaces zero entries for 20 cops
                                            with real data from include-gated job)
  ↓
gen_tiers.py / update_readme.py / cop_coverage.py  ←── unchanged
```

#### Per-repo comparison approach

For each repo, the Include-gated job:

1. Writes `.rubocop_include_check.yml` inside the repo dir:
   ```yaml
   inherit_from: /absolute/path/to/baseline_rubocop.yml
   # The .rubocop* filename ensures base_dir = repo_dir for both tools
   ```

2. Runs RuboCop from the repo dir with `--only` for the 20 cops:
   ```
   cd /path/to/repo
   bundle exec rubocop --config .rubocop_include_check.yml \
     --only Rails/ReversibleMigration,Rails/ThreeStateBooleanColumn,... \
     --format json .
   ```

3. Runs nitrocop the same way (cwd=repo_dir):
   ```
   nitrocop --config .rubocop_include_check.yml \
     --only Rails/ReversibleMigration,Rails/ThreeStateBooleanColumn,... \
     --format json .
   ```

4. Both tools have `base_dir = repo_dir`, both resolve `db/**/*.rb` and
   `spec/**/*.rb` correctly, no Exclude asymmetry (19/20 have no Exclude).

#### Merge step

A new script (e.g., `bench/corpus/merge_include_gated.py`) replaces the
zero-data `by_cop` entries in main-results.json with real data from the
Include-gated job. It also updates:
- `by_department` — adds the new matches/FP/FN to the Rails department totals
- `by_repo_cop` — adds per-repo-per-cop divergence data for the 20 cops
- `cop_activity_repos` — adds repo lists for the 20 cops
- `summary` — adjusts totals (total_matches, total_fp, total_fn)

The merge is safe because the main oracle has exactly zero data for these 20
cops (both tools skip them), so there's no double-counting.

#### Why this works when the main oracle fix didn't

- `--only` restricts to cops with no Exclude patterns → no asymmetry
- In-repo `.rubocop*` config → `base_dir = repo_dir` for both tools
- CWD = repo_dir → matches normal user workflow
- Per-cop comparison → isolated from other cops' Exclude behavior
- Merge into corpus-results.json → downstream scripts pick it up automatically

#### Why this can't be folded into the main oracle comparison

The main oracle compares ALL cops at once without `--only`. Changing `base_dir`
in the main oracle activates Exclude patterns for ALL cops, creating asymmetry
in the 400+ cops that have Exclude patterns. The main comparison must stay as-is;
the Include-gated comparison runs alongside it.

#### Implementation scope

- **New workflow job** in `.github/workflows/corpus-oracle.yml`: runs after
  repos are cloned, in parallel with the main oracle-repo batches. Reuses the
  same cloned repos. Uses `--only` with the 20 cop names.
- **New script** `bench/corpus/merge_include_gated.py`: merges Include-gated
  results into the main corpus-results.json before downstream scripts run.
- **Existing infrastructure**: `run_nitrocop.py` already supports `cwd`
  parameter; `diff_results.py` output format is well-defined; downstream
  scripts need no changes.

### Quick plausibility check (nitrocop-only)

For a faster but less rigorous check, batch-run all 20 cops through
`check_cop.py --rerun --clone --sample 30` (nitrocop only, no RuboCop
comparison). This identifies broken implementations without the overhead of
running RuboCop:

```
scripts/list_include_gated_cops.py --json | \
  python3 -c "import json,sys; [print(c['cop']) for c in json.load(sys.stdin)]" | \
  while read cop; do
    python3 scripts/check_cop.py "$cop" --rerun --clone --sample 30
  done
```

## Key Code Locations

- `src/config/mod.rs:294-343` — `is_cop_match()` (Include/Exclude checking)
- `src/config/mod.rs:924-997` — `load_config()` (base_dir/config_dir setup)
- `src/config/mod.rs:534-553` — `build_glob_set()` with `literal_separator(true)`
- `src/config/mod.rs:984-997` — `base_dir` resolution: `.rubocop*` → config dir, else CWD
- `bench/corpus/run_nitrocop.py:87-121` — corpus runner (`cwd=/tmp`, `--config`)
- `bench/corpus/gen_repo_config.py` — overlay config generation
- `.github/workflows/corpus-oracle.yml:284-298` — oracle RuboCop invocation (same bug)
- `src/fs.rs:44-50` — file discovery (CWD-independent, uses walk root)
- `vendor/rubocop/lib/rubocop/cop/base.rb:286-297` — `relevant_file?` (cop Include/Exclude)
- `vendor/rubocop/lib/rubocop/path_util.rb:13-29` — `relative_path` (ArgumentError catch)
