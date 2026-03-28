#!/usr/bin/env python3
"""Generic repo-owned task planner/executor entrypoint for 6/bot."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

import bot_command

SCRIPTS_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPTS_DIR.parent.parent
DIFF_PATHS = "src/cop/ tests/fixtures/cops/ bench/corpus/"
SETUP_PROFILE = "nitrocop"
SETUP_CONFIG = {
    "rust_components": "rustfmt",
    "install_vendor_gems": True,
    "install_python": True,
    "cargo_linker": "clang",
    "rustflags": "-C link-arg=-fuse-ld=mold",
}


def _run(
    cmd: list[str],
    *,
    cwd: Path | None = None,
    env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str]:
    command_env = os.environ.copy()
    command_env.pop("GITHUB_OUTPUT", None)
    if env:
        command_env.update(env)
    return subprocess.run(
        cmd,
        cwd=str(cwd) if cwd else None,
        env=command_env,
        text=True,
        capture_output=True,
        check=True,
    )


def _gh(*args: str) -> str:
    return _run(["gh", *args]).stdout


def _output(key: str, value: str) -> None:
    print(f"{key}={value}")


def _output_multiline(key: str, value: str) -> None:
    delim = "MULTILINE_EOF_7e41"
    print(f"{key}<<{delim}")
    print(value, end="" if value.endswith("\n") else "\n")
    print(delim)


def _load_payload(payload_json: str, *, repo: str) -> tuple[dict, bot_command.RoutedCommand]:
    payload = json.loads(payload_json)
    routed = bot_command.route_payload(payload, repo=repo)
    return payload, routed


def _load_issue(repo: str, issue_number: int) -> dict:
    return json.loads(
        _gh(
            "issue",
            "view",
            str(issue_number),
            "--repo",
            repo,
            "--json",
            "number,state,title,body,labels,url",
        )
    )


def _resolve_cop_issue(repo: str, issue_number: int) -> tuple[dict, str | None, str]:
    issue = _load_issue(repo, issue_number)
    if issue.get("state") != "OPEN":
        return issue, None, "Issue is not open"

    labels = {label["name"] for label in issue.get("labels", [])}
    if "type:cop-issue" not in labels:
        return issue, None, "Issue is not a cop tracker issue"

    cop = bot_command.extract_cop_from_issue(issue)
    if not cop:
        return issue, None, "Could not determine the cop from the tracker issue"

    return issue, cop, ""


def _build_extra_context(routed: bot_command.RoutedCommand) -> str:
    lines = [f"Requested by @{routed.requested_by} via {routed.trigger_summary}."]
    if routed.request_url:
        lines.append(f"Trigger URL: {routed.request_url}")
    if routed.prompt_text:
        lines.extend(["", routed.prompt_text.strip()])
    return "\n".join(lines)


def _run_keyval(cmd: list[str], *, cwd: Path | None = None, env: dict[str, str] | None = None) -> dict[str, str]:
    result = _run(cmd, cwd=cwd, env=env)
    values: dict[str, str] = {}
    for line in result.stdout.splitlines():
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key.strip()] = value.strip()
    return values


def _write_json(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2) + "\n")


def _read_json(path: Path) -> dict:
    return json.loads(path.read_text())


def _request_path_from_output(output: dict[str, str]) -> Path:
    request_file = output.get("request_file", "").strip()
    if not request_file:
        raise SystemExit("request builder did not emit request_file")
    return Path(request_file).resolve()


def _parse_pr_number(pr_url: str) -> str:
    return pr_url.rstrip("/").rsplit("/", 1)[-1]


def _determine_cop_mode(cop: str) -> str:
    try:
        output = _gh(
            "pr",
            "list",
            "--state",
            "all",
            "--search",
            f"{cop} in:title",
            "--json",
            "title,headRefName,mergedAt",
            "--limit",
            "20",
        )
    except subprocess.CalledProcessError:
        return "fix"

    prs = json.loads(output) if output else []
    cop_key = cop.lower().replace("/", "")
    for pr in prs:
        title_key = str(pr.get("title", "")).lower().replace("/", "").replace(" ", "")
        branch_key = str(pr.get("headRefName", "")).lower().replace("-", "").replace("_", "")
        if cop_key not in title_key and cop_key not in branch_key:
            continue
        if not pr.get("mergedAt"):
            return "retry"
    return "fix"


def cmd_route(args: argparse.Namespace) -> int:
    _, routed = _load_payload(args.payload_json, repo=args.repo)

    outputs = {
        "requested_by": routed.requested_by,
        "requested_by_association": routed.requested_by_association,
        "request_url": routed.request_url,
        "trigger_summary": routed.trigger_summary,
        "issue_number": str(routed.issue_number),
        "subject_kind": routed.subject_kind,
    }
    for key, value in outputs.items():
        _output(key, value)

    if routed.action != "fix_issue":
        _output("action", "comment_only")
        _output("reason", routed.reason or "This repo task only handles issue-driven cop fixes.")
        return 0

    _, cop, reason = _resolve_cop_issue(args.repo, routed.issue_number)
    if reason:
        _output("action", "comment_only")
        _output("reason", reason)
        return 0

    mode = _determine_cop_mode(cop or "")
    _output("action", "run_agent")
    _output("workflow", "agent-cop-fix")
    _output("setup_profile", SETUP_PROFILE)
    _output("setup_config_json", json.dumps(SETUP_CONFIG, separators=(",", ":")))
    _output("cop", cop or "")
    _output("mode", mode)
    return 0


def cmd_prepare(args: argparse.Namespace) -> int:
    _, routed = _load_payload(args.payload_json, repo=args.repo)
    if routed.action != "fix_issue":
        raise SystemExit("prepare only supports issue-driven cop fixes")

    issue, cop, reason = _resolve_cop_issue(args.repo, routed.issue_number)
    if reason:
        raise SystemExit(reason)
    if args.default_branch != "main":
        raise SystemExit("repo_task cop-fix currently requires main as the default branch")

    mode = _determine_cop_mode(cop)
    run_id = os.environ.get("GITHUB_RUN_ID", "repo-task")
    init = _run_keyval(
        [
            sys.executable,
            "scripts/workflows/cop_fix_lifecycle.py",
            "init",
            "--cop",
            cop,
            "--mode",
            mode,
            "--backend-input",
            "auto",
            "--run-id",
            run_id,
        ],
        cwd=REPO_ROOT,
    )

    _run(["cargo", "build"], cwd=REPO_ROOT)
    build_env = {"CARGO_INCREMENTAL": "1"}
    _run(["cargo", "test", "--lib", "--no-run"], cwd=REPO_ROOT, env=build_env)

    task_file = Path(os.environ["TASK_FILE"])
    _run(
        [
            sys.executable,
            "scripts/dispatch_cops.py",
            "task",
            cop,
            "--binary",
            "target/debug/nitrocop",
            "--output",
            str(task_file),
        ],
        cwd=REPO_ROOT,
    )

    task_text = task_file.read_text()
    code_bugs = task_text.count("CODE BUG")
    tokens = _run(
        [sys.executable, "scripts/workflows/count_tokens.py", str(task_file)],
        cwd=REPO_ROOT,
    ).stdout.strip()

    extra_context = _build_extra_context(routed)
    _run(
        [
            sys.executable,
            "scripts/workflows/cop_fix_lifecycle.py",
            "build-prompt",
            "--cop",
            cop,
            "--mode",
            mode,
            "--extra-context",
            extra_context,
            "--filter",
            init["filter"],
        ],
        cwd=REPO_ROOT,
    )

    state = {
        "cop": cop,
        "issue_number": str(routed.issue_number),
        "issue_url": str(issue.get("url", "")),
        "branch": init["branch"],
        "filter": init["filter"],
        "mode": mode,
        "backend_input": "auto",
        "tokens": tokens,
        "default_branch": args.default_branch,
        "default_branch_sha": args.default_branch_sha,
        "trigger_summary": routed.trigger_summary,
        "requested_by": routed.requested_by,
        "request_url": routed.request_url,
        "prompt_text": routed.prompt_text,
        "extra_context": extra_context,
    }

    request_dir = args.state_file.parent / "repo-task-request"
    if code_bugs == 0:
        skip = _run_keyval(
            [
                sys.executable,
                "scripts/workflows/cop_fix_publish.py",
                "skip-fixed-request",
                "--output-dir",
                str(request_dir),
                "--cop",
                cop,
                "--issue-number",
                str(routed.issue_number),
                "--run-url",
                f"{os.environ['GITHUB_SERVER_URL']}/{os.environ['GITHUB_REPOSITORY']}/actions/runs/{os.environ['GITHUB_RUN_ID']}",
                "--backend-input",
                "auto",
                "--mode",
                mode,
            ],
            cwd=REPO_ROOT,
        )
        state["result"] = "skip"
        _write_json(args.state_file, state)
        _output("mode", "skip")
        _output("request_file", str(_request_path_from_output(skip)))
        _output("target_ref", f"refs/heads/{args.default_branch}")
        _output("target_sha", args.default_branch_sha)
        return 0

    selected = _run_keyval(
        [
            sys.executable,
            "scripts/workflows/cop_fix_lifecycle.py",
            "select-backend",
            "--cop",
            cop,
            "--mode",
            mode,
            "--backend-input",
            "auto",
            "--issue-number",
            str(routed.issue_number),
            "--repo",
            args.repo,
            "--binary",
            "target/debug/nitrocop",
        ],
        cwd=REPO_ROOT,
    )
    backend = selected["backend"]
    backend_config = _run_keyval(
        [
            sys.executable,
            "scripts/workflows/cop_fix_lifecycle.py",
            "resolve-backend",
            "--backend",
            backend,
        ],
        cwd=REPO_ROOT,
    )
    claim = _run_keyval(
        [
            sys.executable,
            "scripts/workflows/cop_fix_publish.py",
            "claim-request",
            "--output-dir",
            str(request_dir),
            "--cop",
            cop,
            "--mode",
            mode,
            "--branch",
            init["branch"],
            "--backend",
            backend,
            "--backend-label",
            selected["display_label"],
            "--model-label",
            backend_config["model_label"],
            "--backend-reason",
            selected["reason"],
            "--run-url",
            f"{os.environ['GITHUB_SERVER_URL']}/{os.environ['GITHUB_REPOSITORY']}/actions/runs/{os.environ['GITHUB_RUN_ID']}",
            "--issue-number",
            str(routed.issue_number),
            "--code-bugs",
            str(code_bugs),
            "--tokens",
            tokens,
            "--task-file",
            str(task_file),
        ],
        cwd=REPO_ROOT,
    )

    state.update(
        {
            "result": "agent",
            "backend": backend,
            "backend_label": selected["display_label"],
            "backend_reason": selected["reason"],
            "model_label": backend_config["model_label"],
        }
    )
    _write_json(args.state_file, state)

    _output("mode", "agent")
    _output("backend", backend)
    _output("diff_paths", DIFF_PATHS)
    _output("request_file", str(_request_path_from_output(claim)))
    _output("target_ref", f"refs/heads/{args.default_branch}")
    _output("target_sha", args.default_branch_sha)
    return 0


def cmd_prepare_agent(args: argparse.Namespace) -> int:
    state = _read_json(args.state_file)
    claim_metadata = _read_json(args.claim_metadata_file)

    _run(
        [sys.executable, "scripts/workflows/wait_healthy_main.py", "--repo", args.repo],
        cwd=REPO_ROOT,
    )
    prepared = _run_keyval(
        [
            sys.executable,
            "scripts/workflows/cop_fix_lifecycle.py",
            "prepare-branch",
            "--branch",
            state["branch"],
            "--cop",
            state["cop"],
            "--filter",
            state["filter"],
        ],
        cwd=REPO_ROOT,
    )

    state["claim_pr_url"] = str(claim_metadata.get("pr_url", ""))
    state["claim_branch"] = str(claim_metadata.get("branch", state["branch"]))
    _write_json(args.state_file, state)

    _output("base_sha", prepared["branch_base_sha"])
    _output("target_ref", f"refs/heads/{state['branch']}")
    return 0


def cmd_finalize(args: argparse.Namespace) -> int:
    state = _read_json(args.state_file)
    claim_metadata = _read_json(args.claim_metadata_file)
    pr_url = str(claim_metadata.get("pr_url", state.get("claim_pr_url", ""))).strip()
    if not pr_url:
        raise SystemExit("claim metadata did not include pr_url")

    _run(
        [
            sys.executable,
            "scripts/workflows/cop_fix_lifecycle.py",
            "snapshot",
            "--base-sha",
            args.base_sha,
            "--cop",
            state["cop"],
            "--backend",
            state["backend"],
            "--mode",
            state["mode"],
            "--run-url",
            args.run_url,
            "--run-number",
            args.run_number,
        ],
        cwd=REPO_ROOT,
    )
    finalized = _run_keyval(
        [
            sys.executable,
            "scripts/workflows/cop_fix_lifecycle.py",
            "finalize",
            "--cop",
            state["cop"],
            "--branch",
            state["branch"],
            "--base-sha",
            args.base_sha,
            "--pr-url",
            pr_url,
            "--backend",
            state["backend"],
            "--backend-label",
            state["backend_label"],
            "--model-label",
            state["model_label"],
            "--mode",
            state["mode"],
            "--issue-number",
            state["issue_number"],
            "--repo",
            args.repo,
            "--run-url",
            args.run_url,
            "--run-number",
            args.run_number,
            "--tokens",
            state["tokens"],
        ],
        cwd=REPO_ROOT,
    )

    request = _run_keyval(
        [
            sys.executable,
            "scripts/workflows/cop_fix_publish.py",
            "finalize-request",
            "--output-dir",
            str(args.output_dir),
            "--repo-root",
            str(REPO_ROOT),
            "--result",
            finalized["result"],
            "--cop",
            state["cop"],
            "--backend",
            state["backend"],
            "--backend-label",
            state["backend_label"],
            "--model-label",
            state["model_label"],
            "--mode",
            state["mode"],
            "--issue-number",
            state["issue_number"],
            "--run-url",
            args.run_url,
            "--base-sha",
            args.base_sha,
            "--pr-url",
            pr_url,
            "--pr-number",
            _parse_pr_number(pr_url),
            "--pr-body-file",
            os.environ["PR_BODY_FILE"],
            "--scope-report-file",
            os.environ["AGENT_SCOPE_REPORT_FILE"],
        ],
        cwd=REPO_ROOT,
    )

    _output("result", finalized["result"])
    _output("request_file", str(_request_path_from_output(request)))
    _output("target_ref", f"refs/heads/{state['branch']}")
    _output("target_sha", args.base_sha)
    return 0


def cmd_cleanup(args: argparse.Namespace) -> int:
    if not args.state_file.exists():
        return 0

    state = _read_json(args.state_file)
    pr_url = ""
    if args.claim_metadata_file and args.claim_metadata_file.exists():
        pr_url = str(_read_json(args.claim_metadata_file).get("pr_url", "")).strip()

    request = _run_keyval(
        [
            sys.executable,
            "scripts/workflows/cop_fix_publish.py",
            "cleanup-request",
            "--output-dir",
            str(args.output_dir),
            "--cop",
            state["cop"],
            *(["--pr", pr_url] if pr_url else []),
            "--issue-number",
            state["issue_number"],
            "--backend-label",
            state.get("backend_label", state.get("backend_input", "auto")),
            "--model-label",
            state.get("model_label", "n/a"),
            "--mode",
            state["mode"],
            "--run-url",
            args.run_url,
        ],
        cwd=REPO_ROOT,
    )

    _output("request_file", str(_request_path_from_output(request)))
    _output("target_ref", f"refs/heads/{state['default_branch']}")
    _output("target_sha", state["default_branch_sha"])
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    route = subparsers.add_parser("route")
    route.add_argument("--repo", required=True)
    route.add_argument("--payload-json", required=True)
    route.set_defaults(func=cmd_route)

    prepare = subparsers.add_parser("prepare")
    prepare.add_argument("--repo", required=True)
    prepare.add_argument("--payload-json", required=True)
    prepare.add_argument("--default-branch", required=True)
    prepare.add_argument("--default-branch-sha", required=True)
    prepare.add_argument("--state-file", type=Path, required=True)
    prepare.set_defaults(func=cmd_prepare)

    prepare_agent = subparsers.add_parser("prepare-agent")
    prepare_agent.add_argument("--repo", required=True)
    prepare_agent.add_argument("--state-file", type=Path, required=True)
    prepare_agent.add_argument("--claim-metadata-file", type=Path, required=True)
    prepare_agent.set_defaults(func=cmd_prepare_agent)

    finalize = subparsers.add_parser("finalize")
    finalize.add_argument("--repo", required=True)
    finalize.add_argument("--state-file", type=Path, required=True)
    finalize.add_argument("--claim-metadata-file", type=Path, required=True)
    finalize.add_argument("--base-sha", required=True)
    finalize.add_argument("--output-dir", type=Path, required=True)
    finalize.add_argument("--run-url", required=True)
    finalize.add_argument("--run-number", required=True)
    finalize.set_defaults(func=cmd_finalize)

    cleanup = subparsers.add_parser("cleanup")
    cleanup.add_argument("--repo", required=True)
    cleanup.add_argument("--state-file", type=Path, required=True)
    cleanup.add_argument("--claim-metadata-file", type=Path, required=True)
    cleanup.add_argument("--output-dir", type=Path, required=True)
    cleanup.add_argument("--run-url", required=True)
    cleanup.set_defaults(func=cmd_cleanup)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
