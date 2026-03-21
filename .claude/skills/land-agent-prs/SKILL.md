# land-agent-prs

Review and merge PRs created by the agent cop fix workflow.

## Trigger

User runs `/land-agent-prs` (optionally with filters like `--dry-run`, `--cop Style/*`).

## Workflow

### 1. List open agent PRs

```bash
gh pr list --repo 6/nitrocop --label agent-fix --state open \
  --json number,title,headRefName,statusCheckRollup,labels,createdAt \
  --jq '.[] | "\(.number)\t\(.title)\t\(.labels | map(.name) | join(","))"'
```

### 2. Categorize each PR

For each open PR, check:
- **Validation status**: look for `validation-failed` label or validation comment
- **CI status**: `gh pr checks <number>` (may be empty if checks weren't dispatched)
- **Mergeable**: no conflicts with main

Categorize into:
- **Ready**: validation passed, no `validation-failed` label, tests OK
- **Failed**: has `validation-failed` label or failing checks
- **Fixable**: CI passes but code has quality nits worth fixing before merge
- **Pending**: no validation results yet

### 2b. Review code quality and fix nits

For each PR that passes CI, review the diff for quality issues common in agent-generated code:
- Missing explicit parentheses for operator precedence clarity
- Overly verbose or unclear comments
- Unnecessary complexity that could be simplified

If nits are found, check out the PR branch, fix them, commit, and push before merging.

### 3. Present summary table

Show a table like:
```
| PR  | Cop                        | Status  | Action     |
|-----|----------------------------|---------|------------|
| #102| Style/VariableInterpolation| Ready   | Will merge |
| #105| Lint/EmptyBlock            | Failed  | Skip       |
| #108| Style/SymbolProc           | Pending | Skip       |
```

### 4. Land ready PRs

Use `scripts/land-branch-commits.sh` to cherry-pick each commit from the PR branch onto main, preserving individual commits (agent fix + any cleanup commits stay separate):

```bash
bash scripts/land-branch-commits.sh <branch-name>
git push origin main
```

Then close the PR and delete the remote branch:
```bash
gh pr close <number> --delete-branch
```

If `--dry-run` was specified, show what would be landed but don't land.

### 5. Report

Summarize: N landed, N failed (skipped), N pending (skipped).

## Rules

- Only land PRs with the `agent-fix` label
- Never land PRs with the `validation-failed` label
- Preserve individual commits (don't squash) — keeps agent fix vs reviewer cleanup visible in history
- Delete the remote branch after landing
- If unsure about a PR, skip it and note why
- Don't land PRs that have merge conflicts — note them for manual resolution
