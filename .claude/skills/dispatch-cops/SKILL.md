---
name: dispatch-cops
description: Dispatch cop-fix tasks to Codex agents (via GHA) for parallel corpus conformance fixes
allowed-tools: Bash(*), Read, Grep, Glob, AskUserQuestion
---

# Dispatch Cops — Remote Agent Orchestration

Dispatch cop-fix tasks to AI agents (running in GitHub Actions) to fix
corpus conformance gaps in parallel. The current system uses one GitHub issue
per diverging cop as a durable backlog item. Dispatchers fill a bounded queue
from those issues, then `agent-cop-fix` opens one PR per cop and
`agent-pr-repair` reacts to failed deterministic CI.

See `docs/agent-dispatch.md` for full setup instructions and architecture.

## Prerequisites

Before dispatching, verify the pipeline is set up:

```bash
# Verify the workflows exist
ls .github/workflows/agent-cop-fix.yml \
   .github/workflows/agent-pr-repair.yml \
   .github/workflows/cop-issue-sync.yml \
   .github/workflows/cop-issue-dispatch.yml
```

The user needs `CODEX_AUTH_JSON` configured in GitHub repo secrets.
See `docs/agent-dispatch.md` for setup instructions.

## Phases

### Phase 1: Triage / Sync

Inspect the current dispatchable set and sync the tracker issues:

```bash
python3 scripts/dispatch-cops.py rank
gh workflow run cop-issue-sync.yml -f corpus=extended
```

This runs pre-diagnostic on every cop's FP/FN examples to classify them as
code bugs (agent can fix) vs config/context issues (agent can't). Only shows
cops with at least 1 real code bug.

For the lighter Codex lane, prefer cops with 3-10 total FP+FN and mostly code bugs:

```bash
python3 scripts/dispatch-cops.py rank --min-bugs 2 --max-total 10
```

For harder cops or overview by tier:

```bash
python3 scripts/dispatch-cops.py tiers --extended --tier 1   # simple FP+FN count view
python3 scripts/investigate-cop.py Department/CopName --extended --context  # deep dive
```

**Skip cops with 0 code bugs** — they're all config issues and the workflow
will auto-skip them anyway (pre-diagnostic gate).

The sync workflow creates or updates one `[cop] Department/CopName` issue per
diverging cop, reopens old issues when a cop regresses again, and applies a
durable difficulty label (`difficulty:simple|medium|complex`). The actual
backend is chosen later by `agent-cop-fix` when the issue is dispatched.

### Phase 2: Dispatch

Fill the bounded active queue from backlog issues:

```bash
gh workflow run cop-issue-dispatch.yml -f max_active=5
```

Dry run first if you want to inspect the selected queue:

```bash
gh workflow run cop-issue-dispatch.yml -f max_active=5 -f dry_run=true
```

If you need to force one backend across the dispatched issues:

```bash
gh workflow run cop-issue-dispatch.yml -f max_active=5 -f backend_override=codex -f strength_override=normal
gh workflow run cop-issue-dispatch.yml -f max_active=5 -f backend_override=codex -f strength_override=hard
```

### Phase 3: Review + Merge

Monitor progress:

```bash
# Open PRs
gh pr list --state open --limit 50

# PRs with passing CI
gh pr list --state open --search "status:success" --limit 50
```

Help the user review and merge PRs. For each PR:

```bash
gh pr view <number>
gh pr checks <number>
gh pr diff <number>
```

If CI passes and the diff looks right:

```bash
gh pr merge <number> --squash
```

### Phase 4: Retry Failures

Find cops with failed PRs:

```bash
gh pr list --state open --search "status:failure" --limit 50
```

Retry each with the stronger Codex model:

```bash
gh workflow run agent-cop-fix.yml -f cop="Department/CopName" -f mode=retry
```

For specific issues, add context:

```bash
gh workflow run agent-cop-fix.yml \
  -f cop="Department/CopName" \
  -f mode=retry \
  -f extra_context="<what went wrong>"
```

### Phase 5: Validate

After merging a batch (~20-50 PRs), run the full corpus oracle:

```bash
gh workflow run corpus-oracle.yml -f corpus_size=extended
```

Wait ~90 min, then check results:

```bash
python3 scripts/dispatch-cops.py tiers --extended
```

## Arguments

- `/dispatch-cops` — start from Phase 1 (triage / issue sync)
- `/dispatch-cops sync` — jump to Phase 1 (sync/update cop tracker issues)
- `/dispatch-cops dispatch` — jump to Phase 2 (fill bounded queue from backlog issues)
- `/dispatch-cops retry` — jump to Phase 4 (retry failures)
- `/dispatch-cops status` — show current PR status and merge candidates
- `/dispatch-cops validate` — jump to Phase 5 (trigger corpus oracle)

## Important Notes

- Codex runs inside GHA — full Rust build environment with cache
- The task prompt contains all context the agent needs
- `workflow_dispatch` requires write access — safe on public repos
- Retries auto-close stale PRs and include prior attempt context
- `agent-cop-fix` now supports `backend=auto`; simple issue-backed fixes use `gpt-5.3-codex`, while harder fixes and repairs use `gpt-5.4`
- `claude` and `minimax` manual overrides still exist for experiments, but do not use them as the default recommendation
- Tracker issues should be created by the GitHub App (`6[bot]`), not manually
- Monitor ChatGPT usage at chatgpt.com/codex/settings/usage — dispatch in small batches
