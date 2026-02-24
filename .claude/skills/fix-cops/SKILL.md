---
name: fix-cops
description: Auto-fix batch of cops after a corpus oracle run. Triages, investigates, and fixes top FP cops in parallel using worktree-isolated teammates.
allowed-tools: Bash(*), Read, Write, Edit, Grep, Glob, Task, TeamCreate, TaskCreate, TaskUpdate, TaskList, TaskGet, SendMessage
---

# Fix Cops — Post-Corpus-Oracle Batch Fix

This skill runs after a corpus oracle CI run. It triages the results, picks the
highest-impact cops to fix, and dispatches parallel teammates (each in an
isolated git worktree) to investigate and fix them.

## Workflow

### Phase 1: Triage (you do this)

1. Download corpus results and run triage:
   ```bash
   python3 .claude/skills/triage/scripts/triage.py --fp-only --limit 20 $ARGUMENTS
   ```

2. From the triage output, select **up to 4 cops** to fix in this batch. Prioritize:
   - **FP-only cops** (FN=0) — these are pure regressions, usually straightforward
   - **High FP count** — more impact per fix
   - **High match rate** (>90%) — the cop mostly works, just has edge case FPs
   - Skip Layout/ alignment cops (HashAlignment, IndentationWidth, etc.) — these are complex multi-line state machines, not good for batch fixing

3. For each selected cop, run investigate-cop.py to understand the FP pattern:
   ```bash
   python3 scripts/investigate-cop.py Department/CopName --context --fp-only --limit 10
   ```

4. Summarize your picks: cop name, FP count, and a one-line hypothesis of the root cause.

### Phase 2: Dispatch (you do this)

1. Create a team:
   ```
   TeamCreate(team_name="fix-cops", description="Batch cop fixes from corpus oracle")
   ```

2. Create tasks for each cop fix.

3. Spawn one teammate per cop using the Task tool. **Critical settings:**
   - `isolation: "worktree"` — each teammate gets its own git worktree (NO git stash/pop!)
   - `subagent_type: "general-purpose"` — needs full edit/bash access
   - `team_name: "fix-cops"`
   - `mode: "bypassPermissions"` — teammates need to run cargo test etc.

4. Each teammate prompt MUST include:
   - The exact cop name (e.g., `Style/PercentQLiterals`)
   - The FP count and root cause hypothesis from your investigation
   - The specific FP examples with source context you found in Phase 1
   - The teammate workflow (Phase 3 below) — paste the full instructions

### Phase 3: Teammate Workflow (paste this into each teammate's prompt)

```
You are fixing false positives in a single turbocop cop. Follow the CLAUDE.md rules strictly.

**NEVER use git stash or git stash pop.** You are in an isolated git worktree — just commit directly.

## Steps

1. **Read the cop source** at `src/cop/<dept>/<cop_name>.rs`
   Read the vendor RuboCop spec at `vendor/rubocop*/spec/rubocop/cop/<dept>/<cop_name>_spec.rb`

2. **Understand the FP pattern** from the examples provided in your prompt.
   If needed, read the actual source files from `vendor/corpus/<repo_id>/<path>` to see more context.

3. **Add test cases (TDD)**:
   - Add the FP pattern to `tests/fixtures/cops/<dept>/<cop_name>/no_offense.rb`
   - Run `cargo test --release -p turbocop --lib -- <cop_name_snake>` to verify the test FAILS (proving the FP exists)

4. **Fix the cop implementation** in `src/cop/<dept>/<cop_name>.rs`

5. **Verify**:
   - `cargo test --release -p turbocop --lib -- <cop_name_snake>` — all tests pass
   - `cargo fmt`
   - `cargo clippy --release -- -D warnings`

6. **Commit your fix**:
   ```bash
   git add src/cop/<dept>/<cop_name>.rs tests/fixtures/cops/<dept>/<cop_name>/no_offense.rb
   # Add any other changed fixture files
   git commit -m "Fix <Department/CopName> false positives

   <one-line description of what was wrong>

   Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
   ```

7. **Report back** via SendMessage with:
   - What the FP root cause was
   - What you changed
   - Whether tests pass
   - The commit SHA
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

4. Report to the user:
   - Which cops were fixed (with FP counts)
   - Which cops couldn't be fixed (and why)
   - Summary of changes ready for commit/PR

## Arguments

Pass arguments through to the triage script:
- `/fix-cops` — default: top FP-only cops
- `/fix-cops --department Style` — only Style cops
- `/fix-cops --limit 10` — consider more candidates
- `/fix-cops --input /path/to/corpus-results.json` — use local file
