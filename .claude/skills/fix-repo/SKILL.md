---
name: fix-repo
description: Improve a specific repo's corpus conformance by fixing its top diverging cops in parallel using worktree-isolated teammates.
allowed-tools: Bash(*), Read, Write, Edit, Grep, Glob, Task, TeamCreate, TaskCreate, TaskUpdate, TaskList, TaskGet, SendMessage
---

# Fix Repo — Repo-Targeted Conformance Improvement

This skill targets a specific corpus repo (e.g., rails, discourse, mastodon) and fixes
the cops that contribute the most FP/FN for that repo. Unlike `/fix-cops` (globally worst
cops) or `/fix-department` (all cops in a gem), this focuses on improving a specific repo's
match rate.

## Workflow

### Phase 0: Assess (you do this)

1. **If no repo was specified**, show the repo list and let the user pick:
   ```bash
   python3 scripts/investigate-repo.py --list $ARGUMENTS
   ```

2. **Once a repo is chosen**, run the repo investigation and **paste its full output
   verbatim to the user** (the table IS the primary output — do not summarize or skip it):
   ```bash
   python3 scripts/investigate-repo.py <repo-name> --exclude-cops-file fix-cops-done.txt $ARGUMENTS
   ```
   The `--exclude-cops-file` flag filters out cops already fixed since the last corpus run,
   so the output only shows cops that still need work.

3. Show the user the top diverging cops and confirm the target repo.

### Phase 1: Suggest & Confirm (you do this)

From the investigation output, **suggest up to 4 cops** to fix in this batch. Prioritize:

1. **FP-only cops** (FP>0, FN=0) — pure false alarms, usually straightforward
2. **High FP count** — more impact per fix
3. **High match rate globally** (>90%) — the cop mostly works, just has edge case FPs
4. Skip Layout/ alignment cops (HashAlignment, IndentationWidth, etc.) — complex multi-line state machines

**Important:** Check FP/FN counts globally, not just for the target repo. A cop with 6k FP
on rails but 110k FP globally is a bigger win. Prioritize cops where the target repo is a
significant contributor to the global divergence.

Present your suggestions to the user:
```
Based on the investigation, I'd suggest fixing these 4 cops:
1. Lint/ConstantResolution — 6,511 FP on rails (152k FP globally), likely <hypothesis>
2. Style/DocumentationMethod — 5,123 FP on rails (97k FP globally), likely <hypothesis>
3. ...

Want me to proceed with these, or would you like to swap any out?
```

**Wait for user confirmation before proceeding.** The user may want to pick different cops
or adjust the batch size.

Once confirmed, investigate each selected cop's FP/FN pattern in depth:
```bash
python3 scripts/investigate-cop.py Department/CopName --context --fp-only --limit 10
python3 scripts/investigate-cop.py Department/CopName --context --fn-only --limit 10
```

Summarize: cop name, repo-specific FP/FN, global FP/FN, root cause hypothesis.

### Phase 2: Dispatch (you do this)

1. Create a team:
   ```
   TeamCreate(team_name="fix-repo", description="Improve <repo-name> conformance")
   ```

2. Create tasks for each cop fix.

3. Spawn one teammate per cop using the Task tool. **Critical settings:**
   - `isolation: "worktree"` — each teammate gets its own git worktree
   - `subagent_type: "general-purpose"` — needs full edit/bash access
   - `team_name: "fix-repo"`
   - `mode: "bypassPermissions"` — teammates need to run cargo test etc.

4. Each teammate prompt MUST include:
   - The exact cop name (e.g., `Lint/ConstantResolution`)
   - The FP/FN counts (both repo-specific and global) and root cause hypothesis
   - The specific FP/FN examples with source context from your investigation
   - Whether to focus on FP fixes, FN fixes, or both
   - The teammate workflow (Phase 3 below) — paste the full instructions

### Phase 3: Teammate Workflow (paste this into each teammate's prompt)

```
You are fixing false positives/negatives in a single turbocop cop to improve corpus
conformance for a target repo. Follow the CLAUDE.md rules strictly.

**NEVER use git stash or git stash pop.** You are in an isolated git worktree — just commit directly.

## Steps

1. **Read the cop source** at `src/cop/<dept>/<cop_name>.rs`
   Read the vendor RuboCop spec at `vendor/rubocop*/spec/rubocop/cop/<dept>/<cop_name>_spec.rb`

2. **Understand the FP/FN pattern** from the examples provided in your prompt.
   If needed, read the actual source files from `vendor/corpus/<repo_id>/<path>` to see more context.

3. **Add test cases (TDD)**:
   - For FP fixes: add the false-positive pattern to `tests/fixtures/cops/<dept>/<cop_name>/no_offense.rb`
   - For FN fixes: add the missed detection to `tests/fixtures/cops/<dept>/<cop_name>/offense.rb`
   - Run `cargo test --release -p turbocop --lib -- <cop_name_snake>` to verify the test FAILS

4. **Fix the cop implementation** in `src/cop/<dept>/<cop_name>.rs`

5. **Verify**:
   - `cargo test --release -p turbocop --lib -- <cop_name_snake>` — all tests pass
   - `cargo fmt`
   - `cargo clippy --release -- -D warnings`

6. **Commit your fix**:
   ```bash
   git add src/cop/<dept>/<cop_name>.rs tests/fixtures/cops/<dept>/<cop_name>/
   # Add any other changed files
   git commit -m "Fix <Department/CopName> false positives/negatives

   <one-line description of what was wrong>

   Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
   ```

7. **Report back** via SendMessage with:
   - What the root cause was
   - What you changed
   - Whether tests pass
   - The commit SHA
   - If you could NOT fix it: explain why and whether it should be deferred
```

### Phase 4: Collect Results (you do this)

1. Wait for all teammates to report back.

2. For each completed fix:
   - Note the worktree branch name from the Task result
   - Cherry-pick or merge the commit into your working branch

3. Run full verification:
   ```bash
   cargo fmt
   cargo clippy --release -- -D warnings
   cargo test --release
   ```

4. Verify each fixed cop against the corpus:
   ```bash
   python3 scripts/check-cop.py Department/CopName --verbose --rerun
   ```

5. Record fixed cops in `fix-cops-done.txt`:
   ```bash
   echo "Department/CopName" >> fix-cops-done.txt
   ```

6. Re-run the repo investigation to show updated status:
   ```bash
   python3 scripts/investigate-repo.py <repo-name>
   ```
   Note: This still reads the original corpus data. Per-cop verification via check-cop.py
   gives the ground truth for fixed cops.

7. Report to the user:
   - Which cops were fixed (with FP/FN counts)
   - Estimated impact on the target repo's match rate
   - Which cops couldn't be fixed (and why)
   - Summary of changes ready for commit/PR

## Arguments

- `/fix-repo` — show repo list, let user pick
- `/fix-repo rails` — fix top diverging cops for the rails repo
- `/fix-repo discourse` — fix top diverging cops for discourse
- `/fix-repo rails --fp-only` — only fix FP-producing cops for rails
- `/fix-repo --input /path/to/corpus-results.json rails` — use local corpus file
