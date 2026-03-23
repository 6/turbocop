#!/usr/bin/env python3
"""Generate canonical artifact path manifests for agent workflows."""

from __future__ import annotations

import argparse
from pathlib import Path

COMMON_RELATIVE = [
    "agent/agent.log",
    "agent/agent-events.jsonl",
    "agent/agent-last-message.txt",
    "agent/agent-result.json",
    "recovery/status.txt",
    "recovery/commits.txt",
    "recovery/diff.stat",
    "recovery/diff.diff",
    "recovery/diff.patch",
    "recovery/scope.md",
    "recovery/logfile-path.txt",
    "recovery/git-activity/before.json",
    "recovery/git-activity/after.json",
    "recovery/git-activity/**/*",
]

WORKFLOW_RELATIVE = {
    "agent-cop-fix": [
        "context/task.md",
        "context/final-task.md",
        "context/summary.md",
        "context/standard-cop-check.log",
    ],
    "agent-pr-repair": [
        "context/pr-diff.stat",
        "context/pr.diff",
        "context/final-task.md",
        "repair/repair.json",
        "repair/cop-check-packet.md",
        "repair/verify.sh",
        "repair/verify.log",
        "repair/summary.md",
        "repair/final-pr-diff.stat",
        "repair/final-pr.diff",
    ],
}

SESSION_GLOBS = [
    "~/.claude/projects/**/*.jsonl",
    "~/.codex/sessions/**/*.jsonl",
]


def _runtime_join(runtime_root: Path, relative: str) -> str:
    return str(runtime_root / relative)


def manifest_for(workflow: str, runtime_root: Path) -> list[str]:
    try:
        extra = WORKFLOW_RELATIVE[workflow]
    except KeyError as exc:
        raise SystemExit(f"unknown workflow manifest: {workflow}") from exc
    runtime_paths = [_runtime_join(runtime_root, rel) for rel in [*COMMON_RELATIVE, *extra]]
    return [*runtime_paths, *SESSION_GLOBS]


def write_manifest(workflow: str, runtime_root: Path, output: Path) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    paths = manifest_for(workflow, runtime_root)
    output.write_text("\n".join([*paths, str(output)]) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate agent workflow artifact manifests")
    parser.add_argument("workflow", choices=sorted(WORKFLOW_RELATIVE))
    parser.add_argument("--runtime-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    write_manifest(args.workflow, args.runtime_root.resolve(), args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
