# check-cop.py vs Corpus Oracle: 38-Offense Discrepancy

## Summary

`check-cop.py --rerun` consistently reports fewer offenses than the corpus oracle for some cops (e.g., Style/MixinUsage: 502 vs 540). This causes agent PRs to fail the CI cop-check gate even when the cop fix is correct.

## Root Cause

The corpus oracle and `check-cop.py` run nitrocop with different working directory contexts, which changes how the `ignore` crate's `WalkBuilder` resolves `.gitignore` files during file discovery.

### Corpus Oracle (`.github/workflows/corpus-oracle.yml`)

```bash
# Clones repos to repos/<id>/ (NOT under vendor/)
DEST="repos/${REPO_ID}"
git init "$DEST"
git -C "$DEST" fetch --depth 1 "$REPO_URL" "$REPO_SHA"
git -C "$DEST" checkout FETCH_HEAD

# Runs nitrocop from workspace root, passing repo path as argument
env BUNDLE_GEMFILE=$PWD/bench/corpus/Gemfile \
    BUNDLE_PATH=$PWD/bench/corpus/vendor/bundle \
bin/nitrocop --preview --format json --no-cache \
    --config "$REPO_CONFIG" "$DEST"
```

Key: `cwd` is the workspace root. The target path is `repos/<id>/`. The `WalkBuilder` starts walking from `repos/<id>/` and enters the repo's `.git` tree. However, because `cwd` is outside the repo, the `ignore` crate's gitignore resolution behaves differently — it may not fully respect the repo's `.gitignore` for files that are gitignored but physically present in the shallow clone.

### check-cop.py (per-repo mode)

```bash
# Clones repos to vendor/corpus/<id>/
# Runs nitrocop from INSIDE the repo
cd vendor/corpus/<id>
env BUNDLE_GEMFILE=.../bench/corpus/Gemfile \
    BUNDLE_PATH=.../bench/corpus/vendor/bundle \
    GIT_CEILING_DIRECTORIES=.../vendor/corpus \
nitrocop --only Style/MixinUsage --preview --format json --no-cache \
    --config .../bench/corpus/baseline_rubocop.yml .
```

Key: `cwd` is inside the repo. The `WalkBuilder` starts from `.` and immediately finds `.git/` and `.gitignore`. Gitignored files (like `bin/update` in some repos) are properly skipped.

## The Discrepancy

The oracle processes gitignored files because `WalkBuilder` doesn't fully apply `.gitignore` when the walk starts from outside the repo's `.git` tree. RuboCop in the oracle also runs from outside, so both tools agree — the oracle is internally consistent at 540.

`check-cop.py` runs from inside the repo, so `.gitignore` is properly applied and gitignored files are skipped. This gives 502 (38 fewer).

## Affected Repos (Style/MixinUsage example)

The 38 missing offenses come from files like:
- `bin/update` (gitignored in many Rails repos)
- Other files under gitignored paths that still exist in the shallow clone

These files contain top-level `include` calls that Style/MixinUsage flags. The oracle sees them, check-cop doesn't.

## Verified Behavior

| Invocation | MixinUsage Count | Notes |
|---|---|---|
| Oracle on CI (`repos/<id>/` from workspace root) | 540 | Gitignored files processed |
| check-cop per-repo (`cwd=vendor/corpus/<id>/`, `.`) | 502 | Gitignored files skipped |
| check-cop from CORPUS_DIR (`vendor/corpus/`, `<id>`) | 574 | Parent .gitignore context changes results |
| Batch `--corpus-check` (removed) | 587 | `vendor/**/*` exclude applied incorrectly |
| From `/tmp` with absolute path | 574 | Same as CORPUS_DIR |
| Repo copied outside project tree | matches oracle | Confirms it's a path/git context issue |

## Previously Fixed Issues

1. **Batch mode (`--corpus-check`)**: Applied `AllCops.Exclude: vendor/**/*` against `vendor/corpus/<id>/...` paths, incorrectly excluding entire repos. Produced 587 (47 extra FPs). **Fixed**: batch mode removed entirely.

2. **`[skip ci]` in squash merges**: The claim-pr placeholder commit included `[skip ci]` which poisoned merged PR commit messages, preventing CI from running. **Fixed**: removed `[skip ci]` from placeholder.

## Options to Fix

### Option A: Fix the oracle to run from inside each repo (recommended)

Change `corpus-oracle.yml` to `cd` into each repo before running nitrocop:

```bash
cd "$DEST"
env BUNDLE_GEMFILE=$WORKSPACE/bench/corpus/Gemfile \
    BUNDLE_PATH=$WORKSPACE/bench/corpus/vendor/bundle \
    GIT_CEILING_DIRECTORIES=$(dirname "$PWD") \
$WORKSPACE/bin/nitrocop --preview --format json --no-cache \
    --config "$REPO_CONFIG" .
```

This would make the oracle respect `.gitignore` properly, matching real-world `rubocop .` behavior. Baselines would drop by ~38 for Style/MixinUsage (and potentially small amounts for other cops). All cop baselines would need a refresh.

Do the same for the RuboCop invocation in the oracle so both tools continue to agree.

### Option B: Make check-cop replicate the oracle's outside-repo behavior

Clone repos to a temp directory outside the git tree (e.g., `/tmp/corpus/<id>/`) and run from the parent directory with the repo as a target. This matches the oracle's path context exactly.

Downside: slower (copies repos to /tmp), and preserves the oracle's arguably-wrong behavior of processing gitignored files.

### Option C: Allow a threshold in CI cop-check

Add `--threshold 38` to the CI cop-check for affected cops. Quick but fragile — the exact number depends on which repos have gitignored files with offenses.

### Option D: Disable gitignore in WalkBuilder for corpus runs

Add a `--no-gitignore` flag to nitrocop that disables `.gitignore` respect in `WalkBuilder`. Use it in both the oracle and check-cop. This makes both tools consistently process all files regardless of `.gitignore`.

Downside: diverges from real-world behavior where users expect `.gitignore` to be respected.

## Debugging on CI

CI runners use the same Ubuntu 24.04 environment as the oracle and cop-check. To debug further:

1. **Create a branch with verbose logging** and push it to trigger CI:
   ```bash
   # Add debug output to check-cop.py or the checks workflow
   git checkout -b debug/check-cop-discrepancy
   # Edit .github/workflows/checks.yml to add verbose logging
   # Or add a one-off debug step that runs both invocation styles
   git push origin debug/check-cop-discrepancy
   ```

2. **Add a debug step to checks.yml** that compares oracle-style vs check-cop-style for a specific repo:
   ```yaml
   - name: Debug corpus invocation difference
     run: |
       REPO="autolab__Autolab__674efe9"
       # Oracle style: from workspace root
       target/release/nitrocop --only Style/MixinUsage --preview --format json \
         --no-cache --config bench/corpus/baseline_rubocop.yml \
         "vendor/corpus/$REPO" 2>/dev/null | python3 -c "
           import json,sys; d=json.loads(sys.stdin.read())
           print(f'outside: {len(d.get(\"offenses\",[]))}')"
       # Check-cop style: from inside repo
       cd "vendor/corpus/$REPO"
       env GIT_CEILING_DIRECTORIES=$(dirname "$PWD") \
         "$GITHUB_WORKSPACE/target/release/nitrocop" \
         --only Style/MixinUsage --preview --format json \
         --no-cache --config "$GITHUB_WORKSPACE/bench/corpus/baseline_rubocop.yml" \
         . 2>/dev/null | python3 -c "
           import json,sys; d=json.loads(sys.stdin.read())
           print(f'inside: {len(d.get(\"offenses\",[]))}')"
   ```

3. **Compare file discovery** by adding `--debug` to nitrocop, which prints per-phase timing and file counts. This reveals whether the difference is in file discovery or offense detection.

This is useful because the discrepancy may behave differently on macOS vs Linux due to filesystem case sensitivity, symlink handling, or the `ignore` crate's platform-specific behavior.

## Recommendation

**Option A** is the cleanest fix. The oracle should match real-world usage (running from inside the repo). The one-time baseline refresh is a small cost. Both the oracle's nitrocop and RuboCop invocations should be updated to run from inside each repo with `GIT_CEILING_DIRECTORIES` set.
