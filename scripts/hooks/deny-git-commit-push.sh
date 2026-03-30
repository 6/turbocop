#!/usr/bin/env bash
set -euo pipefail

# Claude Code PreToolUse hook: deny git commit/push during CI agent runs.
#
# Registered dynamically by exclude_agent_context.py so it only activates
# in CI, not during local development.

input=$(cat)
command=$(echo "$input" | jq -r '.tool_input.command // empty')

if [ -z "$command" ]; then
  exit 0
fi

# Match git commit or git push anywhere in the command.
# Handles: git commit, git -c user.name=bot commit, git push --force, etc.
if echo "$command" | grep -qE '\bgit\b.*\b(commit|push)\b'; then
  cat >&2 <<'MSG'
DENIED: git commit and git push are not allowed during CI agent runs.

The workflow's finalize step handles committing and pushing AFTER you exit.
It also runs cargo fmt, scope validation, and signed-commit promotion.

Your code changes are preserved as unstaged/staged modifications in the
working tree. The workflow will commit them for you.

Just make your code changes, verify they work, and finish your task.
MSG
  exit 2
fi

exit 0
