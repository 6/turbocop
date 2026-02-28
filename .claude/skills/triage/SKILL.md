---
name: triage
description: Rank cops by FP+FN from the corpus oracle to find the next batch to fix
allowed-tools: Bash(*), Read, Grep, Glob
---

Run the triage script to find the top cops to fix:

```bash
python3 .claude/skills/triage/scripts/triage.py $ARGUMENTS
```

If no `--input` is provided, it auto-downloads the latest `corpus-results.json` from CI via `gh run download`.

Common invocations:
- `/triage` — top 30 cops by divergence
- `/triage --limit 50` — show more
- `/triage --department RSpec` — filter to one department
- `/triage --exclude-department Layout` — skip complex layout cops
- `/triage --fp-only` — only cops with false positives
- `/triage --input /tmp/corpus-report/corpus-results.json` — use local file

After identifying targets, use `investigate-cop.py` to see all FP/FN locations with source context:

```bash
python3 scripts/investigate-cop.py Department/CopName --context --fp-only
```

Then read the cop source in `src/cop/<dept>/<cop>.rs` and the vendor spec at `vendor/rubocop*/spec/` to understand the root cause before fixing.
