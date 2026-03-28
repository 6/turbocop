#!/usr/bin/env python3
"""Build remote repo-write requests for agent-pr-repair publish phases."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path


def _write(path: Path, content: str) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)
    return path


def _dump_request(output_dir: Path, *, target_sha: str, target_ref: str, operations: list[dict]) -> int:
    output_dir.mkdir(parents=True, exist_ok=True)
    request = {
        "match_mode": "current_head",
        "operations": operations,
    }
    _write(output_dir / "request.json", json.dumps(request, indent=2) + "\n")
    print(f"target_sha={target_sha}")
    print(f"target_ref={target_ref}")
    print(f"request_file={output_dir / 'request.json'}")
    return 0


def _skip_bodies(args: argparse.Namespace) -> tuple[str, str | None]:
    pr_lines = [
        f"## {args.heading}",
        "",
        f"- Checks run: [#{args.checks_run_id}]({args.checks_url})",
        f"- Reason: {args.reason}",
        f"- Repair workflow: [#{args.run_id}]({args.run_url})",
    ]

    issue_body = None
    if args.linked_issue_number and (args.needs_human or not args.issue_only_if_needs_human):
        issue_lines = [
            f"{args.heading} for linked PR #{args.pr_number}.",
            "",
            f"- Checks run: [#{args.checks_run_id}]({args.checks_url})",
            f"- Backend: `{args.backend_label or 'n/a'}`",
        ]
        if args.route:
            issue_lines.append(f"- Route: `{args.route}`")
        issue_lines.extend([
            f"- Reason: {args.reason}",
            f"- Repair workflow: [#{args.run_id}]({args.run_url})",
        ])
        issue_body = "\n".join(issue_lines)

    return "\n".join(pr_lines), issue_body


def cmd_skip_request(args: argparse.Namespace) -> int:
    output_dir = Path(args.output_dir)
    pr_body, issue_body = _skip_bodies(args)

    operations = [
        {
            "type": "comment_pr",
            "pr_number": int(args.pr_number),
            "body_file": "pr-comment.md",
        }
    ]
    _write(output_dir / "pr-comment.md", pr_body)

    if issue_body is not None:
        _write(output_dir / "issue-comment.md", issue_body)
        operations.append(
            {
                "type": "comment_issue",
                "issue_number": int(args.linked_issue_number),
                "body_file": "issue-comment.md",
            }
        )
        operations.append(
            {
                "type": "edit_issue_labels",
                "issue_number": int(args.linked_issue_number),
                "add_labels": ["state:blocked"],
                "remove_labels": ["state:pr-open", "state:dispatched", "state:backlog"],
                "ignore_failure": True,
            }
        )

    return _dump_request(
        output_dir,
        target_sha=args.target_sha,
        target_ref=args.target_ref,
        operations=operations,
    )


def cmd_attempt_request(args: argparse.Namespace) -> int:
    output_dir = Path(args.output_dir)
    prompt = Path(args.prompt_file).read_text()
    body = "\n".join(
        [
            "## Auto-repair Started",
            "",
            f"- Checks run: [#{args.checks_run_id}]({args.checks_url})",
            f"- Route: `{args.route}`",
            f"- Backend: `{args.backend_label}`",
            f"- Model: `{args.model_label}`",
            f"- Reason: {args.reason}",
            f"- Repair workflow: [#{args.run_id}]({args.run_url})",
            "",
            "<details>",
            f"<summary>Task prompt ({args.tokens} tokens)</summary>",
            "",
            prompt,
            "",
            "</details>",
            "",
            f"<!-- nitrocop-auto-repair: phase=started head_sha={args.head_sha} backend={args.backend} checks_run_id={args.checks_run_id} -->",
        ]
    )
    _write(output_dir / "pr-comment.md", body)
    return _dump_request(
        output_dir,
        target_sha=args.target_sha,
        target_ref=args.target_ref,
        operations=[
            {
                "type": "comment_pr",
                "pr_number": int(args.pr_number),
                "body_file": "pr-comment.md",
            }
        ],
    )


def _tail(path: str, lines: int = 40) -> str:
    if not path:
        return "(no verification log)"
    file_path = Path(path)
    if not file_path.is_file():
        return "(no verification log)"
    content = file_path.read_text().splitlines()
    return "\n".join(content[-lines:]) or "(no verification log)"


def _scope_report(path: str) -> str:
    if not path:
        return ""
    file_path = Path(path)
    if not file_path.is_file():
        return ""
    return file_path.read_text().rstrip()


def _write_patch(repo_root: Path, base_sha: str, output_path: Path) -> None:
    result = subprocess.run(
        ["git", "diff", "--binary", f"{base_sha}...HEAD"],
        cwd=str(repo_root),
        text=True,
        capture_output=True,
        check=True,
    )
    output_path.write_text(result.stdout)


def cmd_result_request(args: argparse.Namespace) -> int:
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    operations: list[dict] = []
    result = args.result

    if result == "pushable":
        patch_path = output_dir / "repair.patch"
        _write_patch(Path(args.repo_root), args.base_sha, patch_path)
        operations.append(
            {
                "type": "push_patch",
                "patch_file": patch_path.name,
                "commit_message": f"Repair PR #{args.pr_number}: auto-repair ({args.backend_name})",
                "promote_message": f"Repair PR #{args.pr_number}: auto-repair ({args.backend_name})",
            }
        )
        body = "\n".join(
            [
                "## Auto-repair Succeeded",
                "",
                f"- Checks run: [#{args.checks_run_id}]({args.checks_url})",
                f"- Backend: `{args.backend_label}`",
                f"- Model: `{args.model_label}`",
                f"- Repair workflow: [#{args.run_id}]({args.run_url})",
                "- Repair commit: `{{SIGNED_SHA}}`",
                "",
                f"Auto-repair succeeded with backend `{args.backend_label}`.",
                "",
                "Validated locally before push using `REPAIR_VERIFY_SCRIPT`.",
                "",
                f"<!-- nitrocop-auto-repair: phase=pushed head_sha={args.target_sha} backend={args.backend_name} checks_run_id={args.checks_run_id} repair_commit={{{{SIGNED_SHA}}}} -->",
            ]
        )
    elif result == "no_changes":
        body = "\n".join(
            [
                "## Auto-repair Did Not Produce a Fix",
                "",
                f"- Checks run: [#{args.checks_run_id}]({args.checks_url})",
                f"- Backend: `{args.backend_label}`",
                f"- Repair workflow: [#{args.run_id}]({args.run_url})",
                "",
                "The repair attempt completed local verification commands but produced no branch changes, so it was treated as unsuccessful.",
            ]
        )
    elif result == "empty_pr":
        body = "\n".join(
            [
                "## Auto-repair Rejected",
                "",
                f"- Checks run: [#{args.checks_run_id}]({args.checks_url})",
                f"- Backend: `{args.backend_label}`",
                f"- Repair workflow: [#{args.run_id}]({args.run_url})",
                "",
                "The repair would have reduced the PR to an empty diff against `origin/main`, which this workflow treats as an invalid blanket revert.",
            ]
        )
    elif result == "file_guard_failed":
        report = _scope_report(args.scope_report_file)
        body = "\n".join(
            [
                "## Auto-repair Rejected",
                "",
                f"- Checks run: [#{args.checks_run_id}]({args.checks_url})",
                f"- Backend: `{args.backend_label}`",
                f"- Repair workflow: [#{args.run_id}]({args.run_url})",
                "",
                f"The workflow rejected this repair because it edited files outside the allowed scope for route `{args.guard_profile}`.",
                "",
                report,
            ]
        ).rstrip()
    elif result == "agent_failed":
        body = "\n".join(
            [
                "## Auto-repair Agent Failed",
                "",
                f"- Checks run: [#{args.checks_run_id}]({args.checks_url})",
                f"- Backend: `{args.backend_label}`",
                f"- Repair workflow: [#{args.run_id}]({args.run_url})",
                "",
                "The repair agent step failed before local verification ran.",
            ]
        )
    elif result == "verify_not_run":
        body = "\n".join(
            [
                "## Auto-repair Verification Did Not Run",
                "",
                f"- Checks run: [#{args.checks_run_id}]({args.checks_url})",
                f"- Backend: `{args.backend_label}`",
                f"- Repair workflow: [#{args.run_id}]({args.run_url})",
                "",
                "The workflow reached assessment without a verification result, so no verification log was produced.",
            ]
        )
    else:
        verify_tail = _tail(args.verify_log)
        body = "\n".join(
            [
                "## Auto-repair Failed Verification",
                "",
                f"- Checks run: [#{args.checks_run_id}]({args.checks_url})",
                f"- Backend: `{args.backend_label}`",
                f"- Repair workflow: [#{args.run_id}]({args.run_url})",
                "",
                f"Auto-repair failed local verification with backend `{args.backend_label}`.",
                "",
                f"Reason: {args.reason}",
                "",
                "<details>",
                "<summary>Verification tail</summary>",
                "",
                "```",
                verify_tail,
                "```",
                "</details>",
            ]
        )

    _write(output_dir / "pr-comment.md", body)
    operations.append(
        {
            "type": "comment_pr",
            "pr_number": int(args.pr_number),
            "body_file": "pr-comment.md",
        }
    )

    if args.linked_issue_number and result != "pushable":
        operations.append(
            {
                "type": "comment_issue",
                "issue_number": int(args.linked_issue_number),
                "body_file": "pr-comment.md",
            }
        )
        operations.append(
            {
                "type": "edit_issue_labels",
                "issue_number": int(args.linked_issue_number),
                "add_labels": ["state:blocked"],
                "remove_labels": ["state:pr-open", "state:dispatched", "state:backlog"],
                "ignore_failure": True,
            }
        )

    return _dump_request(
        output_dir,
        target_sha=args.target_sha,
        target_ref=args.target_ref,
        operations=operations,
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    skip_request = subparsers.add_parser("skip-request")
    skip_request.add_argument("--output-dir", required=True)
    skip_request.add_argument("--pr-number", required=True)
    skip_request.add_argument("--linked-issue-number", default="")
    skip_request.add_argument("--heading", required=True)
    skip_request.add_argument("--reason", required=True)
    skip_request.add_argument("--checks-run-id", required=True)
    skip_request.add_argument("--checks-url", required=True)
    skip_request.add_argument("--backend-label", default="")
    skip_request.add_argument("--route", default="")
    skip_request.add_argument("--run-id", required=True)
    skip_request.add_argument("--run-url", required=True)
    skip_request.add_argument("--target-sha", required=True)
    skip_request.add_argument("--target-ref", required=True)
    skip_request.add_argument("--needs-human", action="store_true")
    skip_request.add_argument("--issue-only-if-needs-human", action="store_true")
    skip_request.set_defaults(func=cmd_skip_request)

    attempt_request = subparsers.add_parser("attempt-request")
    attempt_request.add_argument("--output-dir", required=True)
    attempt_request.add_argument("--pr-number", required=True)
    attempt_request.add_argument("--checks-run-id", required=True)
    attempt_request.add_argument("--checks-url", required=True)
    attempt_request.add_argument("--route", required=True)
    attempt_request.add_argument("--backend-label", required=True)
    attempt_request.add_argument("--model-label", required=True)
    attempt_request.add_argument("--reason", required=True)
    attempt_request.add_argument("--run-id", required=True)
    attempt_request.add_argument("--run-url", required=True)
    attempt_request.add_argument("--head-sha", required=True)
    attempt_request.add_argument("--backend", required=True)
    attempt_request.add_argument("--prompt-file", required=True)
    attempt_request.add_argument("--tokens", required=True)
    attempt_request.add_argument("--target-sha", required=True)
    attempt_request.add_argument("--target-ref", required=True)
    attempt_request.set_defaults(func=cmd_attempt_request)

    result_request = subparsers.add_parser("result-request")
    result_request.add_argument("--output-dir", required=True)
    result_request.add_argument("--repo-root", required=True)
    result_request.add_argument("--result", required=True)
    result_request.add_argument("--pr-number", required=True)
    result_request.add_argument("--linked-issue-number", default="")
    result_request.add_argument("--checks-run-id", required=True)
    result_request.add_argument("--checks-url", required=True)
    result_request.add_argument("--backend-label", required=True)
    result_request.add_argument("--model-label", default="")
    result_request.add_argument("--backend-name", required=True)
    result_request.add_argument("--run-id", required=True)
    result_request.add_argument("--run-url", required=True)
    result_request.add_argument("--reason", default="")
    result_request.add_argument("--guard-profile", default="")
    result_request.add_argument("--scope-report-file", default="")
    result_request.add_argument("--verify-log", default="")
    result_request.add_argument("--base-sha", required=True)
    result_request.add_argument("--target-sha", required=True)
    result_request.add_argument("--target-ref", required=True)
    result_request.set_defaults(func=cmd_result_request)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
