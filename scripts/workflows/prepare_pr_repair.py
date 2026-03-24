#!/usr/bin/env python3
"""Prepare an automatic PR repair attempt from a failed Checks run.

Fetches the failed workflow run, classifies the failure as easy/hard/skip,
builds a concise repair prompt, and writes a deterministic verification script
that the workflow can rerun before pushing any changes.
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import agent_runtime
from shared.corpus_artifacts import download_corpus_results as _download_corpus

ANSI_RE = re.compile(r"\x1B\[[0-?]*[ -/]*[@-~]")
ACTIONS_LOG_PREFIX_RE = re.compile(
    r"^[^\t]+\t[^\t]+\t\d{4}-\d{2}-\d{2}T[\d:.+-]+Z\s*"
)
FAILED_CONCLUSIONS = {"failure", "cancelled", "timed_out", "action_required", "startup_failure"}
SKIPPABLE_STEP_NAMES = {
    "Set up job",
    "Run actions/checkout@v4",
    "Run dtolnay/rust-toolchain@stable",
    "Run Swatinem/rust-cache@v2",
    "Run ruby/setup-ruby@v1",
    "Cache corpus bundle",
    "Install corpus gems",
    "Post Cache corpus bundle",
    "Post Run Swatinem/rust-cache@v2",
    "Post Run actions/checkout@v4",
    "Complete job",
}

PYTHON_WORKFLOW_STEPS = {
    "Format",
    "Validate corpus manifest",
    "No vendor include macros",
    "Python script tests",
}

# Steps that a cop-fix bot (which only changes src/cop/*.rs and test fixtures)
# could never have caused to fail. If ALL failed steps are in this set or
# match COP_FIX_UNRELATED_RE, the failure is an infra issue — skip repair.
COP_FIX_UNRELATED_STEPS = {
    "Validate corpus manifest",
    "No vendor include macros",
    "Gem builder tests",
}
COP_FIX_UNRELATED_RE = re.compile(r"^Python\b", re.IGNORECASE)

EASY_STEP_COMMANDS = {
    "Format": "cargo fmt --check",
    "Validate corpus manifest": "python3 bench/corpus/validate_manifest.py",
    "No vendor include macros": "bash scripts/workflows/check_no_vendor_include_macros.sh",
    "Gem builder tests": "ruby gem/test/gem_builder_test.rb",
    "Python script tests": "uv run pytest tests/python/ -v",
    "Clippy": "cargo clippy --profile ci -- -D warnings",
    "Build": "cargo build --profile ci",
    "Build release binary": "cargo build --release",
    "Compile tests": "cargo test --no-run",
    "NodePattern verifier": "cargo test verifier -- --nocapture",
    "Config audit": "cargo test config_audit -- --nocapture",
    "Prism pitfalls": "cargo test prism_pitfalls -- --nocapture",
    "File discovery sync": "cargo test file_discovery_sync -- --nocapture",
    "Test": "cargo test",
}

HARD_STEP_COMMANDS = {
    "Check cops against corpus baseline": (
        "python3 scripts/dispatch_cops.py changed --base origin/main --head HEAD > \"$REPAIR_CHANGED_COPS_FILE\"\n"
        "failed=0\n"
        "while IFS= read -r cop; do\n"
        "  [ -z \"$cop\" ] && continue\n"
        "  echo \"==============================\"\n"
        "  echo \"Checking: $cop (re-running against corpus)\"\n"
        "  echo \"==============================\"\n"
        "  if ! python3 scripts/check_cop.py \"$cop\" --verbose --rerun --quick --clone; then\n"
        "    echo \"FAIL: $cop regression detected\"\n"
        "    failed=$((failed + 1))\n"
        "  fi\n"
        "done < \"$REPAIR_CHANGED_COPS_FILE\"\n"
        "test \"$failed\" -eq 0"
    ),
    "Run smoke test": "python3 scripts/corpus_smoke_test.py --binary target/release/nitrocop",
}


def strip_ansi(text: str) -> str:
    return ANSI_RE.sub("", text)


def strip_actions_log_prefix(line: str) -> str:
    return ACTIONS_LOG_PREFIX_RE.sub("", line, count=1).lstrip()


def is_actions_log_noise(line: str) -> bool:
    stripped = line.strip()
    if not stripped:
        return False
    noise_prefixes = (
        "[command]/usr/bin/git ",
        "Entering '",
        "Temporarily overriding HOME=",
        "Adding repository directory to the temporary git global config as a safe directory",
        "Cleaning up orphan processes",
    )
    if stripped.startswith(noise_prefixes):
        return True
    noise_exact = {
        "Post job cleanup.",
        "http.https://github.com/.extraheader",
    }
    if stripped in noise_exact:
        return True
    if stripped.startswith("git version "):
        return True
    if stripped.startswith("##[warning]Node.js "):
        return True
    if stripped.startswith("file:") and stripped.endswith("\tremote.origin.url"):
        return True
    return False


def normalize_log(text: str, max_lines: int = 80) -> str:
    lines = []
    for raw_line in text.splitlines():
        line = strip_actions_log_prefix(strip_ansi(raw_line).rstrip())
        if not line.strip():
            continue
        if is_actions_log_noise(line):
            continue
        lines.append(line)
    if len(lines) > max_lines:
        lines = ["... (truncated, showing last %d lines) ..." % max_lines] + lines[-max_lines:]
    return "\n".join(lines)


def run_gh(args: list[str], repo: str | None = None, check: bool = True) -> str:
    cmd = ["gh"] + args
    if repo:
        cmd += ["-R", repo]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if check and result.returncode != 0:
        raise subprocess.CalledProcessError(result.returncode, cmd, result.stdout, result.stderr)
    return result.stdout


def load_run(repo: str, run_id: str) -> dict:
    output = run_gh(
        [
            "run",
            "view",
            run_id,
            "--json",
            "databaseId,workflowName,headSha,event,conclusion,jobs,url,number,headBranch",
        ],
        repo=repo,
    )
    return json.loads(output)


def failed_steps(job: dict) -> list[str]:
    names: list[str] = []
    for step in job.get("steps", []):
        if step.get("conclusion") in FAILED_CONCLUSIONS:
            name = step.get("name", "").strip()
            if name:
                names.append(name)
    if names:
        return names
    if job.get("conclusion") in FAILED_CONCLUSIONS:
        return [job.get("name", "unknown job")]
    return []


def job_route(job: dict) -> str:
    name = job.get("name", "")
    steps = failed_steps(job)
    if not steps:
        return "skip"
    if "macos" in name.lower():
        return "skip"
    route = "skip"
    for step_name in steps:
        if step_name in HARD_STEP_COMMANDS:
            return "hard"
        if step_name in EASY_STEP_COMMANDS:
            route = "easy"
            continue
        if step_name.startswith("Post ") or step_name in SKIPPABLE_STEP_NAMES:
            continue
        return "hard"
    return route


def command_for_step(step_name: str) -> str | None:
    if step_name in HARD_STEP_COMMANDS:
        return HARD_STEP_COMMANDS[step_name]
    if step_name in EASY_STEP_COMMANDS:
        return EASY_STEP_COMMANDS[step_name]
    return None


def classify_run(run: dict) -> dict:
    jobs = [job for job in run.get("jobs", []) if job.get("conclusion") in FAILED_CONCLUSIONS]
    commands: list[str] = []
    command_keys: set[str] = set()
    hard_jobs: list[dict] = []
    easy_jobs: list[dict] = []
    skip_jobs: list[dict] = []

    for job in jobs:
        route = job_route(job)
        job["repair_route"] = route
        job["failed_step_names"] = failed_steps(job)
        for step_name in job["failed_step_names"]:
            command = command_for_step(step_name)
            if command and command not in command_keys:
                command_keys.add(command)
                commands.append(command)
        if route == "hard":
            hard_jobs.append(job)
        elif route == "easy":
            easy_jobs.append(job)
        else:
            skip_jobs.append(job)

    # Check if ALL failures are in steps unrelated to cop changes.
    # If so, this is an infra issue the bot can't fix — skip repair.
    all_failed_step_names = {
        step_name
        for job in jobs
        for step_name in job.get("failed_step_names", [])
    }
    def _is_unrelated(step: str) -> bool:
        return step in COP_FIX_UNRELATED_STEPS or step in SKIPPABLE_STEP_NAMES or bool(COP_FIX_UNRELATED_RE.match(step))

    unrelated_failures = {s for s in all_failed_step_names if _is_unrelated(s)}
    non_skippable_failures = all_failed_step_names - unrelated_failures
    if all_failed_step_names and not non_skippable_failures:
        route = "skip"
        backend = ""
        infra_names = sorted(all_failed_step_names - SKIPPABLE_STEP_NAMES)
        reasons = [f"All failures are in infra steps unrelated to cop changes: "
                    f"{', '.join(infra_names)}"]
        return {
            "route": route,
            "backend": backend,
            "guard_profile": "",
            "cop_check_failure": False,
            "jobs": jobs,
            "hard_jobs": [],
            "easy_jobs": [],
            "skip_jobs": jobs,
            "verification_commands": [],
            "reason": "; ".join(reasons),
        }

    if hard_jobs:
        route = "hard"
        backend = "codex-hard"
    elif easy_jobs:
        route = "easy"
        backend = "codex-hard"
    else:
        route = "skip"
        backend = ""

    reasons = []
    for job in hard_jobs + easy_jobs + skip_jobs:
        steps = ", ".join(job.get("failed_step_names", [])) or job.get("name", "unknown")
        reasons.append(f"{job.get('name', 'unknown')}: {steps}")

    return {
        "route": route,
        "backend": backend,
        "guard_profile": determine_guard_profile(route, jobs),
        "cop_check_failure": any(
            "Check cops against corpus baseline" in job.get("failed_step_names", [])
            for job in jobs
        ),
        "jobs": jobs,
        "hard_jobs": hard_jobs,
        "easy_jobs": easy_jobs,
        "skip_jobs": skip_jobs,
        "verification_commands": commands,
        "reason": "; ".join(reasons) if reasons else "No repairable failed jobs",
    }


def determine_guard_profile(route: str, jobs: list[dict]) -> str:
    failed_step_names = {
        step_name
        for job in jobs
        for step_name in job.get("failed_step_names", [])
    }
    if route == "hard":
        if "Check cops against corpus baseline" in failed_step_names:
            return "repair-cop-check"
        return "repair-smoke"
    if route == "easy":
        if failed_step_names & PYTHON_WORKFLOW_STEPS:
            return "repair-python-workflow"
        return "repair-rust-test"
    return ""


def fetch_failed_log(repo: str, run_id: str, job_id: str) -> str:
    return run_gh(
        ["run", "view", run_id, "--job", job_id, "--log-failed"],
        repo=repo,
        check=False,
    )


def build_verification_script(commands: list[str]) -> str:
    parts = ["#!/usr/bin/env bash", "set -euo pipefail", ""]
    for idx, command in enumerate(commands, 1):
        parts.append(f"echo \"=== Verification {idx}/{len(commands)} ===\"")
        parts.append(command)
        parts.append("")
    return "\n".join(parts).rstrip() + "\n"


def excerpt_diff(diff_text: str, max_lines: int = 220) -> str:
    lines = diff_text.splitlines()
    if len(lines) > max_lines:
        lines = lines[:max_lines] + [f"... ({len(diff_text.splitlines()) - max_lines} more lines truncated)"]
    return "\n".join(lines)


def prefetch_corpus_context(route: str) -> dict[str, str]:
    if route != "hard":
        return {}

    runtime_paths = agent_runtime.current_paths("agent-pr-repair")
    source_path, run_id, head_sha = _download_corpus()
    target_path = Path(runtime_paths["REPAIR_CORPUS_FILE"])
    shutil.copy2(source_path, target_path)
    return {
        "path": str(target_path),
        "run_id": str(run_id),
        "head_sha": head_sha,
    }


def build_prompt(
    run: dict,
    classification: dict,
    pr_meta: dict,
    diff_stat: str,
    diff_text: str,
    extra_context: str,
    corpus_context: dict[str, dict[str, str]] | None = None,
) -> str:
    route = classification["route"]
    backend = classification["backend"] or "none"
    backend_label_map = {
        "codex-normal": "codex / normal",
        "codex-hard": "codex / hard",
        "claude-normal": "claude / normal",
        "claude-hard": "claude / hard",
        "minimax": "minimax / normal",
    }
    backend_label = backend_label_map.get(backend, backend)
    lines = [
        f"# PR Repair Task: PR #{pr_meta['number']}",
        "",
        "Before making changes, read `docs/agent-ci.md`.",
        "",
        "Repair the existing PR branch in place. Do not create a new branch or PR.",
        "Preserve the intent of the current PR and make the smallest changes needed to fix the failing checks.",
        "Do not repair this PR by reverting it back to `origin/main`, deleting the whole diff, or otherwise turning it into an empty/no-op PR.",
        "If the only plausible fix is a full revert of the PR, stop and explain that clearly instead of performing the revert.",
        "Do not edit unrelated files or do broad cleanup.",
        "",
        "## Context",
        "",
        f"- PR: #{pr_meta['number']} {pr_meta['title']}",
        f"- Branch: `{pr_meta['headRefName']}`",
        f"- Checks run: #{run.get('number', '?')} ({run.get('workflowName', 'Checks')})",
        f"- Route: `{route}`",
        f"- Selected backend: `{backend_label}`",
        f"- Failure summary: {classification['reason']}",
        "",
        "## Required Outcome",
        "",
        "Fix the currently failing checks shown below.",
        "Before finishing, run the targeted validations needed to make the workflow's final verification likely to pass.",
        "",
        "## Required Local Verification",
        "",
        "The workflow will rerun these commands before pushing. Your changes should make them pass:",
        "",
        "```bash",
    ]
    for command in classification["verification_commands"]:
        lines.append(command)
        lines.append("")
    lines += [
        "```",
        "",
        "## Current PR Diff Stat",
        "",
        "```",
        diff_stat.strip() or "(no diff stat available)",
        "```",
        "",
        "## Current PR Diff Excerpt",
        "",
        "```diff",
        excerpt_diff(diff_text),
        "```",
        "",
    ]

    if corpus_context:
        lines.extend([
            "## Local Corpus Context",
            "",
            "These corpus oracle artifacts are already downloaded locally by the workflow.",
            "Prefer these local files over re-downloading the same corpus data from GitHub Actions.",
            "If you still need GitHub metadata for debugging, a read-only token is available in `GH_TOKEN`.",
            "",
        ])
        corpus_path = corpus_context.get("path")
        if corpus_path:
            lines.append(
                f"- Corpus JSON (matches the PR `cop-check` gate): `{corpus_path}` "
                f"(corpus oracle run #{corpus_context['run_id']})"
            )
        lines.extend([
            "",
            "Use these files directly with the repo scripts when you need corpus context.",
            "",
            "```bash",
        ])
        if corpus_path:
            lines.append(
                f"python3 scripts/investigate_cop.py Department/CopName --input {corpus_path} --context"
            )
            lines.append(
                f"python3 scripts/check_cop.py Department/CopName --input {corpus_path} --verbose --rerun --quick --clone"
            )
        lines.extend([
            "```",
            "",
        ])

    lines.extend([
        "## Failed Checks Packet",
        "",
    ])

    for job in classification["jobs"]:
        lines.append(f"### {job.get('name', 'unknown job')}")
        lines.append("")
        lines.append(f"- Route: `{job.get('repair_route', 'skip')}`")
        lines.append(f"- Failed steps: {', '.join(job.get('failed_step_names', [])) or '(unknown)'}")
        log_text = normalize_log(job.get("failed_log", ""))
        if log_text:
            lines.extend(["", "```text", log_text, "```"])
        lines.append("")

    if extra_context.strip():
        lines.extend([
            "## Additional Instructions",
            "",
            extra_context.strip(),
            "",
        ])

    lines.extend([
        "## Constraints",
        "",
        "- Keep the fix scoped to the failing checks.",
        "- Reuse the existing PR branch and existing tests where possible.",
        "- Prefer the minimal patch that makes the deterministic verification pass.",
        "- A full revert to `origin/main` or an empty PR is treated as a failed repair, not a success.",
        "- If a fix is blocked by missing context, explain that clearly in the final message.",
        "",
    ])
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(description="Prepare an automatic PR repair task")
    parser.add_argument("--repo", required=True, help="owner/repo")
    parser.add_argument("--run-id", required=True, help="Failed Checks workflow run ID")
    parser.add_argument("--pr-number", required=True, help="Target PR number")
    parser.add_argument("--pr-title", required=True, help="Target PR title")
    parser.add_argument("--head-branch", required=True, help="Target PR branch")
    parser.add_argument("--diff-stat", type=Path, required=True, help="Path to diff stat text")
    parser.add_argument("--diff", type=Path, required=True, help="Path to PR diff")
    parser.add_argument("--prompt-out", type=Path, required=True, help="Output markdown prompt")
    parser.add_argument("--verify-out", type=Path, required=True, help="Output shell script")
    parser.add_argument("--json-out", type=Path, required=True, help="Output JSON metadata")
    parser.add_argument(
        "--backend-override",
        choices=[
            "auto",
            "minimax",
            "codex-normal",
            "codex-hard",
            "claude-normal",
            "claude-hard",
        ],
        default="auto",
    )
    parser.add_argument("--extra-context", default="", help="Additional human instructions")
    args = parser.parse_args()

    run = load_run(args.repo, args.run_id)
    classification = classify_run(run)
    if args.backend_override != "auto" and classification["route"] != "skip":
        classification["backend"] = args.backend_override

    for job in classification["jobs"]:
        job["failed_log"] = fetch_failed_log(args.repo, args.run_id, str(job.get("databaseId", "")))

    corpus_context = prefetch_corpus_context(classification["route"])

    pr_meta = {
        "number": args.pr_number,
        "title": args.pr_title,
        "headRefName": args.head_branch,
    }

    prompt = build_prompt(
        run=run,
        classification=classification,
        pr_meta=pr_meta,
        diff_stat=args.diff_stat.read_text() if args.diff_stat.exists() else "",
        diff_text=args.diff.read_text() if args.diff.exists() else "",
        extra_context=args.extra_context,
        corpus_context=corpus_context,
    )
    verify_script = build_verification_script(classification["verification_commands"])

    args.prompt_out.write_text(prompt)
    args.verify_out.write_text(verify_script)
    args.verify_out.chmod(0o755)

    metadata = {
        "route": classification["route"],
        "backend": classification["backend"],
        "guard_profile": classification["guard_profile"],
        "cop_check_failure": classification["cop_check_failure"],
        "reason": classification["reason"],
        "verification_commands": classification["verification_commands"],
        "corpus_context": corpus_context,
        "failed_jobs": [
            {
                "name": job.get("name"),
                "route": job.get("repair_route"),
                "failed_steps": job.get("failed_step_names", []),
                "url": job.get("url"),
            }
            for job in classification["jobs"]
        ],
    }
    args.json_out.write_text(json.dumps(metadata, indent=2) + "\n")

    print(f"route={classification['route']}")
    print(f"backend={classification['backend']}")
    print(f"guard_profile={classification['guard_profile']}")
    print(f"cop_check_failure={'true' if classification['cop_check_failure'] else 'false'}")
    print(f"reason={classification['reason']}")
    print(f"failed_jobs={len(classification['jobs'])}")
    if "path" in corpus_context:
        print(f"corpus={corpus_context['path']}")


if __name__ == "__main__":
    main()
