#!/usr/bin/env python3
"""Tests for prepare_agent_workspace.py."""

from __future__ import annotations

import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).parents[3] / "scripts" / "workflows" / "prepare_agent_workspace.py"


def git(repo: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", *args],
        cwd=str(repo),
        text=True,
        capture_output=True,
        check=True,
    )
    return result.stdout.strip()


def make_repo() -> Path:
    tmp = Path(tempfile.mkdtemp())
    git(tmp, "init")
    git(tmp, "config", "user.name", "Test Bot")
    git(tmp, "config", "user.email", "test@example.com")

    (tmp / "scripts" / "workflows").mkdir(parents=True)
    (tmp / "scripts" / "workflows" / "helper.py").write_text("print('ok')\n")
    (tmp / "scripts" / "check-cop.py").write_text("print('check')\n")
    (tmp / "scripts" / "corpus-smoke-test.py").write_text("print('smoke')\n")
    (tmp / "scripts" / "dispatch-cops.py").write_text("print('dispatch')\n")
    (tmp / "scripts" / "investigate-cop.py").write_text("print('investigate')\n")
    (tmp / "scripts" / "verify-cop-locations.py").write_text("print('verify')\n")
    (tmp / "scripts" / "noise.py").write_text("print('noise')\n")
    (tmp / ".github" / "workflows").mkdir(parents=True)
    (tmp / ".github" / "workflows" / "test.yml").write_text("name: test\n")
    (tmp / ".claude").mkdir()
    (tmp / ".claude" / "skill.txt").write_text("skill\n")
    (tmp / ".agents").mkdir()
    (tmp / ".agents" / "skill.txt").write_text("skill\n")
    (tmp / ".devcontainer").mkdir()
    (tmp / ".devcontainer" / "devcontainer.json").write_text("{}\n")
    (tmp / "docs").mkdir()
    (tmp / "docs" / "note.md").write_text("doc\n")
    (tmp / "gem").mkdir()
    (tmp / "gem" / "foo.rb").write_text("puts :ok\n")
    (tmp / "bench" / "corpus").mkdir(parents=True)
    (tmp / "bench" / "corpus" / "manifest.jsonl").write_text("{}\n")
    (tmp / "AGENTS.minimal.md").write_text("minimal instructions\n")
    (tmp / "AGENTS.md").write_text("full agents\n")
    (tmp / "CLAUDE.md").write_text("full claude\n")

    git(tmp, "add", ".")
    git(tmp, "commit", "-m", "init")
    return tmp


def run_script(repo: Path, mode: str, backend: str) -> tuple[subprocess.CompletedProcess[str], Path]:
    preserved = repo / "tmp-preserved-ci"
    result = subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            "--mode",
            mode,
            "--backend",
            backend,
            "--repo-root",
            str(repo),
            "--preserve-ci-scripts",
            str(preserved),
        ],
        cwd=str(repo),
        text=True,
        capture_output=True,
        check=True,
    )
    return result, preserved


def test_agent_cop_fix_codex_53_keeps_allowlisted_scripts_only():
    repo = make_repo()
    result, preserved = run_script(repo, "agent-cop-fix", "codex-5.3")

    assert "cleanup_sha=" in result.stdout
    assert (repo / "AGENTS.md").read_text() == "minimal instructions\n"
    assert (repo / "CLAUDE.md").read_text() == "minimal instructions\n"
    assert "scripts/check-cop.py" in result.stdout
    assert (repo / "scripts" / "check-cop.py").exists()
    assert (repo / "scripts" / "dispatch-cops.py").exists()
    assert (repo / "scripts" / "investigate-cop.py").exists()
    assert (repo / "scripts" / "verify-cop-locations.py").exists()
    assert not (repo / "scripts" / "corpus-smoke-test.py").exists()
    assert not (repo / "scripts" / "noise.py").exists()
    assert not (repo / "scripts" / "workflows").exists()
    assert not (repo / ".github").exists()
    assert not (repo / "docs").exists()
    assert not (repo / "gem").exists()
    assert (repo / "bench").exists()
    assert (preserved / "helper.py").exists()
    assert git(repo, "log", "--format=%s", "-1") == "tmp: clean workspace for agent"


def test_agent_cop_fix_minimax_prunes_all_scripts():
    repo = make_repo()
    result, preserved = run_script(repo, "agent-cop-fix", "minimax")

    assert "cleanup_sha=" in result.stdout
    assert (repo / "AGENTS.md").read_text() == "minimal instructions\n"
    assert (repo / "CLAUDE.md").read_text() == "minimal instructions\n"
    assert not (repo / "scripts").exists()
    assert not (repo / ".github").exists()
    assert not (repo / "docs").exists()
    assert not (repo / "gem").exists()
    assert (repo / "bench").exists()
    assert (preserved / "helper.py").exists()
    assert git(repo, "log", "--format=%s", "-1") == "tmp: clean workspace for agent"


def test_agent_pr_repair_codex_keeps_subset_scripts_and_bench():
    repo = make_repo()
    result, preserved = run_script(repo, "agent-pr-repair", "codex")

    assert "cleanup_sha=" in result.stdout
    assert (repo / "AGENTS.md").read_text() == "minimal instructions\n"
    assert (repo / "CLAUDE.md").read_text() == "minimal instructions\n"
    assert (repo / "scripts" / "check-cop.py").exists()
    assert (repo / "scripts" / "dispatch-cops.py").exists()
    assert (repo / "scripts" / "corpus-smoke-test.py").exists()
    assert (repo / "scripts" / "investigate-cop.py").exists()
    assert (repo / "scripts" / "verify-cop-locations.py").exists()
    assert not (repo / "scripts" / "noise.py").exists()
    assert not (repo / "scripts" / "workflows").exists()
    assert (repo / "bench" / "corpus" / "manifest.jsonl").exists()
    assert not (repo / ".github").exists()
    assert not (repo / "docs").exists()
    assert not (repo / "gem").exists()
    assert (preserved / "helper.py").exists()
    assert git(repo, "log", "--format=%s", "-1") == "tmp: clean workspace for agent"


if __name__ == "__main__":
    test_agent_cop_fix_codex_53_keeps_allowlisted_scripts_only()
    test_agent_cop_fix_minimax_prunes_all_scripts()
    test_agent_pr_repair_codex_keeps_subset_scripts_and_bench()
    print("All tests passed.")
