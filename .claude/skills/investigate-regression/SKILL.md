---
name: investigate-regression
description: Investigate a corpus regression between two corpus-oracle runs, reopen the linked cop issue, and determine whether to dispatch a repair or surface a strong revert candidate.
allowed-tools: Bash(*), Read, Grep, Glob, AskUserQuestion
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

## Workflow entrypoint

For GitHub-hosted execution, use:

```bash
gh workflow run investigate-regression.yml -f corpus=standard -f action=report
```

## Decision rule

- If there is exactly one merged bot PR candidate for the regressed cop in the run window, treat that as a strong revert candidate.
- Otherwise, if the linked issue is `difficulty:simple`, prefer dispatching a repair.
- Otherwise, reopen/comment the issue and stop with the report.
