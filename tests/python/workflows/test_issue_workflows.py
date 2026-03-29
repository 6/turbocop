#!/usr/bin/env python3
"""Smoke tests for issue-linked and PR-repair workflow wiring."""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).parents[3]
CHECKS = ROOT / ".github" / "workflows" / "checks.yml"
PR_CLOSE_RESET = ROOT / ".github" / "workflows" / "pr-close-reset.yml"
REPO_TASK = ROOT / "scripts" / "workflows" / "repo_task.py"
BOT_COMMAND = ROOT / "scripts" / "workflows" / "bot_command.py"
COP_FIX_LIFECYCLE = ROOT / "scripts" / "workflows" / "cop_fix_lifecycle.py"
COP_FIX_PUBLISH = ROOT / "scripts" / "workflows" / "cop_fix_publish.py"
REPAIR_PUBLISH = ROOT / "scripts" / "workflows" / "repair_publish.py"
COP_ISSUE_SYNC = ROOT / ".github" / "workflows" / "cop-issue-sync.yml"
CORPUS_ORACLE = ROOT / ".github" / "workflows" / "corpus-oracle.yml"
RELEASE = ROOT / ".github" / "workflows" / "release.yml"

AGENT_PR_REPAIR = ROOT / ".github" / "workflows" / "agent-pr-repair.yml"
BOT_COMMAND_WORKFLOW = ROOT / ".github" / "workflows" / "bot-command.yml"
RUN_AGENT_REMOTE = ROOT / ".github" / "actions" / "run-agent-remote" / "action.yml"
RUN_REPO_WRITE_REMOTE = ROOT / ".github" / "actions" / "run-repo-write-remote" / "action.yml"
REMOTE_AGENT_BRIDGE = ROOT / "scripts" / "workflows" / "remote_agent_bridge.py"
REMOTE_REPO_WRITE_BRIDGE = ROOT / "scripts" / "workflows" / "remote_repo_write_bridge.py"


def test_cop_fix_logic_lives_in_repo_task_and_publish_helpers() -> None:
    planner = REPO_TASK.read_text()
    lifecycle = COP_FIX_LIFECYCLE.read_text()
    publish = COP_FIX_PUBLISH.read_text()

    assert '"fix_issue"' in planner
    assert '"agent-cop-fix"' in planner
    assert '"select-backend"' in planner
    assert '"skip-fixed-request"' in planner
    assert '"claim-request"' in planner
    assert '"finalize-request"' in planner
    assert '"cleanup-request"' in planner
    assert "wait_healthy_main.py" in planner
    assert "gh workflow run agent-cop-fix.yml" not in planner

    assert "dispatch_cops.py" in lifecycle
    assert "offense_fixtures_have_no_unannotated_blocks" in lifecycle
    assert "nitrocop-cop-issue" in lifecycle
    assert '"gh", "issue", "comment"' not in lifecycle
    assert '"gh", "pr", "merge"' not in lifecycle

    assert '"type": "create_branch"' in publish
    assert '"type": "create_pr"' in publish
    assert '"type": "edit_pr"' in publish
    assert '"type": "ready_pr"' in publish
    assert '"type": "merge_pr"' in publish
    assert '"type": "close_pr"' in publish


def test_repo_task_handles_pr_repair_inside_control_plane() -> None:
    content = REPO_TASK.read_text()

    assert '"repair_pr"' in content
    assert '"agent-pr-repair"' in content
    assert "prepare_pr_repair.py" in content
    assert "repair_retry_policy.py" in content
    assert "repair_publish.py" in content
    assert "precompute_repair_cop_check.py" in content
    assert "render_helper_scripts_section.py" in content
    assert "validate_agent_changes.py" in content
    assert 'nitrocop-auto-repair-request' in content
    assert "run-agent-remote" not in content
    assert "run-repo-write-remote" not in content


def test_checks_workflow_comments_with_github_token_and_auto_mentions_6() -> None:
    content = CHECKS.read_text()

    assert "actions/create-github-app-token@v3" not in content
    assert "GH_TOKEN: ${{ github.token }}" in content
    assert "@6 Please repair the latest failing Checks run on this PR." in content
    assert "nitrocop-auto-repair-request: checks_run_id=${{ github.run_id }}" in content
    assert "gh pr comment" in content


def test_pr_close_reset_workflow_uses_github_token_only() -> None:
    content = PR_CLOSE_RESET.read_text()

    assert "pull_request:" in content
    assert "types: [closed]" in content
    assert "GH_TOKEN: ${{ github.token }}" in content
    assert "actions/create-github-app-token@v3" not in content
    assert "gh issue edit" in content
    assert "state:pr-open,state:dispatched" in content
    assert "state:backlog" in content


def test_bot_command_helpers_still_route_issue_and_pr_triggers() -> None:
    helper = BOT_COMMAND.read_text()

    assert 'MENTION_TRIGGER = "mention"' in helper
    assert 'ASSIGNMENT_TRIGGER = "assignment"' in helper
    assert "build_issue_assignment_prompt" in helper
    assert "Checks is not currently failing for the current PR head" in helper
    assert 'payload.trigger_kind must be mention or assignment' in helper
    assert 'action="repair_pr"' in helper
    assert 'action="fix_issue"' in helper


def test_old_local_pr_repair_workflow_and_bridges_are_removed() -> None:
    assert not AGENT_PR_REPAIR.exists()
    assert not BOT_COMMAND_WORKFLOW.exists()
    assert not RUN_AGENT_REMOTE.exists()
    assert not RUN_REPO_WRITE_REMOTE.exists()
    assert not REMOTE_AGENT_BRIDGE.exists()
    assert not REMOTE_REPO_WRITE_BRIDGE.exists()


def test_issue_sync_workflow_uses_github_token_and_dispatch_script() -> None:
    content = COP_ISSUE_SYNC.read_text()
    assert "actions/create-github-app-token@v3" not in content
    assert "GH_TOKEN: ${{ github.token }}" in content
    assert "python3 scripts/dispatch_cops.py issues-sync" in content
    assert "--binary target/debug/nitrocop" in content


def test_corpus_oracle_workflow_uses_dynamic_pr_renderer() -> None:
    content = CORPUS_ORACLE.read_text()
    assert "scripts/workflows/render_corpus_oracle_pr.py" in content
    assert "GH_TOKEN: ${{ github.token }}" in content
    assert "actions/create-github-app-token@v3" not in content
    assert "--identity github-actions" in content
    assert 'gh pr create \\' in content


def test_release_workflow_commits_directly_to_main() -> None:
    content = RELEASE.read_text()
    assert "Release workflow must run from main" in content
    assert "GH_TOKEN: ${{ github.token }}" in content
    assert "--identity github-actions" in content
    assert "git push origin main" in content
    assert "actions/create-github-app-token@v3" not in content


if __name__ == "__main__":
    test_cop_fix_logic_lives_in_repo_task_and_publish_helpers()
    test_repo_task_handles_pr_repair_inside_control_plane()
    test_checks_workflow_comments_with_github_token_and_auto_mentions_6()
    test_pr_close_reset_workflow_uses_github_token_only()
    test_bot_command_helpers_still_route_issue_and_pr_triggers()
    test_old_local_pr_repair_workflow_and_bridges_are_removed()
    test_issue_sync_workflow_uses_github_token_and_dispatch_script()
    test_corpus_oracle_workflow_uses_dynamic_pr_renderer()
    test_release_workflow_commits_directly_to_main()
    print("All tests passed.")
