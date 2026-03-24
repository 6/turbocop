# Agent CI Rules

These rules apply when `GITHUB_ACTIONS` is set and the workflow is driving the agent, such as `agent-cop-fix` and `agent-pr-repair`.

## Branch And Git Rules

- Work only in the current checked-out branch. The workflow has already switched to the target PR branch for you.
- Do not create extra branches or `git worktree`s.
- Do not use `git stash`.
- Do not revert the branch to `origin/main` or collapse the PR into an empty diff.
- Commit on the current branch only when the task explicitly requires a commit.

## Scope Rules

- Stay within the file scope implied by the workflow route.
- `agent-cop-fix` is limited to cop implementation and corpus-fixture files.
- `agent-pr-repair` is limited by the failing route:
  - Rust/test repairs: Rust sources, tests, and bench files.
  - Python/workflow repairs: `scripts/`, `tests/python/`, workflow YAML, and Python metadata.
  - `cop-check` repairs: cop sources, cop fixtures, `tests/integration.rs`, and `bench/corpus/`.
  - smoke/systemic repairs: broad source, test, bench, and script paths only when the failure truly requires that scope.
- The workflow enforces these scopes after the agent runs. Edits outside the allowed scope will fail the run.

## How To Work

- Read the task prompt first, then inspect the existing PR diff if this is a repair.
- Prefer the provided helper scripts over ad hoc corpus debugging when they directly answer the question.
- Use local corpus artifacts and cached repos when they are already present in the prompt or runtime files.
- Keep fixes narrow. The workflow prefers a small correct fix over a broad cleanup.
- Add or update tests with every real behavior fix.

## Helper Script Conventions

- Public helper CLIs live in `scripts/`.
- Workflow internals live in `scripts/workflows/`.
- Shared importable helpers live in `scripts/shared/`.
- Use the stable top-level CLI paths shown in the prompt, for example:
  - `python3 scripts/check_cop.py Department/CopName --verbose --rerun --quick --clone`
  - `python3 scripts/investigate_cop.py Department/CopName --context`
  - `python3 scripts/dispatch_cops.py changed --base origin/main --head HEAD`

## Failure Handling

- If the only plausible resolution is a full revert of the PR, stop and say so clearly instead of doing the revert.
- If required context is missing, explain the blocker in the final message rather than improvising a broad change.
