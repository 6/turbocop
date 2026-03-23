#!/usr/bin/env python3
"""Precompute local changed-cop corpus diagnostics for PR repair."""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from pathlib import Path

REPO_OFFENSE_RE = re.compile(r"^\s*\d+\s+([^\s]+)\s*$")


def tail_lines(text: str, max_lines: int = 220) -> str:
    lines = [line.rstrip() for line in text.splitlines()]
    if len(lines) <= max_lines:
        return "\n".join(lines)
    kept = lines[-max_lines:]
    return "\n".join(
        [
            f"... (truncated, showing last {max_lines} of {len(lines)} lines) ...",
            *kept,
        ]
    )


def extract_top_repo_ids(output: str, limit: int = 5) -> list[str]:
    repo_ids: list[str] = []
    in_repo_block = False
    for line in output.splitlines():
        stripped = line.strip()
        if stripped.startswith("Repos with offenses "):
            in_repo_block = True
            continue
        if not in_repo_block:
            continue
        if not stripped:
            break
        if stripped.startswith("... and "):
            break
        match = REPO_OFFENSE_RE.match(line)
        if match:
            repo_ids.append(match.group(1))
            if len(repo_ids) >= limit:
                break
    return repo_ids


def render_start_here(
    cop: str,
    top_repos: list[str],
    *,
    standard_corpus: Path | None,
    corpus_dir: Path,
) -> list[str]:
    lines = [
        "Start here:",
        f"- Re-run after edits: `python3 scripts/check-cop.py {cop} --verbose --rerun --quick --clone`",
    ]
    if standard_corpus is not None:
        lines.append(
            f"- Baseline corpus context: `python3 scripts/investigate-cop.py {cop} --input {standard_corpus} --repos-only`"
        )
    for repo_id in top_repos:
        lines.append(f"- Inspect repo: `{corpus_dir / repo_id}`")
    return lines


def render_packet(
    results: list[dict[str, object]],
    *,
    standard_corpus: Path | None = None,
    corpus_dir: Path = Path("vendor/corpus"),
) -> str:
    lines = [
        "",
        "## Local Cop-Check Diagnosis",
        "",
        "The workflow already reran the changed-cop corpus check locally before agent execution.",
        "Use this packet as the starting point instead of rediscovering the same corpus regression.",
        "",
    ]

    if not results:
        lines.extend(
            [
                "No changed cops were detected for local corpus diagnosis.",
                "",
            ]
        )
        return "\n".join(lines)

    lines.extend(
        [
            "Changed cops:",
            *[f"- `{result['cop']}`" for result in results],
            "",
        ]
    )

    for result in results:
        top_repos = extract_top_repo_ids(str(result["output"]))
        lines.extend(
            [
                f"### {result['cop']}",
                "",
                *render_start_here(
                    str(result["cop"]),
                    top_repos,
                    standard_corpus=standard_corpus,
                    corpus_dir=corpus_dir,
                ),
                "",
                "```bash",
                result["command"],
                "```",
                "",
                f"Exit status: `{result['status']}`",
                "",
                "```text",
                result["output"],
                "```",
                "",
            ]
        )

    return "\n".join(lines).rstrip() + "\n"


def run_capture(cmd: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        cmd,
        cwd=str(cwd),
        capture_output=True,
        text=True,
        check=False,
    )


def main() -> int:
    parser = argparse.ArgumentParser(description="Precompute changed-cop corpus diagnostics")
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--changed-cops-out", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    repo_root = args.repo_root.resolve()
    standard_corpus = os.environ.get("REPAIR_CORPUS_STANDARD_FILE")
    changed_result = run_capture(
        [
            sys.executable,
            "scripts/dispatch-cops.py",
            "changed",
            "--base",
            "origin/main",
            "--head",
            "HEAD",
        ],
        repo_root,
    )

    results: list[dict[str, object]] = []
    if changed_result.returncode == 0:
        cops = [line.strip() for line in changed_result.stdout.splitlines() if line.strip()]
        args.changed_cops_out.parent.mkdir(parents=True, exist_ok=True)
        args.changed_cops_out.write_text("\n".join(cops) + ("\n" if cops else ""))
        for cop in cops:
            cmd = [
                sys.executable,
                "scripts/check-cop.py",
                cop,
                "--verbose",
                "--rerun",
                "--quick",
                "--clone",
            ]
            result = run_capture(cmd, repo_root)
            results.append(
                {
                    "cop": cop,
                    "command": " ".join(cmd),
                    "status": result.returncode,
                    "output": tail_lines((result.stdout + result.stderr).strip()),
                }
            )
    else:
        args.changed_cops_out.parent.mkdir(parents=True, exist_ok=True)
        args.changed_cops_out.write_text("")
        results.append(
            {
                "cop": "(changed-cops detection failed)",
                "command": f"{sys.executable} scripts/dispatch-cops.py changed --base origin/main --head HEAD",
                "status": changed_result.returncode,
                "output": tail_lines((changed_result.stdout + changed_result.stderr).strip()),
            }
        )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        render_packet(
            results,
            standard_corpus=Path(standard_corpus) if standard_corpus else None,
            corpus_dir=repo_root / "vendor" / "corpus",
        )
    )
    failed = sum(1 for result in results if isinstance(result["status"], int) and result["status"] != 0)
    print(f"changed_cops={sum(1 for result in results if result['cop'] != '(changed-cops detection failed)')}")
    print(f"failed_cops={failed}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
