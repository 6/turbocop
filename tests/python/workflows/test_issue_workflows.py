#!/usr/bin/env python3
"""Smoke tests for issue-linked workflow wiring."""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).parents[3]
AGENT_COP_FIX = ROOT / ".github" / "workflows" / "agent-cop-fix.yml"
COP_FIX_LIFECYCLE = ROOT / "scripts" / "workflows" / "cop_fix_lifecycle.py"
AGENT_PR_REPAIR = ROOT / ".github" / "workflows" / "agent-pr-repair.yml"
BOT_COMMAND_WORKFLOW = ROOT / ".github" / "workflows" / "bot-command.yml"
RUN_AGENT_REMOTE = ROOT / ".github" / "actions" / "run-agent-remote" / "action.yml"
RUN_AGENT_LOCAL = ROOT / ".github" / "actions" / "run-agent" / "action.yml"
RUN_REPO_WRITE_REMOTE = ROOT / ".github" / "actions" / "run-repo-write-remote" / "action.yml"
REMOTE_AGENT_BRIDGE = ROOT / "scripts" / "workflows" / "remote_agent_bridge.py"
REMOTE_REPO_WRITE_BRIDGE = ROOT / "scripts" / "workflows" / "remote_repo_write_bridge.py"
BOT_COMMAND = ROOT / "scripts" / "workflows" / "bot_command.py"
COP_FIX_PUBLISH = ROOT / "scripts" / "workflows" / "cop_fix_publish.py"
REPAIR_PUBLISH = ROOT / "scripts" / "workflows" / "repair_publish.py"
COP_ISSUE_SYNC = ROOT / ".github" / "workflows" / "cop-issue-sync.yml"
CORPUS_ORACLE = ROOT / ".github" / "workflows" / "corpus-oracle.yml"


def test_agent_cop_fix_supports_issue_linking_and_auto_backend():
    yml = AGENT_COP_FIX.read_text()
    py = COP_FIX_LIFECYCLE.read_text()

    # Workflow inputs and orchestrator calls
    assert "issue_number:" in yml
    assert "- auto" in yml
    assert "Generate bot control token" in yml
    assert "cop_fix_lifecycle.py select-backend" in yml
    assert "cop_fix_publish.py skip-fixed-request" in yml
    assert "cop_fix_publish.py claim-request" in yml
    assert "cop_fix_lifecycle.py finalize" in yml
    assert "cop_fix_publish.py finalize-request" in yml
    assert "uses: ./.github/actions/run-agent-remote" in yml
    assert "uses: ./.github/actions/run-repo-write-remote" in yml
    assert "control_repo_token: ${{ steps.bot-control-token.outputs.token }}" in yml
    assert "target_ref: refs/heads/main" in yml
    assert "setup_profile: nitrocop" in yml
    assert "setup_config_json:" in yml
    assert "cop_fix_publish.py cleanup-request" in yml
    assert "Publish skip remotely" in yml
    assert "Publish finalize remotely" in yml
    assert 'gh pr list \\' in yml
    assert "--head \"${{ steps.init.outputs.branch }}\"" in yml

    # Logic now lives in cop_fix_lifecycle.py
    assert "dispatch_cops.py" in py
    assert "offense_fixtures_have_no_unannotated_blocks" in py
    assert "Refs #{" in py
    assert "nitrocop-cop-issue" in py
    assert "docs/agent-ci.md" in py
    assert "validate_agent_changes.py" in py
    assert '"gh", "issue", "comment"' not in py
    assert '"gh", "pr", "merge"' not in py
    publish = COP_FIX_PUBLISH.read_text()
    assert '"type": "create_branch"' in publish
    assert '"type": "create_pr"' in publish
    assert '"type": "edit_pr"' in publish
    assert '"type": "ready_pr"' in publish
    assert '"type": "merge_pr"' in publish
    assert '"type": "close_pr"' in publish
    assert '"match_mode": match_mode' in publish
    assert 'match_mode: str = "contained"' in publish
    assert '"type": "comment_pr"' in publish
    assert '"type": "edit_issue_labels"' in publish

    # Removed patterns should not appear in either
    assert "prepare_agent_workspace.py" not in yml
    assert "CI_SCRIPTS_DIR" not in yml
    assert "tmp: clean workspace" not in yml
    assert "git apply --3way" not in yml
    assert "CODEX_AUTH_JSON" not in yml
    assert "CLAUDE_CODE_OAUTH_TOKEN" not in yml
    assert "ANTHROPIC_API_KEY" not in yml


def test_agent_pr_repair_reads_linked_issue_and_can_update_it():
    content = AGENT_PR_REPAIR.read_text()
    publish = REPAIR_PUBLISH.read_text()
    assert '--json state --jq \'.state\'' in content
    assert "--json number,title,url,body,state" in content
    assert "--require-trusted-bot" in content
    assert "types: [closed]" in content
    assert "github.event.pull_request.number" in content
    assert "linked_issue_number" in content
    assert "Skip closed PRs" in content
    assert "Reconfirm PR is still repairable before agent" in content
    assert "Skip closed or moved PR before agent" in content
    assert "python3 scripts/workflows/repair_retry_policy.py live-gate" in content
    assert "uses: ./.github/actions/run-agent-remote" in content
    assert "uses: ./.github/actions/run-repo-write-remote" in content
    assert "Generate bot control token" in content
    assert "control_repo_token: ${{ steps.bot-control-token.outputs.token }}" in content
    assert "repositories: bot" in content
    assert "target_ref: refs/heads/${{ steps.pr.outputs.head_branch }}" in content
    assert "setup_profile: nitrocop" in content
    assert "setup_config_json:" in content
    assert "scripts/workflows/repair_publish.py" in content
    assert "Generate read-only GitHub token" not in content
    assert "Refresh app token" not in content
    assert "Skip closed or moved PR before publish" not in content
    assert 'echo "result=stale_pr" >> "$GITHUB_OUTPUT"' in content
    assert "Local Cop-Check Diagnosis" in content or "Precompute local cop-check diagnosis packet" in content
    assert "Detect local cop-check verification" in content
    assert 'steps.verify_meta.outputs.needs_local_cop_check == \'true\'' in content
    assert "validate_agent_changes.py" in content
    assert "guard_profile" in content
    assert 'python3 scripts/workflows/precompute_repair_cop_check.py' in content
    assert 'python3 scripts/workflows/count_tokens.py "$FINAL_TASK_FILE"' in content
    assert "<summary>Task prompt (" in publish
    assert "## Auto-repair Succeeded" in publish
    assert "## Auto-repair Failed Verification" in publish
    assert "prepare_agent_workspace.py" not in content
    assert "CI_SCRIPTS_DIR" not in content
    assert "repair-workspace-" not in content
    assert "git apply --3way" not in content
    assert "CODEX_AUTH_JSON" not in content
    assert "CLAUDE_CODE_OAUTH_TOKEN" not in content
    assert "ANTHROPIC_API_KEY" not in content
    assert "cop_fix_publish.py reset-issue-request" in content
    assert "Reset linked tracker issue remotely" in content


def test_agent_pr_repair_checks_out_repo_before_running_local_scripts():
    content = AGENT_PR_REPAIR.read_text()
    checkout_index = content.index("uses: actions/checkout@v6")
    pr_state_index = content.index("python3 scripts/workflows/repair_retry_policy.py pr-state")
    assert checkout_index < pr_state_index


def test_agent_pr_repair_live_gate_reads_branch_name():
    content = AGENT_PR_REPAIR.read_text()
    assert "--json state,baseRefName,isCrossRepository,headRepository,author,labels,headRefName,headRefOid" in content


def test_agent_pr_repair_distinguishes_agent_failure_from_verify_failure():
    content = AGENT_PR_REPAIR.read_text()
    publish = REPAIR_PUBLISH.read_text()
    assert "id: agent" in content
    assert 'if: always() && steps.pr.outputs.should_run == \'true\'' in content
    assert 'if [ "${{ steps.agent.outcome }}" != "success" ]; then' in content
    assert 'echo "result=agent_failed" >> "$GITHUB_OUTPUT"' in content
    assert 'echo "result=file_guard_failed" >> "$GITHUB_OUTPUT"' in content
    assert "## Auto-repair Agent Failed" in publish
    assert "## Auto-repair Verification Did Not Run" in publish
    assert "## Auto-repair Rejected" in publish
    assert "(verification did not run)" in content


def test_bot_command_workflow_dispatches_local_pr_repair() -> None:
    workflow = BOT_COMMAND_WORKFLOW.read_text()
    helper = BOT_COMMAND.read_text()

    assert 'name: Bot Command' in workflow
    assert "workflow_dispatch:" in workflow
    assert "python3 scripts/workflows/bot_command.py route" in workflow
    assert "python3 scripts/workflows/bot_command.py resolve-repair" in workflow
    assert "python3 scripts/workflows/bot_command.py resolve-fix" in workflow
    assert 'actions/workflows/agent-pr-repair.yml/dispatches' in workflow
    assert 'actions/workflows/agent-cop-fix.yml/dispatches' in workflow
    assert "## Bot repair not started" in workflow
    assert "## Bot fix not started" in workflow
    assert "## Bot command not started" in workflow
    assert "Requested by @" in workflow

    assert 'REPAIR_COMMAND = "/6bot repair"' in helper
    assert 'FIX_COMMAND = "/6bot fix"' in helper
    assert 'CHECKS_WORKFLOW_FILE = "checks.yml"' in helper
    assert 'subject_kind' in helper
    assert 'COP_TRACKER_RE' in helper
    assert 'repos/{repo}/actions/workflows/{CHECKS_WORKFLOW_FILE}/runs' in helper
    assert 'Checks is not currently failing for the current PR head' in helper
    assert 'source_repo input must match the target repository' in workflow


def test_remote_agent_bridge_contract_targets_6_bot():
    action = RUN_AGENT_REMOTE.read_text()
    bridge = REMOTE_AGENT_BRIDGE.read_text()

    assert not RUN_AGENT_LOCAL.exists()
    assert "default: 6/bot" in action
    assert "remote_agent_bridge.py prepare-input" in action
    assert "remote_agent_bridge.py dispatch" in action
    assert "remote_agent_bridge.py await-output" in action
    assert "--target-ref" in action
    assert "--setup-profile" in action
    assert "--setup-config-json" in action
    assert 'f"repos/{args.control_repo}/actions/workflows/{REMOTE_AGENT_WORKFLOW}/dispatches"' in bridge
    assert '"ref": CONTROL_REF' in bridge
    assert '"payload": json.dumps(payload)' in bridge
    assert '"target_ref": args.target_ref' in bridge
    assert '"setup_profile": args.setup_profile' in bridge
    assert '"setup_config_json": json.loads(args.setup_config_json)' in bridge
    assert '"workflow_dispatch"' in bridge
    assert 'dispatch.add_argument("--target-ref", required=True)' in bridge


def test_remote_repo_write_bridge_contract_targets_6_bot():
    action = RUN_REPO_WRITE_REMOTE.read_text()
    bridge = REMOTE_REPO_WRITE_BRIDGE.read_text()

    assert "default: 6/bot" in action
    assert "remote_repo_write_bridge.py prepare-input" in action
    assert "remote_repo_write_bridge.py dispatch" in action
    assert "remote_repo_write_bridge.py await-output" in action
    assert 'f"repos/{args.control_repo}/actions/workflows/{REMOTE_REPO_WRITE_WORKFLOW}/dispatches"' in bridge
    assert '"ref": CONTROL_REF' in bridge
    assert '"payload": json.dumps(payload)' in bridge
    assert '"target_ref": args.target_ref' in bridge
    assert '"workflow_dispatch"' in bridge
    assert 'dispatch.add_argument("--target-ref", required=True)' in bridge


def test_issue_sync_workflow_uses_github_token_and_dispatch_script():
    content = COP_ISSUE_SYNC.read_text()
    assert "actions/create-github-app-token@v3" not in content
    assert "GH_TOKEN: ${{ github.token }}" in content
    assert "python3 scripts/dispatch_cops.py issues-sync" in content
    assert "--binary target/debug/nitrocop" in content


def test_issue_close_workflow_uses_github_token() -> None:
    content = ROOT.joinpath(".github", "workflows", "cop-issue-close.yml").read_text()
    assert "actions/create-github-app-token@v3" not in content
    assert "GH_TOKEN: ${{ github.token }}" in content
    assert "gh issue close" in content



def test_corpus_oracle_workflow_uses_dynamic_pr_renderer():
    content = CORPUS_ORACLE.read_text()
    assert "scripts/workflows/render_corpus_oracle_pr.py" in content
    assert "--identity github-actions" in content
    assert "COMMIT_MSG=$(printf '%s' \"$PR_META\" | jq -r '.commit_message')" in content
    assert "PR_TITLE=$(printf '%s' \"$PR_META\" | jq -r '.pr_title')" in content
    assert "gh pr create \\" in content
    assert "--title \"$PR_TITLE\" \\" in content
    assert "--body-file \"$PR_BODY_FILE\" \\" in content


if __name__ == "__main__":
    test_agent_cop_fix_supports_issue_linking_and_auto_backend()
    test_agent_pr_repair_reads_linked_issue_and_can_update_it()
    test_agent_pr_repair_checks_out_repo_before_running_local_scripts()
    test_agent_pr_repair_distinguishes_agent_failure_from_verify_failure()
    test_issue_sync_workflow_uses_github_token_and_dispatch_script()
    test_issue_close_workflow_uses_github_token()
    test_corpus_oracle_workflow_uses_dynamic_pr_renderer()
    print("All tests passed.")
