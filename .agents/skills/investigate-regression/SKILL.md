---
name: investigate-regression
description: Investigate a corpus regression between two corpus-oracle runs, reopen the linked cop issue, and determine whether to dispatch a repair or surface a strong revert candidate.
---

# Investigate Regression

Use the deterministic regression investigator first. Do not start with ad hoc GitHub browsing.

## Primary command

```bash
python3 scripts/investigate_regression.py --repo 6/nitrocop --corpus standard
```

This compares the latest two successful corpus-oracle runs for that corpus,
lists regressed cops, links them to tracker issues, and surfaces:
- candidate merged bot PRs in the run window
- candidate commits touching the regressed cop
- a suggested action:
  - `dispatch_repair`
  - `strong_revert_candidate`
  - `manual_investigation`

## Mutating modes

Reopen/comment linked issues for regressed cops:

```bash
python3 scripts/investigate_regression.py --repo 6/nitrocop --action reopen
```

Reopen issues and dispatch simple regressions back into `agent-cop-fix`:

```bash
python3 scripts/investigate_regression.py --repo 6/nitrocop --action dispatch-simple
```

## Per-PR regressions (cop-check CI failures)

When a PR's cop-check CI job fails, follow this workflow:

### Step 1: Identify the regression from CI logs

```bash
gh pr checks <pr-number>                                    # find failed job ID
gh run view <run-id> --job <job-id> --log 2>&1 | grep -A 3 "FAIL:"
```

The FAIL line names the regressed repo(s) and shows counts:
```
FAIL: FN regression (+1) in: ryuzee__SlideHub__315be3f
  +   1  ryuzee__SlideHub__315be3f  (local=6, baseline_nc=7, rubocop=7)
```

Do NOT re-run `check_cop.py --rerun --clone` locally — CI already did the
work. Reading its logs takes seconds vs minutes for a local rerun.

### Step 2: Understand what changed

```bash
# What did the PR change in the cop?
git diff main...HEAD -- src/cop/<dept>/<cop_name>.rs

# What does the regressed repo's file look like?
python3 scripts/investigate_cop.py Department/CopName --context --limit 0 2>&1 | grep -A 15 "<repo_id>"
```

Compare the regressed repo's pattern against the PR's fix logic. The
regression is usually a side effect of the fix being too broad — e.g.,
an operator-write kill that suppresses a legitimate offense along with
the false positive it was targeting.

### Step 3: Fix or revert

- If the regression is a narrow side effect of a correct fix, patch the
  fix to be more precise and push to the same PR branch.
- If the fix approach is fundamentally flawed, revert the cop change but
  keep test fixtures and investigation comments.
- Document the regression and its root cause in the cop's doc comments
  regardless of which path you take.

## Decision rule

- If there is exactly one merged bot PR candidate for the regressed cop in the run window, treat that as a strong revert candidate.
- Otherwise, if the linked issue is `difficulty:simple`, prefer dispatching a repair.
- Otherwise, reopen/comment the issue and stop with the report.
