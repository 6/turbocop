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

For GitHub-hosted execution:

```bash
gh workflow run investigate-regression.yml -f corpus=standard -f action=report
```

Decision rule:
- one merged bot PR candidate => strong revert candidate
- otherwise, simple issue => dispatch repair
- otherwise, reopen/comment the issue and stop with the report
