#!/usr/bin/env python3
"""Build remote repo-write requests for agent-cop-fix publish phases."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


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

    reset_issue_request = subparsers.add_parser("reset-issue-request")
    reset_issue_request.add_argument("--output-dir", required=True)
    reset_issue_request.add_argument("--issue-number", required=True)
    reset_issue_request.set_defaults(func=cmd_reset_issue_request)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
