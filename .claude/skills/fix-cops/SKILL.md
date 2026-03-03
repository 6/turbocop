---
name: fix-cops
description: Auto-fix batch of cops after a corpus oracle run. Triages, investigates, and fixes top FP cops in parallel using worktree-isolated teammates.
allowed-tools: Bash(*), Read, Write, Edit, Grep, Glob, Task, TeamCreate, TaskCreate, TaskUpdate, TaskList, TaskGet, SendMessage
---

# Fix Cops — Post-Corpus-Oracle Batch Fix

This skill runs after a corpus oracle CI run. It triages the results, picks the
highest-impact cops to fix, and dispatches parallel teammates (each in an
isolated git worktree) to investigate and fix them.

If you edit code yourself (without dispatching teammates), do that work in a dedicated
git worktree by default. Only skip this when the user explicitly asks to use the current tree.

## Workflow

### Phase 1: Triage (you do this)

1. Download corpus results and run triage (automatically excludes cops fixed since the oracle run):
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

4. **Run the delta reducer** on up to 3 FP examples per cop to get minimal reproductions:
   ```bash
   python3 scripts/reduce-mismatch.py Department/CopName repo_id filepath:line
   ```
   The repo_id and filepath:line come directly from investigate-cop.py output. Pick examples
   from different repos when possible to catch distinct root causes. The reduced files
   (typically 5–20 lines) go to `/tmp/nitrocop-reduce/` — read them and include them in the
   teammate prompt.

5. Summarize your picks: cop name, FP count, minimal repro(s), and root cause hypothesis.

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
   - **The minimal repro(s) from the delta reducer** — paste the reduced Ruby source directly
   - A reminder that parallel-agent activity often leaves unrelated local modifications; those files are off-limits
   - The teammate workflow (Phase 3 below) — paste the full instructions

### Phase 3: Teammate Workflow (paste this into each teammate's prompt)

```
You are fixing false positives in a single nitrocop cop. Follow the CLAUDE.md rules strictly.

**NEVER use git stash or git stash pop.** You are in an isolated git worktree — just commit directly.
Parallel-agent activity is common. If you see unrelated modified files, do not edit/revert them.

## Steps

1. **Read the cop source** at `src/cop/<dept>/<cop_name>.rs`
   Read the vendor RuboCop spec at `vendor/rubocop*/spec/rubocop/cop/<dept>/<cop_name>_spec.rb`
   **Check for existing investigation comments** (marked with "Known false positives" or
   "reverted") — these document previously attempted fixes that regressed on corpus
   validation. Do NOT repeat the same approach. Either find a different root cause or
   extend the prior approach to avoid its documented failure mode.

2. **Understand the FP pattern** from the examples provided in your prompt.
   If needed, read the actual source files from `vendor/corpus/<repo_id>/<path>` to see more context.
   **DO NOT run nitrocop or rubocop directly on corpus repos** — they require special
   env vars (BUNDLE_GEMFILE, BUNDLE_PATH, GIT_CEILING_DIRECTORIES) that only `check-cop.py`
   sets up correctly. If you need to verify corpus behavior, use `check-cop.py --rerun`.

3. **Add test cases (TDD)**:
   - Add the FP pattern to `tests/fixtures/cops/<dept>/<cop_name>/no_offense.rb`
   - Run `cargo test --release -p nitrocop --lib -- <cop_name_snake>` to verify the test FAILS (proving the FP exists)

4. **Fix the cop implementation** in `src/cop/<dept>/<cop_name>.rs`

5. **Verify**:
   - `cargo test --release -p nitrocop --lib -- <cop_name_snake>` — all tests pass
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
   Stage only files for this cop fix. Do not include unrelated modified files.

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

4. Verify each fixed cop against the corpus:
   ```bash
   python3 scripts/check-cop.py Department/CopName --verbose --rerun
   ```
   Run these in parallel (background). **Corpus validation is the acceptance gate** —
   unit tests passing is necessary but NOT sufficient.

5. **Handle regressions**: if a fix increases FP count (even if unit tests pass), revert
   the code change but **add a detailed investigation comment** to the cop source file
   documenting:
   - What approach was tried (with the reverted commit SHA)
   - Exact code path changed (function/condition)
   - Acceptance-gate numbers before and after (`expected`, `actual`, `excess`, `missing`)
   - Why it regressed (root cause of the new FPs)
   - What a correct fix would need to do differently
   This prevents future attempts from repeating the same failed approach. Use the format:
   ```rust
   /// ## Known false positives (N FP in corpus as of YYYY-MM-DD)
   ///
   /// An attempt was made to ... (commit XXXXXXXX, reverted). The approach: ...
   /// Code path changed: <file::function and condition changed>.
   /// Acceptance gate before: expected=?, actual=?, excess=?, missing=?
   /// Acceptance gate after: expected=?, actual=?, excess=?, missing=?
   /// This fixed the target FPs but introduced N NEW false positives (X→Y FP).
   /// Root cause of regression: ...
   /// A correct fix needs to: ...
   ```

6. **Document ALL investigation outcomes** as `///` comments on the cop's struct in its
   source file — not just regressions and reverts, but also cops investigated and found
   to need no fix (e.g., FPs caused by file-drop noise, config artifacts, etc.). This
   prevents future investigators from repeating the same analysis. Use:
   ```rust
   /// ## Corpus investigation (YYYY-MM-DD)
   ///
   /// Corpus oracle run #N reported FP=X, FN=Y. Investigation found ...
   /// <conclusion and whether a fix is needed>
   ```

7. Report to the user:
   - Which cops were fixed (with FP counts)
   - Which cops regressed and were reverted (with investigation comments added)
   - Summary of changes ready for commit/PR

### Phase 5: Integrate Back to Main (Default)

Do not leave retained progress only in a worktree/collector branch.

1. Ensure all retained progress is committed:
   - Accepted cop fixes: one commit per cop (preferred).
   - Useful investigation artifacts retained in repo (for example, reverted-attempt notes): separate commit.

2. Integrate those commit(s) into `main` immediately (unless the user explicitly says not to):
   ```bash
   git -C /path/to/main checkout main
   git -C /path/to/main cherry-pick <sha1> [<sha2> ...]
   ```
   If a merge is preferred, use a normal non-interactive merge.

3. Verify integration on `main`:
   ```bash
   git -C /path/to/main log --oneline -n 10
   git -C /path/to/main status --short --branch
   ```

4. Report exactly what was integrated (commit SHA(s) and short subjects).

5. If there is truly no repo-retained progress, explicitly report that no commit was made.

## Arguments

Pass arguments through to the triage script:
- `/fix-cops` — default: top FP-only cops
- `/fix-cops --department Style` — only Style cops
- `/fix-cops --limit 10` — consider more candidates
- `/fix-cops --input /path/to/corpus-results.json` — use local file
