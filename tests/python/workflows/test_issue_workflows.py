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
    assert "docs/agent-ci.md" in content
    assert "Switch to claimed PR branch" in content
    assert "validate_agent_changes.py" in content
    assert "Run local standard quick corpus gate" in content
    assert "requires_standard_quick_gate" in content
    assert "Keep draft PR when local standard gate fails" in content
    assert "bundle check || bundle install" in content
    assert 'python3 scripts/check-cop.py "${{ github.event.inputs.cop }}"' in content
    assert 'gh pr merge "$PR_URL" --auto --squash --delete-branch' in content
    assert "prepare_agent_workspace.py" not in content
    assert "CI_SCRIPTS_DIR" not in content
    assert "tmp: clean workspace" not in content
    assert "git apply --3way" not in content


def test_agent_pr_repair_reads_linked_issue_and_can_update_it():
    content = AGENT_PR_REPAIR.read_text()
    assert '--json state --jq \'.state\'' in content
    assert "--json number,title,url,body,state" in content
    assert "linked_issue_number" in content
    assert 'gh issue comment "${{ steps.pr.outputs.linked_issue_number }}"' in content
    assert '--add-label "state:blocked"' in content
    assert "Skip closed PRs" in content
    assert "Local Cop-Check Diagnosis" in content or "Precompute local cop-check diagnosis packet" in content
    assert '<summary>Task prompt (${{ steps.prompt.outputs.tokens }} tokens)</summary>' in content
    assert 'cat "$FINAL_TASK_FILE"' in content
    assert "Detect local cop-check verification" in content
    assert 'steps.verify_meta.outputs.needs_local_cop_check == \'true\'' in content
    assert "validate_agent_changes.py" in content
    assert "guard_profile" in content
    assert 'python3 scripts/workflows/precompute_repair_cop_check.py' in content
    assert 'python3 scripts/workflows/count_tokens.py "$FINAL_TASK_FILE"' in content
    assert "prepare_agent_workspace.py" not in content
    assert "CI_SCRIPTS_DIR" not in content
    assert "repair-workspace-" not in content
    assert "git apply --3way" not in content


def test_agent_pr_repair_checks_out_repo_before_running_local_scripts():
    content = AGENT_PR_REPAIR.read_text()
    checkout_index = content.index("uses: actions/checkout@v4")
    pr_state_index = content.index("python3 scripts/workflows/repair_retry_policy.py pr-state")
    assert checkout_index < pr_state_index


def test_agent_pr_repair_distinguishes_agent_failure_from_verify_failure():
    content = AGENT_PR_REPAIR.read_text()
    assert "id: agent" in content
    assert 'if: always() && steps.pr.outputs.should_run == \'true\'' in content
    assert 'if [ "${{ steps.agent.outcome }}" != "success" ]; then' in content
    assert 'echo "result=agent_failed" >> "$GITHUB_OUTPUT"' in content
    assert 'echo "result=file_guard_failed" >> "$GITHUB_OUTPUT"' in content
    assert "## Auto-repair Agent Failed" in content
    assert "## Auto-repair Verification Did Not Run" in content
    assert "## Auto-repair Rejected" in content
    assert "(verification did not run)" in content


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
    test_agent_pr_repair_distinguishes_agent_failure_from_verify_failure()
    test_issue_sync_workflow_uses_app_token_and_dispatch_script()
    test_issue_dispatch_workflow_uses_app_token_and_dispatch_script()
    test_investigate_regression_workflow_uses_script()
    print("All tests passed.")
