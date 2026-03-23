#!/usr/bin/env python3
"""Smoke tests for issue-linked workflow wiring."""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).parents[3]
AGENT_COP_FIX = ROOT / ".github" / "workflows" / "agent-cop-fix.yml"
AGENT_PR_REPAIR = ROOT / ".github" / "workflows" / "agent-pr-repair.yml"
COP_ISSUE_SYNC = ROOT / ".github" / "workflows" / "cop-issue-sync.yml"
COP_ISSUE_DISPATCH = ROOT / ".github" / "workflows" / "cop-issue-dispatch.yml"
INVESTIGATE_REGRESSION = ROOT / ".github" / "workflows" / "investigate-regression.yml"


def test_agent_cop_fix_supports_issue_linking_and_auto_backend():
    content = AGENT_COP_FIX.read_text()
    assert "issue_number:" in content
    assert "- auto" in content
    assert "scripts/dispatch-cops.py backend" in content
    assert "cargo test --test integration offense_fixtures_have_no_unannotated_blocks" in content
    assert "Closes #${ISSUE_NUMBER}" in content
    assert "<!-- nitrocop-cop-issue: number=${ISSUE_NUMBER} cop=${COP} -->" in content
    assert 'gh issue comment "${{ github.event.inputs.issue_number }}"' in content


def test_agent_pr_repair_reads_linked_issue_and_can_update_it():
    content = AGENT_PR_REPAIR.read_text()
    assert "--json number,title,url,body,state" in content
    assert "linked_issue_number" in content
    assert 'gh issue comment "${{ steps.pr.outputs.linked_issue_number }}"' in content
    assert '--add-label "state:blocked"' in content


def test_agent_pr_repair_checks_out_repo_before_running_local_scripts():
    content = AGENT_PR_REPAIR.read_text()
    checkout_index = content.index("uses: actions/checkout@v4")
    pr_state_index = content.index("python3 scripts/workflows/repair_retry_policy.py pr-state")
    assert checkout_index < pr_state_index


def test_issue_sync_workflow_uses_app_token_and_dispatch_script():
    content = COP_ISSUE_SYNC.read_text()
    assert "actions/create-github-app-token@v1" in content
    assert "python3 scripts/dispatch-cops.py issues-sync" in content
    assert "--binary target/debug/nitrocop" in content


def test_issue_dispatch_workflow_uses_app_token_and_dispatch_script():
    content = COP_ISSUE_DISPATCH.read_text()
    assert "actions/create-github-app-token@v1" in content
    assert "python3 scripts/dispatch-cops.py dispatch-issues" in content
    assert "--max-active" in content


def test_investigate_regression_workflow_uses_script():
    content = INVESTIGATE_REGRESSION.read_text()
    assert "actions/create-github-app-token@v1" in content
    assert "python3 scripts/investigate-regression.py" in content
    assert "dispatch-simple" in content


if __name__ == "__main__":
    test_agent_cop_fix_supports_issue_linking_and_auto_backend()
    test_agent_pr_repair_reads_linked_issue_and_can_update_it()
    test_agent_pr_repair_checks_out_repo_before_running_local_scripts()
    test_issue_sync_workflow_uses_app_token_and_dispatch_script()
    test_issue_dispatch_workflow_uses_app_token_and_dispatch_script()
    test_investigate_regression_workflow_uses_script()
    print("All tests passed.")
