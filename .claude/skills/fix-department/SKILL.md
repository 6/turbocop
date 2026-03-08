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

If you edit code yourself (without dispatching teammates), do that work in a dedicated
git worktree by default. Only skip this when the user explicitly asks to use the current tree.

## Workflow

### Phase 0: Assess (you do this)

1. **Always start with the scoreboard.** Run the script and **paste its full output verbatim
   to the user** (the table IS the primary output — do not summarize or skip it):
   ```bash
   python3 .claude/skills/fix-department/scripts/gem_progress.py --summary
   ```
   The script automatically detects cops fixed since the last corpus oracle run by
   scanning git commit messages, and shows them as "Fixed (pending corpus confirmation)"
   so the scoreboard reflects reality between corpus runs.
   The script also prints a recommendation at the bottom.

2. **If no gem was specified**, after showing the table, let the user pick a gem.

3. **Once a gem is chosen** (by user or from args), run the deep-dive:
   ```bash
   python3 .claude/skills/fix-department/scripts/gem_progress.py --gem <gem-name>
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

**Run the delta reducer** on up to 3 examples per cop (mix of FP and FN) to get minimal reproductions:
```bash
python3 scripts/reduce-mismatch.py Department/CopName repo_id filepath:line            # FP
python3 scripts/reduce-mismatch.py Department/CopName repo_id filepath:line --type fn   # FN
```
Pick examples from different repos when possible. The reduced files (typically 5–20 lines) go
to `/tmp/nitrocop-reduce/` — read them and include them in the teammate prompt.

Summarize: cop name, FP/FN counts, minimal repro(s), root cause hypothesis.

### Phase 2: Dispatch (you do this)

1. Create a team:
   ```
   TeamCreate(team_name="fix-department", description="Bring <gem-name> to 100% conformance")
   ```

2. Create tasks for each cop fix.

3. Spawn one teammate per cop using the Agent tool. **Critical settings:**
   - `isolation: "worktree"` — each teammate gets its own git worktree
   - `subagent_type: "general-purpose"` — needs full edit/bash access
   - `team_name: "fix-department"`
   - `mode: "bypassPermissions"` — teammates need to run cargo test etc.

   **Worktree caveat:** `isolation: "worktree"` may silently fail, leaving the teammate
   writing directly to the main tree. The teammate workflow below includes a self-check.
   During Phase 4, also verify with `git status --short` that no leaked changes landed on main.

4. Each teammate prompt MUST include:
   - The exact cop name (e.g., `Performance/AncestorsInclude`)
   - The FP/FN counts and root cause hypothesis from your investigation
   - **The minimal repro(s) from the delta reducer** — paste the reduced Ruby source directly
   - Whether to focus on FP fixes, FN fixes, or both
   - A reminder that parallel-agent activity often leaves unrelated local modifications; those files are off-limits
   - The teammate workflow (Phase 3 below) — paste the full instructions

### Phase 3: Teammate Workflow (paste this into each teammate's prompt)

```
You are fixing false positives/negatives in a single nitrocop cop to bring its gem
to 100% corpus conformance. Follow the CLAUDE.md rules strictly.

**NEVER use git stash or git stash pop.** You should be in an isolated git worktree — just commit directly.
Parallel-agent activity is common. If you see unrelated modified files, do not edit/revert them.

## Step 0: Verify your working directory

Run `git rev-parse --show-toplevel` and check that it contains `.claude/worktrees/` in the path.
If you are in the main repo (no worktree), note this in your report — your commits will land
directly on main.

## Steps

1. **Read the cop source** at `src/cop/<dept>/<cop_name>.rs`
   Read the vendor RuboCop spec at `vendor/rubocop*/spec/rubocop/cop/<dept>/<cop_name>_spec.rb`
   **Check for existing investigation comments** (marked with "Known false positives" or
   "reverted") — these document previously attempted fixes that regressed on corpus
   validation. Do NOT repeat the same approach. Either find a different root cause or
   extend the prior approach to avoid its documented failure mode.

2. **Understand the FP/FN pattern** from the examples provided in your prompt.
   If needed, read the actual source files from `vendor/corpus/<repo_id>/<path>` to see more context.
   **DO NOT run nitrocop or rubocop directly on corpus repos** — they require special
   env vars (BUNDLE_GEMFILE, BUNDLE_PATH, GIT_CEILING_DIRECTORIES) that only `check-cop.py`
   sets up correctly. If you need to verify corpus behavior, use `check-cop.py --rerun`
   for the cop you modified; use `check-cop.py --verbose` for untouched cops when the
   latest corpus oracle run is current.

3. **Add test cases (TDD)**:
   - For FP fixes: add the false-positive pattern to `tests/fixtures/cops/<dept>/<cop_name>/no_offense.rb`
   - For FN fixes: add the missed detection to `tests/fixtures/cops/<dept>/<cop_name>/offense.rb`
   - Run `cargo test --lib -- <cop_name_snake>` to verify the test FAILS
   - Use debug mode (no `--release`) for fast TDD iteration (~8s first run, <1s incremental)

4. **Fix the cop implementation** in `src/cop/<dept>/<cop_name>.rs`
   - Iterate with `cargo test --lib -- <cop_name_snake>` (debug mode) until tests pass

5. **Add investigation comment** to the cop's struct as a `///` doc comment. This is
   **required** even if you fully fixed the cop — document what the original FP/FN were,
   what you fixed, and any remaining gaps with root causes. Example:
   ```rust
   /// ## Corpus investigation (YYYY-MM-DD)
   ///
   /// Corpus oracle reported FP=X, FN=Y.
   ///
   /// FP=X: Fixed by <description>. Commit <sha>.
   /// FN=Y: <Fixed/remaining/expected behavior — with root cause>.
   pub struct CopName;
   ```

6. **Verify (pre-commit, release mode)**:
   - `cargo test --release -p nitrocop --lib -- <cop_name_snake>` — all tests pass
   - `cargo fmt`
   - `cargo clippy --release -- -D warnings`

7. **Commit your fix**:
   ```bash
   git add src/cop/<dept>/<cop_name>.rs tests/fixtures/cops/<dept>/<cop_name>/
   # Add any other changed files
   git commit -m "Fix <Department/CopName> false positives/negatives

   <one-line description of what was wrong>

   Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
   ```
   Stage only files for this cop fix. Do not include unrelated modified files.

8. **Report back** via SendMessage with:
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
   **Corpus validation is the acceptance gate** — unit tests passing is necessary but
   NOT sufficient. Use `--rerun` only for cops changed in this batch; for untouched
   cops, use artifact mode (`--verbose`).

5. **Handle regressions**: if a fix increases FP count (even if unit tests pass), revert
   the code change but **add a detailed investigation comment** to the cop source file
   documenting what was tried, exactly where code changed, acceptance-gate numbers
   before/after, why it regressed, and what a correct fix would need. Use:
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

6. Re-run the gem deep-dive to see updated progress:
   ```bash
   python3 .claude/skills/fix-department/scripts/gem_progress.py --gem <gem-name>
   ```
   Note: This still reads the original corpus data. Per-cop verification via check-cop.py
   gives the ground truth for fixed cops.

6. If diverging cops remain, go back to Phase 1 for the next batch.

7. For cops that teammates couldn't fix, decide whether to:
   - Retry with more context in the next batch
   - Defer with a documented reason

8. **Document ALL investigation outcomes** as `///` comments on the cop's struct in its
   source file. **This is mandatory for EVERY cop in the batch** — including:
   - Cops that were **fixed** but have remaining FP/FN (document what was fixed and why
     the remaining gaps exist)
   - Cops that were **investigated but need no code fix** (e.g., FPs caused by encoding
     differences, file-drop noise, config artifacts)
   - Cops that were **deferred** (document why and what a fix would need)
   - Cops with FN from `standard:disable` handling or other config edge cases (these are still bugs to fix, not acceptable differences)

   This prevents future investigators from repeating the same analysis. **Do not
   consider a cop "done" until it has an investigation comment, even if the fix itself
   is already committed.** Use:
   ```rust
   /// ## Corpus investigation (YYYY-MM-DD)
   ///
   /// Corpus oracle reported FP=X, FN=Y.
   ///
   /// FP=X: <what was fixed or why no fix is needed>
   /// FN=Y: <what was fixed, what remains, and root cause of any remaining FN>
   ```

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

### Phase 6: Integrate Back to Main (Default)

Do not leave retained progress only in a worktree/collector branch.

1. Ensure all retained progress is committed:
   - Accepted cop fixes: one commit per cop (preferred).
   - Useful investigation artifacts retained in repo (for example, reverted-attempt notes): separate commit.

2. Integrate those commit(s) into `main` immediately (unless the user explicitly says not to).
   If teammates committed in worktrees, cherry-pick from the worktree branch:
   ```bash
   git checkout main
   git cherry-pick <sha1> [<sha2> ...]
   ```
   If teammates committed directly on main (worktree isolation failed), their commits
   are already on main — just verify with `git log`.

3. Verify integration on `main`:
   ```bash
   git log --oneline -n 10
   git status --short --branch
   ```

4. Report exactly what was integrated (commit SHA(s) and short subjects).

5. If there is truly no repo-retained progress, explicitly report that no commit was made.

## Arguments

- `/fix-department` — **show the scoreboard, recommend a gem, and ask** which to target
- `/fix-department rubocop-performance` — target rubocop-performance directly
- `/fix-department rubocop-rspec` — target rubocop-rspec directly
- `/fix-department --input /path/to/corpus-results.json` — use local corpus file

## How to Choose the Next Gem

The scoreboard (`gem_progress.py --summary`) shows per-gem stats. Prioritize by:

1. **Zero untested cops** — only gems where every cop triggered on the corpus
   can claim true 100% conformance. Gems with untested cops get an asterisk. The "Untest"
   column in the scoreboard shows this.
2. **Fewest diverging cops** — less work to complete the gem. The "Dvrg" column shows this.
3. **Adoption value** — rubocop-performance is the most commonly added plugin, so completing
   it has more impact than rubocop-factory_bot, even if factory_bot is smaller.
4. **FP-free first** — a gem with 0 FP but some FN is already safe to adopt (no false alarms).
   Fix FNs later for completeness.
