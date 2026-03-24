---
name: ci-agents
description: Manage CI agents — sync tracker issues, dispatch cop-fix tasks, review PRs, retry failures
allowed-tools: Bash(*), Read, Grep, Glob, AskUserQuestion
---

# CI Agents — Remote Agent Orchestration

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

## Interactive Prompts

When invoked without explicit arguments, use `AskUserQuestion` to gather:

1. **Department** — Ask: "Which department do you want to target? (e.g., Rails, Style, Performance, or leave blank for all)"
2. **Max concurrency** — For dispatch phase, ask: "How many concurrent agent runs? (default: 5)"

Pass the answers through as `--department` / `--max-active` to the CLI and workflows.
If the user provides these as skill arguments (e.g., `/ci-agents sync Rails`), skip the prompts.

## Phases

### Phase 1: Triage / Sync

Inspect the current dispatchable set and sync the tracker issues:

```bash
python3 scripts/ci-agents.py rank
gh workflow run cop-issue-sync.yml
```

To scope to a single department:

```bash
gh workflow run cop-issue-sync.yml -f department=Rails
```

This runs pre-diagnostic on every cop's FP/FN examples to classify them as
code bugs (agent can fix) vs config/context issues (agent can't). Only shows
cops with at least 1 real code bug.

For the lighter Codex lane, prefer cops with 3-10 total FP+FN and mostly code bugs:

```bash
python3 scripts/ci-agents.py rank --min-bugs 2 --max-total 10
```

For harder cops or overview by tier:

```bash
python3 scripts/ci-agents.py tiers --tier 1   # simple FP+FN count view
python3 scripts/investigate_cop.py Department/CopName --context  # deep dive
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

Scope to a department with concurrency limit:

```bash
gh workflow run cop-issue-dispatch.yml -f max_active=3 -f department=Rails
```

Dry run first if you want to inspect the selected queue:

```bash
gh workflow run cop-issue-dispatch.yml -f max_active=5 -f dry_run=true
gh workflow run cop-issue-dispatch.yml -f max_active=3 -f department=Rails -f dry_run=true
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
gh workflow run corpus-oracle.yml
```

Wait ~90 min, then check results:

```bash
python3 scripts/ci-agents.py tiers
```

## Arguments

- `/ci-agents` — start from Phase 1 (prompts for department interactively)
- `/ci-agents sync` — jump to Phase 1 (prompts for department)
- `/ci-agents sync Rails` — jump to Phase 1, scoped to Rails department
- `/ci-agents dispatch` — jump to Phase 2 (prompts for department and max concurrency)
- `/ci-agents dispatch Rails 3` — jump to Phase 2, Rails only, max 3 concurrent
- `/ci-agents retry` — jump to Phase 4 (retry failures)
- `/ci-agents status` — show current PR status and merge candidates
- `/ci-agents validate` — jump to Phase 5 (trigger corpus oracle)

## Important Notes

- Codex runs inside GHA — full Rust build environment with cache
- The task prompt contains all context the agent needs
- `workflow_dispatch` requires write access — safe on public repos
- Retries auto-close stale PRs and include prior attempt context
- `agent-cop-fix` now supports `backend=auto`; simple issue-backed fixes use `gpt-5.3-codex`, while harder fixes and repairs use `gpt-5.4`
- `claude` and `minimax` manual overrides still exist for experiments, but do not use them as the default recommendation
- Tracker issues should be created by the GitHub App (`6[bot]`), not manually
- Monitor ChatGPT usage at chatgpt.com/codex/settings/usage — dispatch in small batches
