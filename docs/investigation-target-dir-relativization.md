# Investigation: target_dir relativization for cop Include patterns

**Status:** Reverted (commit 93d80fad reverts a81e1179)
**Date:** 2026-03-26

## Problem

When nitrocop runs via the corpus runner with an overlay config from a temp
directory, cop-level Include patterns fail to match files. This causes systemic
FN across cops that use Include patterns (most Rails/*, some RSpec/*).

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

Result: every cop with Include patterns is silently skipped for all files.

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

## Underlying Issues (not yet resolved)

The `target_dir` relativization made Include patterns match correctly, which
exposed a deeper problem: nitrocop runs cops that RuboCop wouldn't, because
other gating mechanisms differ between them:

1. **Corpus config resolution mismatch**: The corpus runner uses a shared
   `baseline_rubocop.yml` that enables all cops. RuboCop's own per-repo config
   resolution may disable cops via the project's `.rubocop.yml`, `inherit_from`,
   or gem-level config that the overlay doesn't replicate.

2. **Migration cops gating**: Cops like `Rails/ReversibleMigration` are meant to
   only run on migration files. RuboCop may skip them via `MigratedSchemaVersion`
   or other project-specific config that the corpus baseline doesn't set per-repo.

3. **Corpus runner CWD**: `run_nitrocop.py` uses `cwd=/tmp` to "avoid .gitignore
   interference." This is what makes `base_dir` resolve to `/tmp` instead of the
   repo root. Changing CWD to the repo dir might fix the relativization without
   needing `target_dir`, but the .gitignore concern hasn't been investigated.

## Possible Directions

These are starting points, not prescriptions — the right fix may be something
different entirely.

- **Fix the corpus runner instead of nitrocop**: Change `run_nitrocop.py` to
  set `cwd` to the repo directory instead of `/tmp`. This would make `base_dir`
  resolve to the repo root naturally, without adding `target_dir` to the core.
  The `/tmp` CWD was chosen to "avoid .gitignore interference" but that concern
  may no longer apply.

- **Narrow the target_dir fix**: Instead of enabling it for all cops, only use
  `target_dir` relativization when the cop already has non-zero baseline matches
  in the oracle. This prevents silent cops from suddenly firing while fixing FN
  for cops that should have been matching.

- **Investigate why those cops produce FP**: The migration-only cops
  (ThreeStateBooleanColumn, ReversibleMigration, etc.) produced thousands of FP.
  Understanding why RuboCop doesn't fire them (MigratedSchemaVersion? project
  config? gem version gating?) would clarify whether the fix needs a guard or
  whether those cops have a separate implementation gap.

- **Different config approach for corpus**: Rather than a temp overlay config,
  the corpus runner could write a `.rubocop.yml` inside the repo directory
  itself (cleaning up after). This would make `config_dir` resolve to the repo
  root, sidestepping the relativization issue entirely.

## Key Code Locations

- `src/config/mod.rs:294-343` — `is_cop_match()` (Include/Exclude checking)
- `src/config/mod.rs:924-997` — `load_config()` (base_dir/config_dir setup)
- `src/config/mod.rs:534-553` — `build_glob_set()` with `literal_separator(true)`
- `bench/corpus/run_nitrocop.py:87-121` — corpus runner (`cwd=/tmp`, `--config`)
- `bench/corpus/gen_repo_config.py` — overlay config generation
