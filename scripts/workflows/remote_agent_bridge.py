#!/usr/bin/env python3
"""Bridge a local workflow to the 6/bot remote agent runner."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import tempfile
import time
from pathlib import Path

CONTROL_REF = "main"
REMOTE_AGENT_WORKFLOW = "remote-agent.yml"


def _env_path(name: str) -> Path:
    value = os.environ.get(name)
    if not value:
        raise SystemExit(f"Required environment variable {name} is not set")
    return Path(value)


def _run(cmd: list[str], *, capture: bool = True, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        cmd,
        text=True,
        capture_output=capture,
        check=check,
    )


def _gh(*args: str) -> str:
    return _run(["gh", *args]).stdout


def _output(name: str, value: str) -> None:
    print(f"{name}={value}")


def cmd_prepare_input(args: argparse.Namespace) -> int:
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    shutil.copy2(_env_path("FINAL_TASK_FILE"), output_dir / "final-task.md")

    task_file = _env_path("TASK_FILE")
    if task_file.exists():
        shutil.copy2(task_file, output_dir / "task.md")

    return 0


def cmd_dispatch(args: argparse.Namespace) -> int:
    payload = {
        "request_id": args.request_id,
        "source_repo": args.source_repo,
        "source_run_id": args.source_run_id,
        "input_artifact": args.input_artifact,
        "output_artifact": args.output_artifact,
        "target_sha": args.target_sha,
        "target_ref": args.target_ref,
        "backend": args.backend,
        "base_sha": args.base_sha,
        "workflow": args.workflow,
        "diff_paths": args.diff_paths,
        "setup_profile": args.setup_profile,
        "setup_config_json": json.loads(args.setup_config_json),
    }
    request = {
        "ref": CONTROL_REF,
        "inputs": {
            "request_id": args.request_id,
            "source_repo": args.source_repo,
            "payload": json.dumps(payload),
        },
    }

    with tempfile.NamedTemporaryFile("w", delete=False) as f:
        json.dump(request, f)
        f.write("\n")
        payload_path = f.name

    try:
        _run(
            [
                "gh",
                "api",
                f"repos/{args.control_repo}/actions/workflows/{REMOTE_AGENT_WORKFLOW}/dispatches",
                "--method",
                "POST",
                "--input",
                payload_path,
            ],
            capture=True,
        )
    finally:
        Path(payload_path).unlink(missing_ok=True)

    _output("request_id", args.request_id)
    return 0


def _find_run(control_repo: str, request_id: str) -> dict | None:
    runs = json.loads(
        _gh(
            "run",
            "list",
            "-R",
            control_repo,
            "-w",
            REMOTE_AGENT_WORKFLOW,
            "--event",
            "workflow_dispatch",
            "--json",
            "databaseId,displayTitle,status,conclusion,url",
            "-L",
            "50",
        )
    )
    expected_title = f"remote-agent {request_id}"
    for run in runs:
        if run.get("displayTitle") == expected_title:
            return run
    return None


def _copy_tree(src: Path, dest: Path) -> None:
    for item in src.iterdir():
        target = dest / item.name
        if item.is_dir():
            shutil.copytree(item, target, dirs_exist_ok=True)
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(item, target)


def cmd_await_output(args: argparse.Namespace) -> int:
    deadline = time.time() + args.timeout_sec
    run = None
    while time.time() < deadline:
        run = _find_run(args.control_repo, args.request_id)
        if run is None:
            time.sleep(args.poll_interval_sec)
            continue
        if run.get("status") != "completed":
            time.sleep(args.poll_interval_sec)
            continue
        break

    if run is None:
        raise SystemExit(f"Timed out waiting for remote run {args.request_id} in {args.control_repo}")
    if run.get("status") != "completed":
        raise SystemExit(f"Timed out waiting for remote run completion: {run.get('url', '')}")
    if run.get("conclusion") != "success":
        raise SystemExit(
            f"Remote run failed with conclusion {run.get('conclusion')}: {run.get('url', '')}"
        )

    download_dir = Path(args.download_dir)
    download_dir.mkdir(parents=True, exist_ok=True)
    _run(
        [
            "gh",
            "run",
            "download",
            str(run["databaseId"]),
            "-R",
            args.control_repo,
            "-n",
            args.output_artifact,
            "-D",
            str(download_dir),
        ],
        capture=True,
    )

    metadata_matches = list(download_dir.rglob("remote-agent-metadata.json"))
    if not metadata_matches:
        raise SystemExit("Remote output artifact did not contain remote-agent-metadata.json")

    remote_root = metadata_matches[0].parent
    extract_into = Path(args.extract_into)
    extract_into.mkdir(parents=True, exist_ok=True)
    _copy_tree(remote_root, extract_into)

    metadata = json.loads((extract_into / "remote-agent-metadata.json").read_text())
    _output("run_id", str(run["databaseId"]))
    _output("run_url", run.get("url", ""))
    _output("leak_scan_ok", "true" if metadata.get("leak_scan_ok") else "false")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    prepare = subparsers.add_parser("prepare-input")
    prepare.add_argument("--output-dir", required=True)
    prepare.set_defaults(func=cmd_prepare_input)

    dispatch = subparsers.add_parser("dispatch")
    dispatch.add_argument("--control-repo", required=True)
    dispatch.add_argument("--request-id", required=True)
    dispatch.add_argument("--source-repo", required=True)
    dispatch.add_argument("--source-run-id", required=True)
    dispatch.add_argument("--input-artifact", required=True)
    dispatch.add_argument("--output-artifact", required=True)
    dispatch.add_argument("--target-sha", required=True)
    dispatch.add_argument("--target-ref", required=True)
    dispatch.add_argument("--backend", required=True)
    dispatch.add_argument("--base-sha", required=True)
    dispatch.add_argument("--workflow", required=True)
    dispatch.add_argument("--diff-paths", default="")
    dispatch.add_argument("--setup-profile", default="")
    dispatch.add_argument("--setup-config-json", default="{}")
    dispatch.set_defaults(func=cmd_dispatch)

    await_output = subparsers.add_parser("await-output")
    await_output.add_argument("--control-repo", required=True)
    await_output.add_argument("--request-id", required=True)
    await_output.add_argument("--output-artifact", required=True)
    await_output.add_argument("--download-dir", required=True)
    await_output.add_argument("--extract-into", required=True)
    await_output.add_argument("--timeout-sec", type=int, default=3600)
    await_output.add_argument("--poll-interval-sec", type=int, default=10)
    await_output.set_defaults(func=cmd_await_output)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
