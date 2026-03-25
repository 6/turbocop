#!/usr/bin/env python3
"""Bounded retry policy helpers for agent-pr-repair."""

from __future__ import annotations

import argparse
import json
import re

MARKER_RE = re.compile(r"<!--\s*nitrocop-auto-repair:\s*(.*?)\s*-->")
PR_ISSUE_RE = re.compile(r"<!--\s*nitrocop-cop-issue:\s*(.*?)\s*-->")


def parse_marker_fields(body: str) -> list[dict[str, str]]:
    markers: list[dict[str, str]] = []
    for payload in MARKER_RE.findall(body):
        fields: dict[str, str] = {}
        for token in payload.split():
            if "=" not in token:
                continue
            key, value = token.split("=", 1)
            fields[key.strip()] = value.strip()
        if fields:
            markers.append(fields)
    return markers


def parse_single_marker(body: str, pattern: re.Pattern[str]) -> dict[str, str]:
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


def parse_linked_issue(body: str) -> tuple[int | None, str]:
    fields = parse_single_marker(body, PR_ISSUE_RE)
    issue_number = fields.get("number", "")
    cop = fields.get("cop", "")
    if issue_number.isdigit():
        return int(issue_number), cop
    return None, cop


def inspect_attempts(comments: list[dict], current_head_sha: str) -> dict[str, int | bool]:
    prior_pushes = 0
    prior_pr_repair_attempts = 0
    prior_attempted_current_head = False

    for comment in comments:
        body = comment.get("body") or ""
        for marker in parse_marker_fields(body):
            phase = marker.get("phase", "")
            head_sha = marker.get("head_sha", "")
            if phase == "started":
                prior_pr_repair_attempts += 1
                if current_head_sha and head_sha == current_head_sha:
                    prior_attempted_current_head = True
            elif phase == "pushed":
                prior_pushes += 1

    return {
        "prior_pushes": prior_pushes,
        "prior_pr_repair_attempts": prior_pr_repair_attempts,
        "prior_attempted_current_head": prior_attempted_current_head,
    }


def gate_pr(pr: dict, repo: str, checks_head_sha: str) -> tuple[bool, str]:
    labels = [label["name"] for label in pr.get("labels", [])]
    head_repo = (pr.get("headRepository") or {}).get("nameWithOwner", "")

    if pr.get("state") != "OPEN":
        return False, "PR is not open"
    if pr.get("baseRefName") != "main":
        return False, "PR does not target main"
    if pr.get("isCrossRepository"):
        return False, "Cross-repository PRs are not trusted for auto-repair"
    if head_repo and head_repo != repo:
        return False, f"PR head repository {head_repo} does not match {repo}"
    if "type:cop-fix" not in labels:
        return False, "PR is not labeled type:cop-fix"
    if checks_head_sha and pr.get("headRefOid") and pr["headRefOid"] != checks_head_sha:
        return False, "PR head moved after the failed Checks run"
    return True, ""


def apply_policy(
    *,
    route: str,
    force: bool,
    prior_attempted_current_head: bool,
    prior_pushes: int,
    prior_pr_repair_attempts: int,
) -> tuple[bool, str, bool]:
    if route == "skip":
        return True, "", False
    if force:
        return True, "", False
    if prior_attempted_current_head:
        return False, "This PR head has already had an automatic repair attempt", False
    if prior_pushes >= 2:
        return False, "PR already has 2 automatic repair pushes", True
    if prior_pr_repair_attempts >= 2:
        return False, "PR already has 2 automatic repair attempts", True
    return True, "", False


def cmd_pr_state(args: argparse.Namespace) -> int:
    pr = json.loads(args.pr_json)
    comments = json.loads(args.comments_json)
    should_run, reason = gate_pr(pr, args.repo, args.checks_head_sha)
    attempts = inspect_attempts(comments, pr.get("headRefOid", ""))
    linked_issue_number, linked_cop = parse_linked_issue(pr.get("body", ""))

    print(f"number={pr['number']}")
    print(f"title={pr['title']}")
    print(f"url={pr['url']}")
    print(f"head_branch={pr['headRefName']}")
    print(f"head_sha={pr['headRefOid']}")
    print(f"linked_issue_number={linked_issue_number or ''}")
    print(f"linked_cop={linked_cop}")
    print(f"should_run={'true' if should_run else 'false'}")
    print(f"skip_reason={reason}")
    print(f"prior_pushes={attempts['prior_pushes']}")
    print(f"prior_pr_repair_attempts={attempts['prior_pr_repair_attempts']}")
    print(
        "prior_attempted_current_head="
        + ("true" if attempts["prior_attempted_current_head"] else "false")
    )
    return 0


def cmd_live_gate(args: argparse.Namespace) -> int:
    pr = json.loads(args.pr_json)
    should_run, reason = gate_pr(pr, args.repo, args.checks_head_sha)
    print(f"should_continue={'true' if should_run else 'false'}")
    print(f"skip_reason={reason}")
    return 0


def cmd_policy(args: argparse.Namespace) -> int:
    should_run, reason, needs_human = apply_policy(
        route=args.route,
        force=args.force,
        prior_attempted_current_head=args.prior_attempted_current_head,
        prior_pushes=args.prior_pushes,
        prior_pr_repair_attempts=args.prior_pr_repair_attempts,
    )
    print(f"should_repair={'true' if should_run else 'false'}")
    print(f"skip_reason={reason}")
    print(f"needs_human={'true' if needs_human else 'false'}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Bounded retry policy for agent-pr-repair")
    subparsers = parser.add_subparsers(dest="command", required=True)

    pr_state = subparsers.add_parser("pr-state")
    pr_state.add_argument("--pr-json", required=True)
    pr_state.add_argument("--comments-json", required=True)
    pr_state.add_argument("--repo", required=True)
    pr_state.add_argument("--checks-head-sha", default="")
    pr_state.set_defaults(func=cmd_pr_state)

    live_gate = subparsers.add_parser("live-gate")
    live_gate.add_argument("--pr-json", required=True)
    live_gate.add_argument("--repo", required=True)
    live_gate.add_argument("--checks-head-sha", default="")
    live_gate.set_defaults(func=cmd_live_gate)

    policy = subparsers.add_parser("policy")
    policy.add_argument("--route", required=True)
    policy.add_argument("--force", action="store_true")
    policy.add_argument("--prior-attempted-current-head", action="store_true")
    policy.add_argument("--prior-pushes", type=int, default=0)
    policy.add_argument("--prior-pr-repair-attempts", type=int, default=0)
    policy.set_defaults(func=cmd_policy)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
