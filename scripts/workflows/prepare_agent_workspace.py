#!/usr/bin/env python3
"""Prepare a reduced agent workspace for workflow-driven agent runs.

This script centralizes the repo-pruning logic used by agent workflows. It:

- preserves `scripts/workflows` to a temporary location for post-cleanup helpers
- replaces `AGENTS.md` / `CLAUDE.md` with `AGENTS.minimal.md`
- removes mode-specific high-noise paths from the git checkout
- commits the cleanup as a temporary workspace-only commit
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import tempfile
from pathlib import Path

SCRIPT_ALLOWLISTS = {
    "agent-cop-fix": {
        "codex-5.3": [
            "scripts/check-cop.py",
            "scripts/investigate-cop.py",
            "scripts/verify-cop-locations.py",
            "scripts/dispatch-cops.py",
        ],
        "codex": [
            "scripts/check-cop.py",
            "scripts/investigate-cop.py",
            "scripts/verify-cop-locations.py",
            "scripts/dispatch-cops.py",
        ],
    },
    "agent-pr-repair": {
        "codex-5.3": [
            "scripts/check-cop.py",
            "scripts/corpus-smoke-test.py",
            "scripts/investigate-cop.py",
            "scripts/verify-cop-locations.py",
            "scripts/dispatch-cops.py",
        ],
        "codex": [
            "scripts/check-cop.py",
            "scripts/corpus-smoke-test.py",
            "scripts/investigate-cop.py",
            "scripts/verify-cop-locations.py",
            "scripts/dispatch-cops.py",
        ],
    },
}


MODES = {
    "agent-cop-fix": {
        "remove_paths": [
            "AGENTS.minimal.md",
            ".claude",
            ".agents",
            ".devcontainer",
            ".github",
            "docs",
            "gem",
            "scripts",
        ],
        "commit_message": "tmp: clean workspace for agent",
    },
    "agent-pr-repair": {
        "remove_paths": [
            "AGENTS.minimal.md",
            ".claude",
            ".agents",
            ".devcontainer",
            ".github",
            "docs",
            "gem",
            "scripts",
        ],
        "commit_message": "tmp: clean workspace for agent",
    },
}


def run(cmd: list[str], *, cwd: Path, capture_output: bool = False) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        cmd,
        cwd=str(cwd),
        text=True,
        capture_output=capture_output,
        check=True,
    )


def preserve_ci_scripts(repo_root: Path, dest: Path) -> None:
    src = repo_root / "scripts" / "workflows"
    if not src.exists():
        raise FileNotFoundError(f"{src} not found")
    if dest.exists():
        shutil.rmtree(dest)
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(src, dest)


def preserve_relative_paths(repo_root: Path, dest: Path, relative_paths: list[str]) -> None:
    if dest.exists():
        shutil.rmtree(dest)
    dest.mkdir(parents=True, exist_ok=True)
    for rel_path in relative_paths:
        src = repo_root / rel_path
        if not src.exists():
            continue
        target = dest / rel_path
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, target)


def replace_agent_docs(repo_root: Path) -> None:
    minimal = repo_root / "AGENTS.minimal.md"
    if not minimal.exists():
        raise FileNotFoundError(f"{minimal} not found")
    content = minimal.read_text()
    (repo_root / "AGENTS.md").write_text(content)
    (repo_root / "CLAUDE.md").write_text(content)
    run(["git", "add", "AGENTS.md", "CLAUDE.md"], cwd=repo_root)


def prune_paths(repo_root: Path, remove_paths: list[str]) -> None:
    cmd = ["git", "rm", "-r", "--quiet", "--ignore-unmatch", "--", *remove_paths]
    run(cmd, cwd=repo_root)


def restore_relative_paths(repo_root: Path, src_root: Path, relative_paths: list[str]) -> None:
    restored = False
    for rel_path in relative_paths:
        src = src_root / rel_path
        if not src.exists():
            continue
        target = repo_root / rel_path
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, target)
        restored = True
    if restored:
        run(["git", "add", "--", *relative_paths], cwd=repo_root)


def has_staged_or_worktree_changes(repo_root: Path) -> bool:
    result = subprocess.run(
        ["git", "status", "--porcelain"],
        cwd=str(repo_root),
        text=True,
        capture_output=True,
        check=True,
    )
    return bool(result.stdout.strip())


def allowed_scripts(mode: str, backend: str) -> list[str]:
    return SCRIPT_ALLOWLISTS.get(mode, {}).get(backend, [])


def prepare_workspace(mode: str, backend: str, repo_root: Path, preserve_ci_to: Path | None) -> str:
    config = MODES[mode]
    script_allowlist = allowed_scripts(mode, backend)

    if preserve_ci_to is not None:
        preserve_ci_scripts(repo_root, preserve_ci_to)

    with tempfile.TemporaryDirectory(prefix="agent-workspace-scripts-") as tmpdir:
        preserved_scripts = Path(tmpdir)
        if script_allowlist:
            preserve_relative_paths(repo_root, preserved_scripts, script_allowlist)

        replace_agent_docs(repo_root)
        prune_paths(repo_root, config["remove_paths"])
        if script_allowlist:
            restore_relative_paths(repo_root, preserved_scripts, script_allowlist)

    if not has_staged_or_worktree_changes(repo_root):
        return subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=str(repo_root),
            text=True,
            capture_output=True,
            check=True,
        ).stdout.strip()

    run(["git", "commit", "-m", config["commit_message"]], cwd=repo_root)
    return subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=str(repo_root),
        text=True,
        capture_output=True,
        check=True,
    ).stdout.strip()


def main() -> int:
    parser = argparse.ArgumentParser(description="Prepare a reduced agent workspace")
    parser.add_argument("--mode", choices=sorted(MODES), required=True)
    parser.add_argument(
        "--backend",
        choices=["claude", "codex", "codex-5.3", "minimax"],
        default="codex",
        help="Agent backend; controls which helper scripts are kept",
    )
    parser.add_argument(
        "--preserve-ci-scripts",
        type=Path,
        help="Copy scripts/workflows to this path before pruning the workspace",
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path.cwd(),
        help="Git repository root (default: current directory)",
    )
    args = parser.parse_args()

    repo_root = args.repo_root.resolve()
    cleanup_sha = prepare_workspace(args.mode, args.backend, repo_root, args.preserve_ci_scripts)
    print(f"cleanup_sha={cleanup_sha}")
    print(f"available_scripts={','.join(allowed_scripts(args.mode, args.backend))}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
