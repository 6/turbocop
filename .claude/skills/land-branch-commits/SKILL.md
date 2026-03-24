---
name: land-branch-commits
description: Fetch remote branches, identify commits whose patches are not already on main, and cherry-pick only those commits onto main in a clean, verifiable order.
allowed-tools: Bash(*), Read, Write, Edit, Grep, Glob
---

# Land Branch Commits

Use this when the user wants commits from one or more branches landed onto
`main`, but only if those commits would be new to `main`.

## Workflow

**Run the script first.** The happy path is fully automated:

```bash
.claude/skills/land-branch-commits/scripts/land_branch_commits.sh <branch1> [branch2 ...]
```

The script handles:
- Resolving branch names against origin (suffix match, disambiguation)
- Fetching, identifying patch-new commits via `git cherry`
- Cherry-picking one at a time in oldest-first order
- Stripping `claude.ai` URLs and `Co-Authored-By` lines from messages
- Appending `Co-Authored-By` trailer when the original author differs from the
  local git user
- Preserving the original author date on each commit
- Resetting authorship to the local git user
- Verifying all commits landed with `git cherry`

**Exit codes:**
- `0` — all commits landed successfully
- `1` — usage error, branch resolution failure, or pre-condition failure
- `2` — cherry-pick conflict; the script stops mid-way

## Conflict Resolution (LLM fallback)

If the script exits with code 2, a cherry-pick conflict is in progress.
Pick up from here:

1. Inspect the conflict:
   ```bash
   git status
   git diff
   ```
2. Resolve the conflict in the working tree.
3. Continue with `git cherry-pick --continue`.
4. If the commit becomes empty (patch already present), skip with
   `git cherry-pick --skip`.
5. After resolving, **re-run the script with the same arguments** — it will
   skip already-landed commits and continue with the remaining ones.
   Always prefer re-running the script over manual cherry-picks so that
   authorship reset, message cleanup, and trailer logic are applied
   automatically.
6. If you must cherry-pick manually (e.g., the script keeps failing), apply
   the same fixups the script would:
   - Reset author to the local git user (do not keep the branch author).
   - Strip any `claude.ai` URLs from the commit message.
   - Strip existing `Co-Authored-By` lines from the original message.
   - Append a `Co-Authored-By` trailer for the original branch author
     when they differ from the local git user.
7. Do not use `git merge`, `git stash`, or destructive reset commands.

## Reporting

Report:
- Which branch refs were fetched
- Which commits were patch-new and selected
- Which new commit SHAs now exist on `main`
- Which commits were excluded because they were already on `main`
- Whether `main` is ahead of `origin/main`, and whether anything was pushed

If thread history shows an active `/fix-department --loop` run, landing commits
onto `main` is only an integration checkpoint, not the end of the overall
task. After reporting the landed SHAs, return to the interrupted fix workflow
unless the user explicitly stopped or redirected the run. If you cannot safely
resume in the same turn, say that the loop is still active and tell the user to
run `$fix-continue --loop`.

## Notes

- Do not push unless the user asks.
- The script compares against local `main` (not `origin/main`) when picking
  commits, so previously landed but un-pushed commits won't be duplicated.
- If local `main` diverges from `origin/main`, the script warns but proceeds.

## Arguments

- `$land-branch-commits branch-a branch-b` - land only the commits from those
  branches that are patch-new to `main`
