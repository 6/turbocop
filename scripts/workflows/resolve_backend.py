#!/usr/bin/env python3
"""Resolve agent backend name to CLI, env vars, and log config.

Backend names map to a CLI tool and its configuration. Multiple backends
can share the same CLI (e.g., minimax and claude both use Claude Code).

Usage:
    python3 resolve_backend.py <backend>

Outputs KEY=VALUE lines suitable for sourcing in shell or appending to
$GITHUB_OUTPUT. All values are shell-safe (no quoting needed).
"""
import sys

BACKENDS = {
    "minimax": {
        "cli": "claude",
        "setup_cmd": (
            'python3 "$CI_SCRIPTS_DIR/guard_backend_secrets.py" '
            '--from-env MINIMAX_API_KEY '
            'emit-masks && '
            'curl -fsSL https://claude.ai/install.sh | bash'
        ),
        "log_format": "claude",
        "log_pattern": "~/.claude/projects/**/*.jsonl",
        "run_cmd": (
            'claude -p --dangerously-skip-permissions '
            '--output-format json '
            '"$(cat "$FINAL_TASK_FILE")" '
            '> "$AGENT_RESULT_FILE" '
            '2> >(tee "$AGENT_LOG_FILE" >&2) || true'
        ),
        "env": {
            "ANTHROPIC_BASE_URL": "https://api.minimax.io/anthropic",
            "ANTHROPIC_MODEL": "MiniMax-M2.7",
            "ANTHROPIC_SMALL_FAST_MODEL": "MiniMax-M2.7",
            "API_TIMEOUT_MS": "300000",
            "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1",
        },
        # Secret name -> env var mapping (secrets resolved by the workflow)
        "secrets": {
            "MINIMAX_API_KEY": "ANTHROPIC_AUTH_TOKEN",
        },
    },
    "claude": {
        "cli": "claude",
        "setup_cmd": (
            'python3 "$CI_SCRIPTS_DIR/guard_backend_secrets.py" '
            '--from-env ANTHROPIC_API_KEY '
            'emit-masks && '
            'curl -fsSL https://claude.ai/install.sh | bash'
        ),
        "log_format": "claude",
        "log_pattern": "~/.claude/projects/**/*.jsonl",
        "run_cmd": (
            'claude -p --dangerously-skip-permissions '
            '--output-format json '
            '"$(cat "$FINAL_TASK_FILE")" '
            '> "$AGENT_RESULT_FILE" '
            '2> >(tee "$AGENT_LOG_FILE" >&2) || true'
        ),
        "env": {
            "API_TIMEOUT_MS": "300000",
            "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1",
        },
        "secrets": {
            "ANTHROPIC_API_KEY": "ANTHROPIC_API_KEY",
        },
    },
    "codex": {
        "cli": "codex",
        "setup_cmd": (
            'python3 "$CI_SCRIPTS_DIR/guard_backend_secrets.py" '
            '--from-env CODEX_AUTH_JSON '
            'emit-masks && '
            'python3 "$CI_SCRIPTS_DIR/validate_codex_auth.py" '
            '--from-env CODEX_AUTH_JSON '
            '--max-age-days 7 && '
            'npm install -g @openai/codex@latest && '
            'mkdir -p ~/.codex && '
            'chmod 700 ~/.codex && '
            'printf \'%s\' "$CODEX_AUTH_JSON" > ~/.codex/auth.json && '
            'chmod 600 ~/.codex/auth.json'
        ),
        "log_format": "codex",
        "log_pattern": "~/.codex/sessions/**/*.jsonl",
        "run_cmd": (
            '( codex exec --dangerously-bypass-approvals-and-sandbox -m gpt-5.4 '
            '-c model_reasoning_effort=xhigh '
            '--json '
            '-o "$AGENT_LAST_MESSAGE_FILE" '
            '- < "$FINAL_TASK_FILE" '
            '> "$AGENT_EVENTS_FILE" '
            '2> >(tee "$AGENT_LOG_FILE" >&2); '
            'STATUS=$?; '
            'python3 "$CI_SCRIPTS_DIR/agent_logs.py" summarize '
            '"$AGENT_EVENTS_FILE" '
            '"$AGENT_LAST_MESSAGE_FILE" '
            '> "$AGENT_RESULT_FILE" || true; '
            'exit $STATUS ) || true'
        ),
        "env": {},
        "secrets": {
            "CODEX_AUTH_JSON": "CODEX_AUTH_JSON",
        },
    },
}


def resolve(backend: str) -> dict:
    """Resolve a backend name to its full config."""
    if backend not in BACKENDS:
        print(f"Unknown backend: {backend}", file=sys.stderr)
        print(f"Available: {', '.join(BACKENDS)}", file=sys.stderr)
        sys.exit(1)
    return BACKENDS[backend]


def main():
    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} <backend>", file=sys.stderr)
        sys.exit(1)

    backend = sys.argv[1]
    config = resolve(backend)

    # Output key=value pairs
    print(f"cli={config['cli']}")
    print(f"setup_cmd={config['setup_cmd']}")
    print(f"log_format={config['log_format']}")
    print(f"log_pattern={config['log_pattern']}")
    print(f"run_cmd={config['run_cmd']}")

    # Output env vars
    for key, val in config["env"].items():
        print(f"env_{key}={val}")

    # Output secret mappings
    for secret_name, env_var in config["secrets"].items():
        print(f"secret_{secret_name}={env_var}")


if __name__ == "__main__":
    main()
