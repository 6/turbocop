#!/usr/bin/env python3
"""Route GitHub App bot triggers into nitrocop workflows."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from dataclasses import dataclass

CHECKS_WORKFLOW_FILE = "checks.yml"
FAILED_CONCLUSIONS = {"failure", "cancelled", "timed_out", "action_required", "startup_failure"}
COP_TRACKER_RE = re.compile(r"<!--\s*nitrocop-cop-tracker:\s*(.*?)\s*-->")
ISSUE_TITLE_PREFIX = "[cop] "
MENTION_TRIGGER = "mention"
ASSIGNMENT_TRIGGER = "assignment"


def _run(cmd: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, text=True, capture_output=True, check=True)


def _gh(*args: str) -> str:
    return _run(["gh", *args]).stdout


def _output(key: str, value: str) -> None:
    print(f"{key}={value}")


def _output_multiline(key: str, value: str) -> None:
    delim = "MULTILINE_EOF_9b37"
    print(f"{key}<<{delim}")
    print(value, end="" if value.endswith("\n") else "\n")
    print(delim)


def _sanitize_output(value: object) -> str:
    return str(value).replace("\n", " ").replace("\r", " ").strip()


@dataclass(frozen=True)
class RoutedCommand:
    action: str
    trigger_kind: str
    subject_kind: str
    issue_number: int
    pr_number: int | None
    requested_by: str
    requested_by_association: str
    request_url: str
    prompt_text: str
    trigger_summary: str
    reason: str = ""


def parse_marker_fields(body: str, pattern: re.Pattern[str]) -> dict[str, str]:
    match = pattern.search(body or "")
    if not match:
        return {}
    fields: dict[str, str] = {}
    for token in match.group(1).split():
        if "=" not in token:
            continue
        key, value = token.split("=", 1)
        fields[key.strip()] = value.strip()
    return fields


def extract_cop_from_issue(issue: dict) -> str | None:
    body_fields = parse_marker_fields(str(issue.get("body", "")), COP_TRACKER_RE)
    cop = body_fields.get("cop")
    if cop:
        return cop
    title = str(issue.get("title", "")).strip()
    if title.startswith(ISSUE_TITLE_PREFIX):
        return title[len(ISSUE_TITLE_PREFIX) :].strip()
    return None


def build_issue_assignment_prompt(issue_title: str, issue_body: str) -> str:
    cleaned_body = COP_TRACKER_RE.sub("", issue_body or "").strip()
    parts: list[str] = []

    title = issue_title.strip()
    if title:
        parts.append("## Tracker Issue Title")
        parts.append(title)

    if cleaned_body:
        if parts:
            parts.append("")
        parts.append("## Tracker Issue Body")
        parts.append(cleaned_body)

    return "\n".join(parts).strip()


def route_payload(payload: dict, *, repo: str) -> RoutedCommand:
    source_repo = str(payload.get("source_repo", "")).strip()
    if not source_repo:
        raise ValueError("payload.source_repo is required")
    if source_repo != repo:
        raise ValueError(f"payload.source_repo {source_repo} does not match {repo}")

    trigger_kind = str(payload.get("trigger_kind", "")).strip()
    subject_kind = str(payload.get("subject_kind", "")).strip()
    issue_number = payload.get("issue_number")
    pr_number = payload.get("pr_number")
    requested_by = str(payload.get("requested_by", "")).strip()
    association = str(payload.get("requested_by_association", "")).strip()
    request_url = str(payload.get("request_url", "")).strip()
    prompt_text = str(payload.get("prompt_text", ""))
    issue_title = str(payload.get("issue_title", "")).strip()
    issue_body = str(payload.get("issue_body", ""))

    if trigger_kind not in {MENTION_TRIGGER, ASSIGNMENT_TRIGGER}:
        raise ValueError("payload.trigger_kind must be mention or assignment")
    if subject_kind not in {"issue", "pull_request"}:
        raise ValueError("payload.subject_kind must be issue or pull_request")
    if not isinstance(issue_number, int):
        raise ValueError("payload.issue_number must be an integer")
    if pr_number is not None and not isinstance(pr_number, int):
        raise ValueError("payload.pr_number must be an integer when present")
    if not requested_by:
        raise ValueError("payload.requested_by is required")

    if trigger_kind == ASSIGNMENT_TRIGGER:
        if subject_kind != "issue":
            return RoutedCommand(
                action="comment_only",
                trigger_kind=trigger_kind,
                subject_kind=subject_kind,
                issue_number=issue_number,
                pr_number=pr_number,
                requested_by=requested_by,
                requested_by_association=association,
                request_url=request_url,
                prompt_text="",
                trigger_summary="issue assignment to @6",
                reason="Assigning @6 only works on cop tracker issues.",
            )
        return RoutedCommand(
            action="fix_issue",
            trigger_kind=trigger_kind,
            subject_kind="issue",
            issue_number=issue_number,
            pr_number=None,
            requested_by=requested_by,
            requested_by_association=association,
            request_url=request_url,
            prompt_text=build_issue_assignment_prompt(issue_title, issue_body),
            trigger_summary="issue assignment to @6",
        )

    if subject_kind == "pull_request":
        if pr_number is None:
            return RoutedCommand(
                action="comment_only",
                trigger_kind=trigger_kind,
                subject_kind=subject_kind,
                issue_number=issue_number,
                pr_number=pr_number,
                requested_by=requested_by,
                requested_by_association=association,
                request_url=request_url,
                prompt_text=prompt_text,
                trigger_summary="@6 mention",
                reason="The @6 trigger only works on pull request comments or issue tracker items.",
            )
        return RoutedCommand(
            action="repair_pr",
            trigger_kind=trigger_kind,
            subject_kind="pull_request",
            issue_number=issue_number,
            pr_number=pr_number,
            requested_by=requested_by,
            requested_by_association=association,
            request_url=request_url,
            prompt_text=prompt_text.strip(),
            trigger_summary="@6 mention",
        )

    return RoutedCommand(
        action="fix_issue",
        trigger_kind=trigger_kind,
        subject_kind="issue",
        issue_number=issue_number,
        pr_number=None,
        requested_by=requested_by,
        requested_by_association=association,
        request_url=request_url,
        prompt_text=prompt_text.strip(),
        trigger_summary="@6 mention",
    )


def choose_failed_checks_run(runs: list[dict], *, head_sha: str) -> tuple[dict | None, str]:
    matching = [run for run in runs if str(run.get("head_sha", "")).strip() == head_sha]
    if not matching:
        return None, "No Checks workflow run was found for the current PR head"

    for run in matching:
        if run.get("status") != "completed":
            return None, "Checks is still running for the current PR head"

    for run in matching:
        if run.get("conclusion") in FAILED_CONCLUSIONS:
            return run, ""

    return None, "Checks is not currently failing for the current PR head"


def load_pr(repo: str, pr_number: int) -> dict:
    return json.loads(
        _gh(
            "pr",
            "view",
            str(pr_number),
            "--repo",
            repo,
            "--json",
            "number,state,url,headRefName,headRefOid",
        )
    )


def load_checks_runs(repo: str, branch: str) -> list[dict]:
    response = json.loads(
        _gh(
            "api",
            f"repos/{repo}/actions/workflows/{CHECKS_WORKFLOW_FILE}/runs",
            "-f",
            f"branch={branch}",
            "-f",
            "event=pull_request",
            "-f",
            "per_page=20",
        )
    )
    workflow_runs = response.get("workflow_runs", [])
    if not isinstance(workflow_runs, list):
        raise ValueError("workflow_runs must be a list")
    return workflow_runs


def cmd_route(args: argparse.Namespace) -> int:
    payload = json.loads(args.payload_json)
    routed = route_payload(payload, repo=args.repo)

    single_line_outputs = {
        "action": routed.action,
        "trigger_kind": routed.trigger_kind,
        "subject_kind": routed.subject_kind,
        "issue_number": str(routed.issue_number),
        "pr_number": str(routed.pr_number or ""),
        "requested_by": routed.requested_by,
        "requested_by_association": routed.requested_by_association,
        "request_url": routed.request_url,
        "trigger_summary": routed.trigger_summary,
        "reason": routed.reason,
    }
    for key, value in single_line_outputs.items():
        _output(key, _sanitize_output(value))
    _output_multiline("prompt_text", routed.prompt_text)
    return 0


def cmd_resolve_repair(args: argparse.Namespace) -> int:
    pr = load_pr(args.repo, args.pr_number)
    if pr.get("state") != "OPEN":
        _output("should_dispatch", "false")
        _output("reason", "PR is not open")
        return 0

    head_branch = str(pr.get("headRefName", "")).strip()
    head_sha = str(pr.get("headRefOid", "")).strip()
    if not head_branch or not head_sha:
        raise ValueError("PR head branch/SHA is missing")

    runs = load_checks_runs(args.repo, head_branch)
    selected, reason = choose_failed_checks_run(runs, head_sha=head_sha)
    if selected is None:
        _output("should_dispatch", "false")
        _output("reason", _sanitize_output(reason))
        _output("head_branch", head_branch)
        _output("head_sha", head_sha)
        return 0

    _output("should_dispatch", "true")
    _output("checks_run_id", str(selected["id"]))
    _output("checks_url", str(selected.get("html_url", "")))
    _output("head_branch", head_branch)
    _output("head_sha", head_sha)
    return 0


def cmd_resolve_fix(args: argparse.Namespace) -> int:
    issue = json.loads(
        _gh(
            "issue",
            "view",
            str(args.issue_number),
            "--repo",
            args.repo,
            "--json",
            "number,state,title,body,labels,url",
        )
    )
    if issue.get("state") != "OPEN":
        _output("should_dispatch", "false")
        _output("reason", "Issue is not open")
        return 0

    labels = {label["name"] for label in issue.get("labels", [])}
    if "type:cop-issue" not in labels:
        _output("should_dispatch", "false")
        _output("reason", "Issue is not a cop tracker issue")
        return 0

    cop = extract_cop_from_issue(issue)
    if not cop:
        _output("should_dispatch", "false")
        _output("reason", "Could not determine the cop from the tracker issue")
        return 0

    _output("should_dispatch", "true")
    _output("cop", cop)
    _output("issue_url", str(issue.get("url", "")))
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    route = subparsers.add_parser("route")
    route.add_argument("--payload-json", required=True)
    route.add_argument("--repo", required=True)
    route.set_defaults(func=cmd_route)

    resolve_repair = subparsers.add_parser("resolve-repair")
    resolve_repair.add_argument("--repo", required=True)
    resolve_repair.add_argument("--pr-number", type=int, required=True)
    resolve_repair.set_defaults(func=cmd_resolve_repair)

    resolve_fix = subparsers.add_parser("resolve-fix")
    resolve_fix.add_argument("--repo", required=True)
    resolve_fix.add_argument("--issue-number", type=int, required=True)
    resolve_fix.set_defaults(func=cmd_resolve_fix)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
