#!/usr/bin/env python3
"""Bridge a local workflow to the 6/bot remote repo-write runner."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import tempfile
import time
from pathlib import Path

CONTROL_REF = "main"
REMOTE_REPO_WRITE_WORKFLOW = "remote-repo-write.yml"


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


def _copy_request_files(request_path: Path, output_dir: Path) -> dict:
    request = json.loads(request_path.read_text())
    operations = request.get("operations", [])
    if not isinstance(operations, list):
        raise SystemExit("request.operations must be a list")

    copied_operations: list[dict] = []
    for index, operation in enumerate(operations):
        if not isinstance(operation, dict):
            raise SystemExit(f"operation #{index + 1} must be an object")
        copied = dict(operation)
        for field in ("body_file", "patch_file"):
            if field not in copied:
                continue
            source = Path(copied[field])
            if not source.is_absolute():
                source = request_path.parent / source
            source = source.resolve()
            if not source.is_file():
                raise SystemExit(f"{field} not found: {source}")
            target_name = f"op{index + 1}-{field}-{source.name}"
            shutil.copy2(source, output_dir / target_name)
            copied[field] = target_name
        copied_operations.append(copied)

    request["operations"] = copied_operations
    return request


def cmd_prepare_input(args: argparse.Namespace) -> int:
    request_path = Path(args.request_file).resolve()
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    request = _copy_request_files(request_path, output_dir)
    (output_dir / "request.json").write_text(json.dumps(request, indent=2) + "\n")
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
    }
    request = {
        "ref": CONTROL_REF,
        "inputs": {
            "request_id": args.request_id,
            "source_repo": args.source_repo,
            "payload": json.dumps(payload),
        },
    }

    with tempfile.NamedTemporaryFile("w", delete=False) as handle:
        json.dump(request, handle)
        handle.write("\n")
        payload_path = handle.name

    try:
        _run(
            [
                "gh",
                "api",
                f"repos/{args.control_repo}/actions/workflows/{REMOTE_REPO_WRITE_WORKFLOW}/dispatches",
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
            REMOTE_REPO_WRITE_WORKFLOW,
            "--event",
            "workflow_dispatch",
            "--json",
            "databaseId,displayTitle,status,conclusion,url",
            "-L",
            "50",
        )
    )
    expected_title = f"remote-repo-write {request_id}"
    for run in runs:
        if run.get("displayTitle") == expected_title:
            return run
    return None


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

    metadata_matches = list(download_dir.rglob("remote-repo-write-metadata.json"))
    if not metadata_matches:
        raise SystemExit("Remote output artifact did not contain remote-repo-write-metadata.json")

    metadata = json.loads(metadata_matches[0].read_text())
    _output("run_id", str(run["databaseId"]))
    _output("run_url", run.get("url", ""))
    _output("signed_sha", str(metadata.get("signed_sha", "")))
    _output("unsigned_sha", str(metadata.get("unsigned_sha", "")))
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    prepare = subparsers.add_parser("prepare-input")
    prepare.add_argument("--request-file", required=True)
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
    dispatch.set_defaults(func=cmd_dispatch)

    await_output = subparsers.add_parser("await-output")
    await_output.add_argument("--control-repo", required=True)
    await_output.add_argument("--request-id", required=True)
    await_output.add_argument("--output-artifact", required=True)
    await_output.add_argument("--download-dir", required=True)
    await_output.add_argument("--timeout-sec", type=int, default=3600)
    await_output.add_argument("--poll-interval-sec", type=int, default=10)
    await_output.set_defaults(func=cmd_await_output)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
