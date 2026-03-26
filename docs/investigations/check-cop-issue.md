# check_cop.py vs Corpus Oracle: File-Filtering Gap (RESOLVED)

## Status

**RESOLVED** — The oracle now stores `nitro_total_unfiltered` in per-cop artifact data (commit 82cedc66). check_cop.py compares against this unfiltered count, eliminating the structural gap. Verified passing locally and on CI for Style/MixinUsage.

## Root Cause

The corpus oracle's `diff_results.py` filters nitrocop offenses to only RuboCop-inspected files before counting (line 304-305):

```python
if rc_inspected_files:
    tc_offenses = {o for o in tc_offenses if o[0] in rc_inspected_files}
```

RuboCop drops files when its parser crashes mid-batch. The oracle correctly excludes nitrocop offenses on those dropped files from the match/FP/FN counts. But `check_cop.py` doesn't run RuboCop, so it counts ALL nitrocop offenses including those on dropped files. This produced a constant +34 offset for Style/MixinUsage (and varying offsets for other cops).

## Fix

The oracle now stores both counts per cop:
- `matches + fp` — the filtered count (after excluding RuboCop-dropped files)
- `nitro_total_unfiltered` — the raw count before filtering

`check_cop.py` compares against `nitro_total_unfiltered` when available, falling back to the filtered count for older artifacts.

## Failed Approaches

1. **GIT_CEILING_DIRECTORIES in oracle** (commit 9966179c) — No effect. The ceiling doesn't change file discovery when the target is passed as an absolute path.

2. **cd into repo in oracle (Option A)** (commit 12f56f2a, reverted 8c345a5b) — Dropped conformance from 98.9% to 0.0%. RuboCop's `--force-exclusion` and config path resolution completely break when cwd changes.

3. **Symlink to temp dir (Option B)** (commit a82349f8) — Per-repo counts matched the oracle, but the aggregate was still +34 because the gap comes from RuboCop file-filtering, not gitignore/path resolution.

## Verified Behavior (Post-Fix)

```
Results:
  Expected (RuboCop):          540
  Actual (nitrocop):           574
  CI nitrocop baseline:        540
  nitro_total_unfiltered:      574
  Excess (potential FP):        34

PASS: no regression vs CI baseline
```

The 34 "excess" is correctly identified as the file-filtering gap (not a regression) because `nitrocop_total (574) == nitro_total_unfiltered (574)`.
