---
name: review-agent-prs
disable-model-invocation: true
---

# review-agent-prs

Review PRs created by the agent cop fix workflow. Approve good ones so workflow auto-merge can land them, fix minor issues, or close bad ones.

## Trigger

User runs `/review-agent-prs` (optionally with filters like `--cop Style/*`).

## Workflow

### 1. List open agent PRs with passing CI

```bash
gh pr list --repo 6/nitrocop --label type:cop-fix --state open \
  --json number,title,headRefName,statusCheckRollup,labels,createdAt \
  --jq '[.[] | select(
    (.statusCheckRollup | length > 0) and
    (.statusCheckRollup | all(.status == "COMPLETED")) and
    (.statusCheckRollup | all(.conclusion == "SUCCESS"))
  )] | .[] | "\(.number)\t\(.title)\t\(.labels | map(.name) | join(","))"'
```

This filters to only PRs where all CI checks have passed, skipping drafts with no checks, PRs with pending checks, and PRs with failures — no need to run `gh pr checks` per PR.

Also list PRs with `validation-failed` label separately so they can be closed:

```bash
gh pr list --repo 6/nitrocop --label type:cop-fix,validation-failed --state open \
  --json number,title --jq '.[] | "\(.number)\t\(.title)"'
```

### 2. Review each PR

For each passing PR, fetch the diff with `gh pr diff <number>` and review:

**Code correctness:**
- Does the logic match the cop's intended behavior?
- Are edge cases handled?
- Could the fix introduce false positives or false negatives?

**Code quality:**
- Missing explicit parentheses for operator precedence clarity
- Overly verbose or unclear comments
- Unnecessary complexity that could be simplified

**Tests:**
- Are offense and no_offense fixtures adequate?
- Does corrected.rb exist if autocorrect was added?

### 3. Take action on each PR

For each PR, do exactly one of:

**Approve** — code is correct and clean:
```bash
gh pr review <number> --approve --body "Reviewed: logic correct, tests adequate."
```
The workflow enables squash auto-merge, so approval is enough.

**Fix then approve** — code is correct but has quality nits:
1. Check out the PR branch
2. Fix the issues (fmt, readability, simplification)
3. Commit and push
4. Approve the PR:
```bash
gh pr review <number> --approve --body "Reviewed: fixed [description of nits]. Logic correct."
```

**Close** — code is wrong, too complex, or not worth fixing:
```bash
gh pr close <number> --comment "Closing: [reason]. [Specific issues found.]" --delete-branch
```

### 4. Present summary

Show a table of actions taken:
```
| PR   | Cop                         | Action          |
|------|-----------------------------|-----------------|
| #102 | Style/VariableInterpolation | Approved        |
| #105 | Lint/EmptyBlock             | Fixed + approved|
| #106 | Layout/SpaceBeforeComment   | Closed (reason) |
```

## Rules

- Only review PRs with the `type:cop-fix` label
- Agent PRs start as drafts, then the workflow marks them ready after the agent finishes. A draft PR with all CI checks passing is ready for review. Only skip draft PRs that have no checks or pending/failing checks — those are still being worked on.
- Skip PRs with failing or pending CI checks — only review PRs where all checks have passed
- PRs with `validation-failed` label: close with comment explaining why
- PRs with merge conflicts: close with comment, agent can retry on fresh main
- Do not merge PRs — only approve, fix+approve, or close
- When fixing, commit with a clear message explaining what was changed
- When closing, always leave a comment with the specific reason so the dispatch system can learn
- If the diff contains changes to Python files (`.py`), treat this as suspicious — agent cop-fix should only touch Rust code and test fixtures. Flag it to the user before approving.
- If the diff contains only `///` doc comment changes in `.rs` files and no code logic or fixture changes, close the PR. Doc-only PRs indicate the agent couldn't find a code fix — they add git noise without closing any FP/FN gap. The agent should have exited without committing.
