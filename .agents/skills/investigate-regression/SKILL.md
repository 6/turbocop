---
name: investigate-regression
description: Investigate a corpus regression between two corpus-oracle runs, reopen the linked cop issue, and determine whether to dispatch a repair or surface a strong revert candidate.
---

# Investigate Regression

Start with the deterministic script, not ad hoc browsing:

```bash
python3 scripts/investigate_regression.py --repo 6/nitrocop```

If mutation is desired:

```bash
python3 scripts/investigate_regression.py --repo 6/nitrocop --corpus standard --action reopen
python3 scripts/investigate_regression.py --repo 6/nitrocop --corpus standard --action dispatch-simple
```

## Per-PR regressions (cop-check CI failures)

When investigating a regression on a specific PR (not a corpus-oracle diff),
check CI logs first — they already name the regressed repo(s):

```bash
gh pr checks <pr-number>                                    # find failed job
gh run view <run-id> --job <job-id> --log 2>&1 | grep -A 3 "FAIL:"
```

This is faster than re-running `check_cop.py --rerun --clone` locally, which
clones hundreds of repos. CI already did the work — just read its output.

Decision rule:
- one merged bot PR candidate => strong revert candidate
- otherwise, simple issue => dispatch repair
- otherwise, reopen/comment the issue and stop with the report
