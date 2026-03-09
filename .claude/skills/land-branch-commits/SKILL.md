---
name: land-branch-commits
description: Fetch remote branches, identify commits whose patches are not already on main, and cherry-pick only those commits onto main in a clean, verifiable order.
allowed-tools: Bash(*), Read, Write, Edit, Grep, Glob
---

# Land Branch Commits

Use this when the user wants commits from one or more branches landed onto
`main`, but only if those commits would be new to `main`.

## Workflow

1. Inspect the current git state without changing it:
   ```bash
   git status --short --branch
   git remote -v
   ```
   Treat unrelated working tree changes as off-limits.

2. Resolve branch names against the remote, then fetch:
   ```bash
   git ls-remote --heads origin
   ```
   The user-provided branch name may not exactly match the remote ref — it
   could be nested under any prefix. Search `ls-remote` output for refs whose
   name ends with the user-provided string. If the exact name isn't found but a
   unique suffix match exists, use that. If multiple matches exist, list them
   and ask the user to disambiguate.

   Then fetch the resolved refs:
   ```bash
   git fetch origin main <resolved-branch1> <resolved-branch2> ...
   ```

3. Identify patch-new commits for each branch:
   ```bash
   git cherry -v origin/main origin/<branch>
   git log --graph --oneline --decorate --boundary origin/main..origin/<branch>
   ```
   Only commits marked `+` in `git cherry` are candidates.
   Commits marked `-` are already present on `main` by patch equivalence and must
   not be cherry-picked.

4. Cherry-pick onto local `main`, **one commit at a time**:
   - Stay on `main`.
   - Cherry-pick each patch-new commit individually in oldest-first order.
     Do NOT pass multiple SHAs to a single `git cherry-pick` invocation.
   - Verify each cherry-pick succeeds before moving to the next.
   - If multiple branches are independent, keep the user's branch order unless
     file overlap suggests a safer order.
   ```bash
   git cherry-pick <sha1>
   # verify success, then:
   git cherry-pick <sha2>
   # ... and so on for each patch-new commit
   ```

5. If a cherry-pick conflicts:
   - Resolve the conflict in the working tree.
   - Continue with `git cherry-pick --continue`.
   - If the commit becomes empty and the patch is already effectively present,
     skip it with `git cherry-pick --skip`.
   - Do not use `git merge`, `git stash`, or destructive reset commands.

6. Verify the result:
   ```bash
   git status --short --branch
   git log --oneline --reverse origin/main..main
   git cherry -v main origin/<branch>
   ```
   After success, each source branch should have no remaining `+` commits versus
   local `main`.

## Reporting

Report:
- Which branch refs were fetched
- Which commits were patch-new and selected
- Which new commit SHAs now exist on `main`
- Which commits were excluded because they were already on `main`
- Whether `main` is ahead of `origin/main`, and whether anything was pushed

## Notes

- Prefer `git cherry` over `git log main..branch` for this task. It filters out
  duplicate patches with different SHAs.
- Compare against `origin/main` to avoid reapplying commits already landed
  upstream.
- If local `main` does not match `origin/main`, call out the exact state before
  cherry-picking.
- Do not push unless the user asks.

## Arguments

- `$land-branch-commits branch-a branch-b` - land only the commits from those
  branches that are patch-new to `main`
