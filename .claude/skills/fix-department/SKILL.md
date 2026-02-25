---
name: fix-department
description: Get all cops in a gem to 100% corpus conformance. Assesses, triages, and fixes all diverging cops in a target gem using worktree-isolated teammates.
allowed-tools: Bash(*), Read, Write, Edit, Grep, Glob, Task, TeamCreate, TaskCreate, TaskUpdate, TaskList, TaskGet, SendMessage
---

# Fix Department — Gem-Level 100% Conformance

This skill targets a specific gem (e.g., rubocop-performance) and fixes ALL diverging
cops until it reaches 100% corpus conformance (0 FP + 0 FN). Unlike `/fix-cops` which
fixes the globally worst cops, this focuses on *completing* one gem at a time to unlock
incremental adoption.

## Workflow

### Phase 0: Assess (you do this)

1. **Always start with the scoreboard.** Run the script and **paste its full output verbatim
   to the user** (the table IS the primary output — do not summarize or skip it):
   ```bash
   python3 .claude/skills/fix-department/scripts/gem_progress.py $ARGUMENTS
   ```
   This auto-detects `fix-cops-done.txt` and shows already-fixed cops as "Fixed (pending
   corpus confirmation)" so the scoreboard reflects reality between corpus runs.
   The script also prints a recommendation at the bottom.

2. **If no gem was specified**, after showing the table, let the user pick a gem.

3. **Once a gem is chosen** (by user or from args), run the deep-dive:
   ```bash
   python3 .claude/skills/fix-department/scripts/gem_progress.py --gem <gem-name> $ARGUMENTS
   ```

4. Show the user the gem status and confirm the target.

### Phase 1: Plan Batch (you do this)

From the deep-dive output, select **up to 4 cops** for this batch. Priority order:

1. **FP-only cops** (FP>0, FN=0) — pure false alarms, usually straightforward to fix
2. **Both FP+FN cops** with highest FP — fix the FP side first
3. **FN-only cops** (FP=0, FN>0) — missing detections, lower priority but needed for 100%

Skip Layout/ alignment cops unless they're the only ones remaining (complex multi-line state machines).

For each selected cop, investigate the FP/FN pattern:
```bash
python3 scripts/investigate-cop.py Department/CopName --context --fp-only --limit 10
python3 scripts/investigate-cop.py Department/CopName --context --fn-only --limit 10
```

Summarize: cop name, FP/FN counts, root cause hypothesis.

### Phase 2: Dispatch (you do this)

1. Create a team:
   ```
   TeamCreate(team_name="fix-department", description="Bring <gem-name> to 100% conformance")
   ```

2. Create tasks for each cop fix.

3. Spawn one teammate per cop using the Task tool. **Critical settings:**
   - `isolation: "worktree"` — each teammate gets its own git worktree
   - `subagent_type: "general-purpose"` — needs full edit/bash access
   - `team_name: "fix-department"`
   - `mode: "bypassPermissions"` — teammates need to run cargo test etc.

4. Each teammate prompt MUST include:
   - The exact cop name (e.g., `Performance/AncestorsInclude`)
   - The FP/FN counts and root cause hypothesis from your investigation
   - The specific examples with source context you found in Phase 1
   - Whether to focus on FP fixes, FN fixes, or both
   - The teammate workflow (Phase 3 below) — paste the full instructions

### Phase 3: Teammate Workflow (paste this into each teammate's prompt)

```
You are fixing false positives/negatives in a single turbocop cop to bring its gem
to 100% corpus conformance. Follow the CLAUDE.md rules strictly.

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

### Phase 4: Collect + Loop (you do this)

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

5. Record fixed cops in `fix-cops-done.txt` (shared with `/fix-cops` — both track
   cops fixed since the last corpus oracle run):
   ```bash
   echo "Department/CopName" >> fix-cops-done.txt
   ```

6. Re-run the gem deep-dive to see updated progress:
   ```bash
   python3 .claude/skills/fix-department/scripts/gem_progress.py --gem <gem-name>
   ```
   Note: This still reads the original corpus data. Per-cop verification via check-cop.py
   gives the ground truth for fixed cops.

7. If diverging cops remain, go back to Phase 1 for the next batch.

7. For cops that teammates couldn't fix, decide whether to:
   - Retry with more context in the next batch
   - Defer with a documented reason

### Phase 5: Declare Done (you do this)

When all cops in the gem are at 0 FP + 0 FN (or explicitly deferred):

1. Run full verification:
   ```bash
   cargo fmt
   cargo clippy --release -- -D warnings
   cargo test --release
   ```

2. Report to the user:
   - Gem name and total cops
   - How many cops were fixed (with FP/FN reduction)
   - How many cops were already perfect
   - Any deferred cops with reasons
   - Summary: "rubocop-performance: 100% corpus conformance (N cops, M fixed in this session)"

3. Remind the user to trigger a fresh corpus oracle run to confirm the result.

## Arguments

- `/fix-department` — **show the scoreboard, recommend a gem, and ask** which to target
- `/fix-department rubocop-performance` — target rubocop-performance directly
- `/fix-department rubocop-rspec` — target rubocop-rspec directly
- `/fix-department --input /path/to/corpus-results.json` — use local corpus file

## How to Choose the Next Gem

The scoreboard (`gem_progress.py --summary`) shows per-gem stats. Prioritize by:

1. **Zero untested cops** — only gems where every cop triggered on the 500-repo corpus
   can claim true 100% conformance. Gems with untested cops get an asterisk. The "Untest"
   column in the scoreboard shows this.
2. **Fewest diverging cops** — less work to complete the gem. The "Dvrg" column shows this.
3. **Adoption value** — rubocop-performance is the most commonly added plugin, so completing
   it has more impact than rubocop-factory_bot, even if factory_bot is smaller.
4. **FP-free first** — a gem with 0 FP but some FN is already safe to adopt (no false alarms).
   Fix FNs later for completeness.
