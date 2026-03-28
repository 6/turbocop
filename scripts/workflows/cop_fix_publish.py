#!/usr/bin/env python3
"""Build remote repo-write requests for agent-cop-fix publish phases."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path

from cop_fix_lifecycle import _build_claim_body, _build_task_body


def _write(path: Path, content: str) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)
    return path


def _dump_request(output_dir: Path, *, operations: list[dict], match_mode: str = "contained") -> int:
    output_dir.mkdir(parents=True, exist_ok=True)
    request = {
        "match_mode": match_mode,
        "operations": operations,
    }
    request_file = output_dir / "request.json"
    _write(request_file, json.dumps(request, indent=2) + "\n")
    print(f"request_file={request_file}")
    return 0


def _write_patch(repo_root: Path, base_sha: str, output_path: Path) -> None:
    result = subprocess.run(
        ["git", "diff", "--binary", f"{base_sha}...HEAD"],
        cwd=str(repo_root),
        text=True,
        capture_output=True,
        check=True,
    )
    output_path.write_text(result.stdout)


def cmd_cleanup_request(args: argparse.Namespace) -> int:
    output_dir = Path(args.output_dir)
    operations: list[dict] = []

    if args.pr:
        operations.append(
            {
                "type": "close_pr",
                "pr": args.pr,
                "comment": f"Agent failed. See run: {args.run_url}",
                "delete_branch": True,
            }
        )

    if args.issue_number:
        if args.pr:
            body = (
                f"Agent fix failed before producing a usable PR for `{args.cop}`.\n\n"
                f"- Backend: `{args.backend_label}`\n"
                f"- Model: `{args.model_label}`\n"
                f"- Mode: `{args.mode}`\n"
                f"- Run: {args.run_url}\n\n"
                "See the failed workflow run for details.\n"
            )
        else:
            body = (
                f"Agent fix failed before it could create a draft PR for `{args.cop}`.\n\n"
                f"- Backend input: `{args.backend_label}`\n"
                f"- Mode: `{args.mode}`\n"
                f"- Run: {args.run_url}\n\n"
                "Review the failed workflow run for details.\n"
            )

        _write(output_dir / "issue-comment.md", body)
        operations.append(
            {
                "type": "comment_issue",
                "issue_number": int(args.issue_number),
                "body_file": "issue-comment.md",
            }
        )

        if args.pr:
            operations.append(
                {
                    "type": "edit_issue_labels",
                    "issue_number": int(args.issue_number),
                    "remove_labels": ["state:pr-open", "state:dispatched"],
                    "add_labels": ["state:backlog"],
                    "ignore_failure": True,
                }
            )

    return _dump_request(output_dir, operations=operations)


def cmd_skip_fixed_request(args: argparse.Namespace) -> int:
    output_dir = Path(args.output_dir)
    if not args.issue_number:
        return _dump_request(output_dir, operations=[], match_mode="contained")

    body = (
        f"No fix PR was created for `{args.cop}`.\n\n"
        "Pre-diagnostic found no reproducible code bugs in the current "
        "corpus examples, so the workflow skipped agent execution.\n\n"
        f"- Backend input: `{args.backend_input}`\n"
        f"- Mode: `{args.mode}`\n"
        f"- Run: {args.run_url}\n"
    )
    _write(output_dir / "issue-comment.md", body)
    return _dump_request(
        output_dir,
        operations=[
            {
                "type": "comment_issue",
                "issue_number": int(args.issue_number),
                "body_file": "issue-comment.md",
            }
        ],
        match_mode="contained",
    )


def cmd_claim_request(args: argparse.Namespace) -> int:
    output_dir = Path(args.output_dir)
    model_label_name = f"model:{args.backend}"
    retry_note = " (retry)" if args.mode == "retry" else ""

    claim_body = _build_claim_body(
        args.cop,
        args.mode,
        args.backend_label,
        args.model_label,
        args.backend_reason,
        args.run_url,
        args.issue_number,
    )
    task_body = _build_task_body(
        args.cop,
        args.mode,
        args.backend_label,
        args.model_label,
        args.run_url,
        args.issue_number,
        args.code_bugs,
        args.tokens,
        Path(args.task_file).read_text(),
    )
    _write(output_dir / "claim-body.md", claim_body)
    _write(output_dir / "task-body.md", task_body)

    operations: list[dict] = [
        {
            "type": "ensure_labels",
            "labels": [
                {"name": "type:cop-fix", "color": "0e8a16"},
                {"name": model_label_name, "color": "c2e0c6"},
                {"name": "state:backlog", "color": "fbca04"},
                {"name": "state:dispatched", "color": "1d76db"},
                {"name": "state:pr-open", "color": "0e8a16"},
                {"name": "state:blocked", "color": "b60205"},
            ],
        },
        {
            "type": "create_branch",
            "branch": args.branch,
            "commit_message": f"[bot] Fix {args.cop}: in progress",
            "promote_message": f"[bot] Fix {args.cop}: in progress",
        },
        {
            "type": "create_pr",
            "base": "main",
            "head": args.branch,
            "title": f"[bot] Fix {args.cop}{retry_note}",
            "draft": True,
            "labels": ["type:cop-fix", model_label_name],
            "body_file": "claim-body.md",
        },
    ]

    if args.issue_number:
        operations.append(
            {
                "type": "edit_issue_labels",
                "issue_number": int(args.issue_number),
                "remove_labels": ["state:backlog", "state:dispatched", "state:blocked"],
                "add_labels": ["state:pr-open"],
                "ignore_failure": True,
            }
        )

    operations.append(
        {
            "type": "edit_pr",
            "pr": "{{PR_URL}}",
            "body_file": "task-body.md",
        }
    )

    return _dump_request(output_dir, operations=operations, match_mode="current_head")


def cmd_finalize_request(args: argparse.Namespace) -> int:
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    operations: list[dict] = []

    if args.result in {"success", "docs_only"}:
        patch_path = output_dir / "cop-fix.patch"
        _write_patch(Path(args.repo_root), args.base_sha, patch_path)
        operations.append(
            {
                "type": "push_patch",
                "patch_file": patch_path.name,
                "commit_message": f"Fix {args.cop}: agent-generated fix{' (retry)' if args.mode == 'retry' else ''} ({args.backend})",
                "promote_message": f"Fix {args.cop}: agent-generated fix{' (retry)' if args.mode == 'retry' else ''} ({args.backend})",
            }
        )
        operations.extend(
            [
                {
                    "type": "edit_pr",
                    "pr": args.pr_url,
                    "body_file": "pr-body.md",
                },
                {
                    "type": "ready_pr",
                    "pr": args.pr_url,
                },
                {
                    "type": "merge_pr",
                    "pr": args.pr_url,
                    "auto": True,
                    "squash": True,
                    "delete_branch": True,
                },
            ]
        )
        _write(output_dir / "pr-body.md", Path(args.pr_body_file).read_text())

        if args.result == "docs_only" and args.issue_number:
            issue_body = (
                f"Agent investigated `{args.cop}` and documented findings, "
                "but no cop logic was changed.\n\n"
                f"- Backend: `{args.backend_label}`\n"
                f"- Model: `{args.model_label}`\n"
                f"- Mode: `{args.mode}`\n"
                f"- Run: {args.run_url}\n\n"
                "The FP/FN gap is likely caused by file-discovery or config differences, "
                "not a cop detection bug. Documentation PR was merged. "
                "Marking as blocked for manual investigation.\n"
            )
            _write(output_dir / "docs-only-issue.md", issue_body)
            operations.extend(
                [
                    {
                        "type": "comment_issue",
                        "issue_number": int(args.issue_number),
                        "body_file": "docs-only-issue.md",
                    },
                    {
                        "type": "edit_issue_labels",
                        "issue_number": int(args.issue_number),
                        "remove_labels": ["state:pr-open", "state:dispatched", "state:backlog"],
                        "add_labels": ["state:blocked"],
                        "ignore_failure": True,
                    },
                ]
            )
    elif args.result == "no_changes":
        operations.append(
            {
                "type": "close_pr",
                "pr": args.pr_url,
                "comment": "Agent produced no changes.",
                "delete_branch": True,
            }
        )
        if args.issue_number:
            body = (
                f"No fix PR was produced for `{args.cop}`.\n\n"
                f"- Backend: `{args.backend_label}`\n"
                f"- Model: `{args.model_label}`\n"
                f"- Mode: `{args.mode}`\n"
                f"- Run: {args.run_url}\n\n"
                "The agent did not produce any branch changes.\n"
            )
            _write(output_dir / "issue-comment.md", body)
            operations.extend(
                [
                    {
                        "type": "comment_issue",
                        "issue_number": int(args.issue_number),
                        "body_file": "issue-comment.md",
                    },
                    {
                        "type": "edit_issue_labels",
                        "issue_number": int(args.issue_number),
                        "remove_labels": ["state:pr-open", "state:dispatched"],
                        "add_labels": ["state:backlog"],
                        "ignore_failure": True,
                    },
                ]
            )
    elif args.result == "rejected":
        scope_report = Path(args.scope_report_file).read_text().rstrip() if args.scope_report_file else ""
        body = (
            "## Agent Fix Rejected\n\n"
            "The workflow rejected this attempt because it edited files outside "
            "the allowed scope for `agent-cop-fix`.\n\n"
            f"{scope_report}\n"
            f"- Run: {args.run_url}\n"
        )
        _write(output_dir / "rejected.md", body)
        operations.extend(
            [
                {
                    "type": "comment_pr",
                    "pr_number": int(args.pr_number),
                    "body_file": "rejected.md",
                },
                {
                    "type": "close_pr",
                    "pr": args.pr_url,
                    "comment": "Agent edited files outside the allowed scope.",
                    "delete_branch": True,
                },
            ]
        )
        if args.issue_number:
            operations.extend(
                [
                    {
                        "type": "comment_issue",
                        "issue_number": int(args.issue_number),
                        "body_file": "rejected.md",
                    },
                    {
                        "type": "edit_issue_labels",
                        "issue_number": int(args.issue_number),
                        "remove_labels": ["state:pr-open", "state:dispatched", "state:blocked"],
                        "add_labels": ["state:backlog"],
                        "ignore_failure": True,
                    },
                ]
            )
    else:
        raise SystemExit(f"unsupported finalize result: {args.result}")

    return _dump_request(output_dir, operations=operations, match_mode="current_head")


def cmd_reset_issue_request(args: argparse.Namespace) -> int:
    output_dir = Path(args.output_dir)
    return _dump_request(
        output_dir,
        operations=[
            {
                "type": "edit_issue_labels",
                "issue_number": int(args.issue_number),
                "remove_labels": ["state:pr-open", "state:dispatched"],
                "add_labels": ["state:backlog"],
                "ignore_failure": True,
            }
        ],
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    cleanup_request = subparsers.add_parser("cleanup-request")
    cleanup_request.add_argument("--output-dir", required=True)
    cleanup_request.add_argument("--cop", required=True)
    cleanup_request.add_argument("--pr", default="")
    cleanup_request.add_argument("--issue-number", default="")
    cleanup_request.add_argument("--backend-label", required=True)
    cleanup_request.add_argument("--model-label", default="n/a")
    cleanup_request.add_argument("--mode", required=True)
    cleanup_request.add_argument("--run-url", required=True)
    cleanup_request.set_defaults(func=cmd_cleanup_request)

    skip_fixed_request = subparsers.add_parser("skip-fixed-request")
    skip_fixed_request.add_argument("--output-dir", required=True)
    skip_fixed_request.add_argument("--cop", required=True)
    skip_fixed_request.add_argument("--issue-number", default="")
    skip_fixed_request.add_argument("--backend-input", default="")
    skip_fixed_request.add_argument("--mode", default="fix")
    skip_fixed_request.add_argument("--run-url", required=True)
    skip_fixed_request.set_defaults(func=cmd_skip_fixed_request)

    claim_request = subparsers.add_parser("claim-request")
    claim_request.add_argument("--output-dir", required=True)
    claim_request.add_argument("--cop", required=True)
    claim_request.add_argument("--mode", required=True)
    claim_request.add_argument("--branch", required=True)
    claim_request.add_argument("--backend", required=True)
    claim_request.add_argument("--backend-label", required=True)
    claim_request.add_argument("--model-label", required=True)
    claim_request.add_argument("--backend-reason", required=True)
    claim_request.add_argument("--run-url", required=True)
    claim_request.add_argument("--issue-number", default="")
    claim_request.add_argument("--code-bugs", required=True)
    claim_request.add_argument("--tokens", required=True)
    claim_request.add_argument("--task-file", required=True)
    claim_request.set_defaults(func=cmd_claim_request)

    finalize_request = subparsers.add_parser("finalize-request")
    finalize_request.add_argument("--output-dir", required=True)
    finalize_request.add_argument("--repo-root", required=True)
    finalize_request.add_argument("--result", required=True)
    finalize_request.add_argument("--cop", required=True)
    finalize_request.add_argument("--backend", required=True)
    finalize_request.add_argument("--backend-label", required=True)
    finalize_request.add_argument("--model-label", required=True)
    finalize_request.add_argument("--mode", required=True)
    finalize_request.add_argument("--issue-number", default="")
    finalize_request.add_argument("--run-url", required=True)
    finalize_request.add_argument("--base-sha", required=True)
    finalize_request.add_argument("--pr-url", required=True)
    finalize_request.add_argument("--pr-number", required=True)
    finalize_request.add_argument("--pr-body-file", required=True)
    finalize_request.add_argument("--scope-report-file", default="")
    finalize_request.set_defaults(func=cmd_finalize_request)

    reset_issue_request = subparsers.add_parser("reset-issue-request")
    reset_issue_request.add_argument("--output-dir", required=True)
    reset_issue_request.add_argument("--issue-number", required=True)
    reset_issue_request.set_defaults(func=cmd_reset_issue_request)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
