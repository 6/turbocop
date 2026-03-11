---
name: fix-wrapup
description: Signal running fix-department/fix-cops/fix-repo agents to wrap up after the current phase, commit, and push.
---

# Fix Wrapup — Signal Fix Agents to Stop

This skill tells a running `/fix-department`, `/fix-cops`, or `/fix-repo` session
to wrap up. Use the team messaging tools to communicate with teammates.

## Default: `/fix-wrapup` (graceful)

Broadcast to all teammates telling them to finish their current step:

```
SendMessage(type="broadcast", content="Wrapup requested. Please finish your current step, commit your changes, and report back. Do not start new investigation work.", summary="Wrapup: finish current step and commit")
```

Then, as the team lead:
1. Wait for teammates to report back with their results
2. Collect and integrate results as normal (cherry-pick, verify)
3. Do NOT start a new batch of cops
4. Commit and push all progress
5. Report what was accomplished and what remains

## Urgent: `/fix-wrapup now`

Send shutdown requests to all teammates — don't wait for them to finish:

1. Use `TaskList` to find in-progress tasks and their owners
2. Send a `shutdown_request` to each teammate:
   ```
   SendMessage(type="shutdown_request", recipient="<teammate-name>", content="Urgent wrapup. Stop now.")
   ```
3. Collect results from any teammates that already reported back
4. Commit all current progress (even partial) to the branch
5. Push the branch
6. Report what was accomplished, what remains, and list any worktree branches
   from teammates that may have uncommitted work (recoverable via
   `/land-branch-commits`)

## Arguments

- `/fix-wrapup` — graceful: finish current step, then stop
- `/fix-wrapup now` — urgent: shutdown teammates, commit and push what you have
