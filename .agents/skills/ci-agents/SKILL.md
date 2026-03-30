---
name: ci-agents
description: Manage CI agents - sync tracker issues, dispatch cop-fix tasks, review PRs, and retry failures.
allowed-tools: Bash(*), Read, Grep, Glob
---

# CI Agents

Dispatch cop-fix tasks to AI agents running in GitHub Actions to fix corpus
conformance gaps in parallel. The current system uses one GitHub issue per
diverging cop as a durable backlog item. Dispatchers fill a bounded queue from
those issues, then `agent-cop-fix` opens one PR per cop and `agent-pr-repair`
reacts to failed deterministic CI.

See `docs/agent-dispatch.md` for full setup instructions and architecture.

## Prerequisites

Before dispatching, verify the pipeline is set up:

```bash
# Verify the workflows exist
ls .github/workflows/agent-cop-fix.yml \
   .github/workflows/agent-pr-repair.yml \
   .github/workflows/cop-issue-sync.yml
```

The user needs `CODEX_AUTH_JSON` configured in GitHub repo secrets.
See `docs/agent-dispatch.md` for setup instructions.

## Interactive Prompts

When invoked without explicit arguments, ask the user:

1. **Department** — "Which department do you want to target? (for example
   Rails, Style, Performance, or leave blank for all)"

If the user provides these as skill arguments (for example `/ci-agents sync
Rails`), skip the prompts.

## Phases

### Phase 1: Triage / Sync

Inspect the current dispatchable set and sync the tracker issues:

```bash
python3 scripts/dispatch_cops.py rank
gh workflow run cop-issue-sync.yml
```

To scope to a single department:

```bash
gh workflow run cop-issue-sync.yml -f department=Rails
```

This runs pre-diagnostic on every cop's FP/FN examples to classify them as code
bugs (agent can fix) vs config/context issues (agent cannot). Only shows cops
with at least 1 real code bug.

For the lighter Codex lane, prefer cops with 3-10 total FP+FN and mostly code
bugs:

```bash
python3 scripts/dispatch_cops.py rank --min-bugs 2 --max-total 10
```

For harder cops or overview by tier:

```bash
python3 scripts/dispatch_cops.py tiers --tier 1
python3 scripts/investigate_cop.py Department/CopName --context
```

Skip cops with 0 code bugs. They are all config issues and the workflow will
auto-skip them anyway via the pre-diagnostic gate.

The sync workflow creates or updates one `[cop] Department/CopName` issue per
diverging cop, reopens old issues when a cop regresses again, and applies a
durable difficulty label (`difficulty:simple|medium|complex`). The actual
backend is chosen later by `agent-cop-fix` when the issue is dispatched.

### Phase 2: Dispatch

Find dispatchable cops. Skip `difficulty:config-only`; those need config
resolution work, not cop logic fixes.

When a department is specified, add `--search "Department/ in:title"` to filter
server-side instead of relying on `--limit` to fetch enough results. There are
300+ cop issues.

```bash
# All departments
gh issue list --state open --label "type:cop-issue" --label "state:backlog" --limit 200 \
  --json number,title,labels \
  -q '.[] | select(.labels | map(.name) | all(. != "difficulty:config-only")) | "#\(.number) \(.title)"'

# Scoped to a department (for example Lint)
gh issue list --state open --label "type:cop-issue" --label "state:backlog" \
  --search "Lint/ in:title" --limit 100 \
  --json number,title,labels \
  -q '.[] | select(.labels | map(.name) | all(. != "difficulty:config-only")) | "#\(.number) \(.title)"'
```

For batch dispatch (ranked by code-bug ratio, deterministic):

```bash
# Dispatch top 10 Layout cops via codex
gh workflow run batch-dispatch.yml -f department=Layout -f count=10 -f backend=codex

# Dispatch top 5 Style cops via claude
gh workflow run batch-dispatch.yml -f department=Style -f count=5 -f backend=claude

# All departments, codex, only cops with ≤20 total FP+FN
gh workflow run batch-dispatch.yml -f count=10 -f backend=codex -f max_total=20

# Delayed dispatch — wait 60 minutes before dispatching (max 330m)
gh workflow run batch-dispatch.yml -f department=Style -f count=5 -f backend=claude -f delay=60
```

Dispatch cops directly via `agent-cop-fix`:

```bash
gh workflow run agent-cop-fix.yml -f cop="Style/ClassVars"
gh workflow run agent-cop-fix.yml -f cop="Style/ClassVars" -f backend=codex
gh workflow run agent-cop-fix.yml -f cop="Style/ClassVars" -f backend=claude
```

To dispatch multiple cops, run one command per cop. To link to a tracker issue:

```bash
gh workflow run agent-cop-fix.yml -f cop="Style/ClassVars" -f issue_number=123
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

After merging a batch (roughly 20-50 PRs), run the full corpus oracle:

```bash
gh workflow run corpus-oracle.yml
```

Wait around 90 minutes, then check results:

```bash
python3 scripts/dispatch_cops.py tiers
```

## Arguments

- `/ci-agents` - start from Phase 1 and ask for department if needed
- `/ci-agents sync` - jump to Phase 1 and ask for department if needed
- `/ci-agents sync Rails` - jump to Phase 1 scoped to Rails
- `/ci-agents dispatch` - jump to Phase 2 and ask for department if needed
- `/ci-agents dispatch Rails` - jump to Phase 2 scoped to Rails
- `/ci-agents dispatch Style 5 claude 60` - dispatch with delay (last arg = minutes to wait)
- `/ci-agents retry` - jump to Phase 4
- `/ci-agents status` - show current PR status and merge candidates
- `/ci-agents validate` - jump to Phase 5

## Important Notes

- Codex runs inside GitHub Actions with the full Rust build environment cached
- The task prompt contains the context the remote agent needs
- `workflow_dispatch` requires write access, which is safe on public repos
- Retries auto-close stale PRs and include prior attempt context
- `agent-cop-fix` supports `backend=auto`; codex-backed fixes use `gpt-5.4`
  with higher effort on harder cases
- `claude` and `minimax` manual overrides still exist for experiments, but they
  are not the default recommendation
- Tracker issues should be created by the GitHub App (`6[bot]`), not manually
- Monitor ChatGPT usage at `chatgpt.com/codex/settings/usage` and dispatch in
  small batches
