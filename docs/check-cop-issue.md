# check-cop.py vs Corpus Oracle: 38-Offense Discrepancy

## Status

**OPEN** — `GIT_CEILING_DIRECTORIES` was added to the oracle (commit 9966179c) and rerun (run 23475296495). The oracle still reports 540, meaning `GIT_CEILING_DIRECTORIES` alone does not change how the `ignore` crate resolves `.gitignore` when the target is passed as an absolute path. The discrepancy persists.

## Summary

`check-cop.py --rerun` consistently reports fewer offenses than the corpus oracle for some cops (e.g., Style/MixinUsage: 502 vs 540). This causes agent PRs to fail the CI cop-check gate even when the cop fix is correct.

## Root Cause

The corpus oracle and `check-cop.py` run nitrocop with different working directory contexts, which changes how the `ignore` crate's `WalkBuilder` resolves `.gitignore` files during file discovery.

The oracle's shallow clones (`git init` + `git fetch --depth 1` + `git checkout FETCH_HEAD`) contain gitignored files physically on disk (e.g., `bin/update`). Whether these files are skipped depends on how the `ignore` crate enters the `.git` tree.

### Corpus Oracle (`.github/workflows/corpus-oracle.yml`)

```bash
DEST="repos/${REPO_ID}"
git init "$DEST"
git -C "$DEST" fetch --depth 1 "$REPO_URL" "$REPO_SHA"
git -C "$DEST" checkout FETCH_HEAD

# Runs from workspace root, passing repo as absolute path
env BUNDLE_GEMFILE=$PWD/bench/corpus/Gemfile \
    BUNDLE_PATH=$PWD/bench/corpus/vendor/bundle \
    GIT_CEILING_DIRECTORIES=$(dirname "$ABS_DEST") \
bin/nitrocop --preview --format json --no-cache \
    --config "$REPO_CONFIG" "$ABS_DEST"
```

Result: 540 offenses for Style/MixinUsage. Gitignored files ARE processed.

### check-cop.py (per-repo mode)

```bash
# Runs from INSIDE the repo with . as target
cd vendor/corpus/<id>
env BUNDLE_GEMFILE=.../bench/corpus/Gemfile \
    BUNDLE_PATH=.../bench/corpus/vendor/bundle \
    GIT_CEILING_DIRECTORIES=.../vendor/corpus \
nitrocop --only Style/MixinUsage --preview --format json --no-cache \
    --config .../bench/corpus/baseline_rubocop.yml .
```

Result: 502 offenses for Style/MixinUsage. Gitignored files ARE skipped.

## Why GIT_CEILING_DIRECTORIES Didn't Fix It

`GIT_CEILING_DIRECTORIES` prevents git from looking for `.git` directories above the ceiling. But the oracle passes the repo as an absolute path target, so the `ignore` crate's `WalkBuilder` starts walking from the repo directory and finds `.git` inside it. The ceiling is set to the parent of the repo dir, which doesn't affect this walk.

The difference is more subtle: when `cwd` is inside the repo (check-cop style), the `ignore` crate resolves `.gitignore` relative to the working directory and correctly skips gitignored files. When `cwd` is outside the repo (oracle style), even though the walker enters the repo and finds `.git`, the gitignore resolution may differ — possibly because the `ignore` crate uses `cwd` as part of its path normalization.

## Verified Behavior

| Invocation | MixinUsage Count | Notes |
|---|---|---|
| Oracle on CI (`repos/<id>/` from workspace root) | 540 | Gitignored files processed |
| Oracle on CI with `GIT_CEILING_DIRECTORIES` | 540 | No change — ceiling doesn't affect it |
| check-cop per-repo (`cwd=repo`, `.` target) | 502 | Gitignored files properly skipped |
| check-cop from CORPUS_DIR (`vendor/corpus/`, `<id>` target) | 574 | Parent gitignore context adds extras |
| Batch `--corpus-check` (removed) | 587 | `vendor/**/*` exclude incorrectly applied |
| From `/tmp` with absolute path | 574 | Same as CORPUS_DIR |
| Repo copied outside project tree to `/tmp` | matches oracle (540) | Confirms it's a cwd/git context issue |

### Per-repo example: autolab/Autolab

```
# From inside repo (check-cop style): 15 offenses — bin/update skipped
# From outside repo (oracle style):   16 offenses — bin/update included
# The file bin/update is gitignored but present in shallow clone
```

## Previously Fixed Issues

1. **Batch mode (`--corpus-check`)**: Applied `AllCops.Exclude: vendor/**/*` against `vendor/corpus/<id>/...` paths, incorrectly excluding entire repos. Produced 587 (47 extra FPs). **Fixed**: batch mode removed entirely (commit 22b833ed).

2. **`[skip ci]` in squash merges**: The claim-pr placeholder commit included `[skip ci]` which poisoned merged PR commit messages, preventing CI from running. **Fixed**: removed `[skip ci]` from placeholder (commit 558ab7e1).

3. **`GIT_CEILING_DIRECTORIES` in oracle**: Added to both nitrocop and RuboCop invocations (commit 9966179c). **Did not fix the discrepancy** — oracle still reports 540.

## Options to Fix

### Option A: Run oracle from inside each repo (cd into DEST) — FAILED

Tried in commit 12f56f2a, reverted in 8c345a5b. Changing the oracle to `cd "$DEST"` and use `.` as the target dropped conformance from **98.9% to 0.0%** — RuboCop's config resolution completely broke when `cwd` changed. The oracle's invocation style (cwd=workspace root, repo as path argument) is load-bearing for RuboCop and cannot be changed.

### Option B: Make check-cop replicate the oracle's outside-repo behavior

Clone repos to a temp directory outside the git tree (e.g., `/tmp/corpus/<id>/`) and run from a neutral parent. This matches the oracle's cwd-outside-repo behavior.

**Implemented** in commit a82349f8. Each per-repo run symlinks the corpus repo into
a temp directory outside the project tree and runs from there, matching the oracle's
`repos/<id>/` path context.

**Local testing (macOS)**: Per-repo counts match the oracle exactly (e.g., autolab=16,
puppet=29, dpl=35). But the aggregate total is 574 vs oracle's 540 — a 34-offense gap
that may be a macOS vs Linux difference in how the `ignore` crate resolves symlinks
or `.gitignore`. **CI (Linux) is the real test** — merge main into PR #151 and see if
cop-check passes.

### Option C: Allow a per-cop threshold in CI cop-check

Add `--threshold N` to the CI cop-check for cops known to have this discrepancy. The number would need to be computed from the oracle data.

**Downside**: Fragile, masks real regressions up to the threshold.

### Option D: Add `--no-gitignore` flag to nitrocop

Disable `.gitignore` respect in `WalkBuilder` for corpus runs. Use in both the oracle and check-cop.

**Downside**: Diverges from real-world behavior. Would need to be added to the Rust code.

## Debugging on CI

CI runners use the same Ubuntu 24.04 environment as the oracle and cop-check. **This discrepancy must be debugged on CI** since local macOS may behave differently.

1. **Create a debug branch** with a one-off workflow step that compares both invocation styles:

   ```yaml
   - name: Debug corpus invocation difference
     run: |
       # Clone a known-affected repo
       REPO="autolab__Autolab__674efe9"
       mkdir -p vendor/corpus
       git init "vendor/corpus/$REPO"
       git -C "vendor/corpus/$REPO" fetch --depth 1 \
         https://github.com/autolab/Autolab.git 674efe9
       git -C "vendor/corpus/$REPO" checkout FETCH_HEAD

       echo "=== Oracle style: cwd=workspace, target=path ==="
       env BUNDLE_GEMFILE=$PWD/bench/corpus/Gemfile \
           BUNDLE_PATH=$PWD/bench/corpus/vendor/bundle \
       target/release/nitrocop --only Style/MixinUsage --preview \
         --format json --no-cache \
         --config bench/corpus/baseline_rubocop.yml \
         "vendor/corpus/$REPO" 2>/dev/null | python3 -c "
           import json,sys; d=json.loads(sys.stdin.read())
           offs=[o['path']+':'+str(o['line']) for o in d.get('offenses',[])]
           print(f'oracle-style: {len(offs)} offenses')
           for o in offs: print(f'  {o}')"

       echo "=== Check-cop style: cwd=repo, target=. ==="
       cd "vendor/corpus/$REPO"
       env BUNDLE_GEMFILE=$GITHUB_WORKSPACE/bench/corpus/Gemfile \
           BUNDLE_PATH=$GITHUB_WORKSPACE/bench/corpus/vendor/bundle \
           GIT_CEILING_DIRECTORIES=$GITHUB_WORKSPACE/vendor/corpus \
       $GITHUB_WORKSPACE/target/release/nitrocop --only Style/MixinUsage \
         --preview --format json --no-cache \
         --config $GITHUB_WORKSPACE/bench/corpus/baseline_rubocop.yml \
         . 2>/dev/null | python3 -c "
           import json,sys; d=json.loads(sys.stdin.read())
           offs=[o['path']+':'+str(o['line']) for o in d.get('offenses',[])]
           print(f'check-cop-style: {len(offs)} offenses')
           for o in offs: print(f'  {o}')"
   ```

2. **Verify Option A on CI** by testing `cd "$DEST"` + `.` target in the debug branch before changing the real oracle.

3. **Compare file discovery** by adding `--debug` to nitrocop in both modes — this prints file counts per phase and reveals whether the difference is in discovery or offense detection.

## Next Steps

1. **Rerun the standard corpus oracle** to restore the known-good baseline (the Option A attempt corrupted it).

2. **Try Option B**: Make check-cop.py match the oracle by running from a neutral directory outside the git tree with the repo as a path argument. This avoids changing the oracle at all.

3. If Option B doesn't work, **try Option D**: Add `--no-gitignore` to nitrocop for corpus runs so both tools consistently process all files regardless of `.gitignore`.

4. Once check-cop matches the oracle, **merge PR #151** (Style/MixinUsage) and **re-dispatch** remaining cops.
