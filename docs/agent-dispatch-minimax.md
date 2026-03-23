# Agent Dispatch: Claude Code + MiniMax on GHA

Legacy alternative to the Codex-based dispatch flow. This backend is still
available as a manual override in the workflows, but it is not the recommended
default for nitrocop cop-fix automation.

If you intentionally want to experiment with it, force it explicitly:

```bash
gh workflow run cop-issue-dispatch.yml -f max_active=5 -f backend_override=minimax
gh workflow run agent-cop-fix.yml -f cop="Style/VariableInterpolation" -f backend=minimax
```

Repository secret required:

| Secret | Value |
|--------|-------|
| `MINIMAX_API_KEY` | Your MiniMax API key |

The recommended operator flow is documented in [agent-dispatch.md](agent-dispatch.md).
