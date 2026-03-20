---
name: fix-continue
description: Tell a fix-department session to continue working on the next batch.
---

# Fix Continue — Resume a Fix Session

Continue with the fix workflow:

- If you were between batches, proceed to Phase 1 (plan the next batch)
- If you were collecting results, finish collecting and then plan the next batch
- If you were in Phase 0 assessment, continue from where you left off
- If the session was running in `--loop` mode, preserve that behavior:
  do not pause on non-blocking questions and keep resuming after commit / land /
  push checkpoints until the target is done or explicitly stopped
- Use this after any out-of-band status / commit / rebase / push turn if the
  prior `--loop` run did not automatically resume at the turn boundary
- If you are resuming in a fresh cloud/worktree environment, rerun the
  `fix-department` bootstrap first (`git submodule update --init --recursive`,
  corpus bundle setup, and only clone `vendor/corpus` repos on demand if a
  step needs local corpus source files)

Resume the normal workflow as described in the skill instructions.

## Arguments

- `/fix-continue` — resume the fix loop
- `/fix-continue --loop` — resume a persistent no-pause fix loop
