#!/usr/bin/env python3
from __future__ import annotations

"""Investigate regressions between two corpus oracle runs.

Compares two corpus runs, identifies cops whose FP/FN counts regressed,
links them back to tracker issues, and surfaces likely candidate commits /
merged bot PRs in the commit range.

Optional actions:
- `report` (default): print a markdown report only
- `reopen`: reopen/comment linked tracker issues for regressed cops
- `dispatch-simple`: additionally dispatch simple regressions back into
  `agent-cop-fix` with `backend=auto`
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from datetime import datetime
from pathlib import Path

ISSUE_TITLE_PREFIX = "[cop] "
TRACKER_LABEL = "cop-tracker"
TRACKER_RE = re.compile(r"<!--\s*nitrocop-cop-tracker:\s*(.*?)\s*-->")
PR_ISSUE_RE = re.compile(r"<!--\s*nitrocop-cop-issue:\s*(.*?)\s*-->")
TITLE_RE = re.compile(r"^\[bot\] Fix (?P<cop>.+?)(?: \(retry\))?$")
DEPT_TO_SRC_DIR = {
    "RSpec": "rspec",
    "RSpecRails": "rspec_rails",
    "FactoryBot": "factory_bot",
}


def run(cmd: list[str], *, check: bool = True, cwd: str | None = None) -> str:
    proc = subprocess.run(
        cmd,
        check=check,
        capture_output=True,
        text=True,
        cwd=cwd,
    )
    return proc.stdout.strip()


def run_gh(args: list[str], *, check: bool = True) -> str:
    return run(["gh", *args], check=check)


def artifact_name() -> str:
    return "corpus-report"


def pascal_to_snake(name: str) -> str:
    s = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", name)
    s = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", s)
    return s.lower()


def dept_dir_name(dept: str) -> str:
    return DEPT_TO_SRC_DIR.get(dept, pascal_to_snake(dept))


def parse_cop_name(cop: str) -> tuple[str, str]:
    dept, name = cop.split("/", 1)
    return dept, pascal_to_snake(name)


def cop_paths(cop: str) -> list[str]:
    dept, snake = parse_cop_name(cop)
    dept_dir = dept_dir_name(dept)
    return [
        f"src/cop/{dept_dir}/{snake}.rs",
        f"tests/fixtures/cops/{dept_dir}/{snake}",
    ]


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
    fields = parse_marker_fields(issue.get("body", ""), TRACKER_RE)
    if fields.get("cop"):
        return fields["cop"]
    title = issue.get("title", "")
    if title.startswith(ISSUE_TITLE_PREFIX):
        return title[len(ISSUE_TITLE_PREFIX):].strip()
    return None


def extract_cop_from_pr(pr: dict) -> str | None:
    fields = parse_marker_fields(pr.get("body", ""), PR_ISSUE_RE)
    if fields.get("cop"):
        return fields["cop"]
    match = TITLE_RE.match(pr.get("title", "").strip())
    if match:
        return match.group("cop")
    return None


def issue_difficulty(issue: dict | None) -> str:
    if not issue:
        return "unknown"
    labels = {label["name"] for label in issue.get("labels", [])}
    if "difficulty:simple" in labels:
        return "simple"
    if "difficulty:medium" in labels:
        return "medium"
    if "difficulty:complex" in labels:
        return "complex"
    fields = parse_marker_fields(issue.get("body", ""), TRACKER_RE)
    return fields.get("difficulty", "unknown")


def list_corpus_runs(repo: str) -> list[dict]:
    output = run_gh([
        "api",
        f"repos/{repo}/actions/workflows/corpus-oracle.yml/runs?status=success&per_page=100",
    ])
    runs = json.loads(output).get("workflow_runs", [])
    return [
        {
            "id": run["id"],
            "head_sha": run.get("head_sha", ""),
            "created_at": run.get("created_at", ""),
            "html_url": run.get("html_url", ""),
        }
        for run in runs
    ]


def download_run_corpus(repo: str, run_id: int) -> Path | None:
    tmpdir = tempfile.mkdtemp(prefix=f"regression-{run_id}-")
    result = subprocess.run(
        [
            "gh", "run", "download",
            "--repo", repo,
            str(run_id),
            f"--name={artifact_name()}",
            f"--dir={tmpdir}",
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        shutil.rmtree(tmpdir, ignore_errors=True)
        return None
    path = Path(tmpdir) / "corpus-results.json"
    if not path.exists():
        shutil.rmtree(tmpdir, ignore_errors=True)
        return None
    return path


def resolve_run_pair(
    repo: str,
    *,
    before_run: int | None,
    after_run: int | None,
) -> tuple[dict, dict, Path, Path]:
    runs = list_corpus_runs(repo)
    if not runs:
        raise SystemExit("No successful corpus-oracle runs found")

    chosen: list[tuple[dict, Path]] = []
    explicit = [after_run, before_run]
    if after_run is not None:
        meta = next((run for run in runs if run["id"] == after_run), {"id": after_run, "head_sha": "", "created_at": "", "html_url": f"https://github.com/{repo}/actions/runs/{after_run}"})
        path = download_run_corpus(repo, after_run)
        if path is None:
            raise SystemExit(f"Could not download corpus artifact from run {after_run}")
        chosen.append((meta, path))
    for run in runs:
        if run["id"] in explicit:
            continue
        path = download_run_corpus(repo, run["id"])
        if path is None:
            continue
        chosen.append((run, path))
        if len(chosen) >= (2 if before_run is None else 1):
            break
    if before_run is not None:
        meta = next((run for run in runs if run["id"] == before_run), {"id": before_run, "head_sha": "", "created_at": "", "html_url": f"https://github.com/{repo}/actions/runs/{before_run}"})
        path = download_run_corpus(repo, before_run)
        if path is None:
            raise SystemExit(f"Could not download corpus artifact from run {before_run}")
        if chosen:
            after_meta, after_path = chosen[0]
        else:
            raise SystemExit("Missing after run")
        return meta, after_meta, path, after_path
    if len(chosen) < 2:
        raise SystemExit("Need two successful corpus runs with downloadable artifacts")
    after_meta, after_path = chosen[0]
    before_meta, before_path = chosen[1]
    return before_meta, after_meta, before_path, after_path


def load_corpus(path: Path) -> dict:
    return json.loads(path.read_text())


def by_cop(data: dict) -> dict[str, dict]:
    return {entry["cop"]: entry for entry in data.get("by_cop", [])}


def compute_regressions(before: dict, after: dict) -> list[dict]:
    before_by_cop = by_cop(before)
    after_by_cop = by_cop(after)
    regressions: list[dict] = []
    for cop, after_entry in after_by_cop.items():
        before_entry = before_by_cop.get(cop, {})
        before_fp = before_entry.get("fp", 0)
        before_fn = before_entry.get("fn", 0)
        after_fp = after_entry.get("fp", 0)
        after_fn = after_entry.get("fn", 0)
        delta_fp = after_fp - before_fp
        delta_fn = after_fn - before_fn
        if delta_fp <= 0 and delta_fn <= 0:
            continue
        regressions.append(
            {
                "cop": cop,
                "before_fp": before_fp,
                "before_fn": before_fn,
                "after_fp": after_fp,
                "after_fn": after_fn,
                "delta_fp": delta_fp,
                "delta_fn": delta_fn,
                "delta_total": delta_fp + delta_fn,
                "matches": after_entry.get("matches", before_entry.get("matches", 0)),
            }
        )
    regressions.sort(key=lambda item: (-item["delta_total"], item["cop"]))
    return regressions


def list_tracker_issues(repo: str) -> dict[str, dict]:
    output = run_gh([
        "issue", "list",
        "--repo", repo,
        "--state", "all",
        "--label", TRACKER_LABEL,
        "--limit", "500",
        "--json", "number,title,url,state,body,labels",
    ])
    issues = json.loads(output) if output else []
    return {
        cop: issue
        for issue in issues
        if (cop := extract_cop_from_issue(issue))
    }


def list_merged_agent_fix_prs(repo: str) -> list[dict]:
    output = run_gh([
        "pr", "list",
        "--repo", repo,
        "--state", "merged",
        "--label", "agent-fix",
        "--limit", "500",
        "--json", "number,title,url,body,mergedAt",
    ])
    return json.loads(output) if output else []


def parse_iso8601(value: str) -> datetime:
    return datetime.fromisoformat(value.replace("Z", "+00:00"))


def prs_between_runs(prs: list[dict], cop: str, before_meta: dict, after_meta: dict) -> list[dict]:
    before_dt = parse_iso8601(before_meta["created_at"])
    after_dt = parse_iso8601(after_meta["created_at"])
    candidates = []
    for pr in prs:
        merged_at = pr.get("mergedAt")
        if not merged_at:
            continue
        merged_dt = parse_iso8601(merged_at)
        if not (before_dt < merged_dt <= after_dt):
            continue
        if extract_cop_from_pr(pr) == cop:
            candidates.append(pr)
    return candidates


def commits_touching_cop(before_sha: str, after_sha: str, cop: str) -> list[dict]:
    paths = cop_paths(cop)
    output = run(
        ["git", "log", "--format=%H%x09%s", f"{before_sha}..{after_sha}", "--", *paths],
        check=False,
    )
    commits = []
    for line in output.splitlines():
        if "\t" not in line:
            continue
        sha, subject = line.split("\t", 1)
        commits.append({"sha": sha, "subject": subject})
    return commits


def recommended_action(regression: dict) -> str:
    if len(regression["pr_candidates"]) == 1:
        return "strong_revert_candidate"
    if regression["difficulty"] == "simple":
        return "dispatch_repair"
    return "manual_investigation"


def build_comment(regression: dict, before_meta: dict, after_meta: dict) -> str:
    lines = [
        f"Regression detected for `{regression['cop']}` in the latest corpus comparison.",
        "",
        f"- Before run: [#{before_meta['id']}]({before_meta['html_url']})",
        f"- After run: [#{after_meta['id']}]({after_meta['html_url']})",
        f"- FP: {regression['before_fp']} -> {regression['after_fp']} (`+{regression['delta_fp']}`)",
        f"- FN: {regression['before_fn']} -> {regression['after_fn']} (`+{regression['delta_fn']}`)",
        f"- Suggested action: `{regression['action']}`",
    ]
    if regression["pr_candidates"]:
        lines.append("")
        lines.append("Candidate merged bot PRs in this range:")
        for pr in regression["pr_candidates"]:
            lines.append(f"- PR #{pr['number']}: {pr['url']}")
    if regression["commit_candidates"]:
        lines.append("")
        lines.append("Candidate commits touching this cop:")
        for commit in regression["commit_candidates"][:5]:
            lines.append(f"- `{commit['sha'][:12]}` {commit['subject']}")
    return "\n".join(lines)


def reopen_and_comment_issue(repo: str, issue: dict, comment: str) -> None:
    if issue["state"] == "CLOSED":
        subprocess.run(
            ["gh", "issue", "reopen", str(issue["number"]), "--repo", repo],
            capture_output=True,
            text=True,
            check=True,
        )
    subprocess.run(
        ["gh", "issue", "comment", str(issue["number"]), "--repo", repo, "--body", comment],
        capture_output=True,
        text=True,
        check=True,
    )
    subprocess.run(
        [
            "gh", "issue", "edit", str(issue["number"]),
            "--repo", repo,
            "--remove-label", "state:pr-open,state:backlog",
            "--add-label", "state:blocked",
        ],
        capture_output=True,
        text=True,
        check=False,
    )


def dispatch_simple_repair(repo: str, issue: dict, regression: dict) -> None:
    extra_context = (
        f"Main regressed for {regression['cop']} between corpus runs {regression['before_run_id']} "
        f"and {regression['after_run_id']}. "
        "Investigate the regression introduced after merge and keep the fix narrow."
    )
    cmd = [
        "gh", "workflow", "run", "agent-cop-fix.yml",
        "--repo", repo,
        "-f", f"cop={regression['cop']}",
        "-f", "backend=auto",
        "-f", "mode=retry",
        "-f", f"issue_number={issue['number']}",
        "-f", f"extra_context={extra_context}",
    ]
    # Use DISPATCH_TOKEN (GITHUB_TOKEN with actions:write) for workflow dispatch,
    # since the default GH_TOKEN is an app token that lacks actions permission.
    env = {**os.environ}
    dispatch_token = os.environ.get("DISPATCH_TOKEN")
    if dispatch_token:
        env["GH_TOKEN"] = dispatch_token
    proc = subprocess.run(cmd, capture_output=True, text=True, env=env)
    if proc.returncode != 0:
        print(f"ERROR dispatching repair for {regression['cop']}: {proc.stderr.strip()}", file=sys.stderr)
        proc.check_returncode()


def render_report(before_meta: dict, after_meta: dict, regressions: list[dict]) -> str:
    lines = [
        "# Regression Investigation",
        "",
        f"- Before run: [#{before_meta['id']}]({before_meta['html_url']}) `{before_meta['head_sha']}`",
        f"- After run: [#{after_meta['id']}]({after_meta['html_url']}) `{after_meta['head_sha']}`",
        f"- Regressed cops: `{len(regressions)}`",
        "",
    ]
    if not regressions:
        lines.append("No FP/FN count regressions found between these corpus runs.")
        return "\n".join(lines)

    for regression in regressions:
        lines.extend([
            f"## {regression['cop']}",
            "",
            f"- FP: {regression['before_fp']} -> {regression['after_fp']} (`+{regression['delta_fp']}`)",
            f"- FN: {regression['before_fn']} -> {regression['after_fn']} (`+{regression['delta_fn']}`)",
            f"- Difficulty: `{regression['difficulty']}`",
            f"- Issue: {regression['issue_ref']}",
            f"- Suggested action: `{regression['action']}`",
        ])
        if regression["pr_candidates"]:
            lines.append("- Candidate bot PRs:")
            for pr in regression["pr_candidates"]:
                lines.append(f"  - PR #{pr['number']}: {pr['url']}")
        if regression["commit_candidates"]:
            lines.append("- Candidate commits touching this cop:")
            for commit in regression["commit_candidates"][:5]:
                lines.append(f"  - `{commit['sha'][:12]}` {commit['subject']}")
        lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="Investigate regressions between corpus oracle runs")
    parser.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY", ""), help="GitHub repo owner/name")
    parser.add_argument("--before-run", type=int, help="Older corpus-oracle run ID")
    parser.add_argument("--after-run", type=int, help="Newer corpus-oracle run ID")
    parser.add_argument("--action", choices=["report", "reopen", "dispatch-simple"], default="report")
    parser.add_argument("--output", type=Path, help="Write markdown report here")
    parser.add_argument("--json-output", type=Path, help="Write machine-readable JSON here")
    args = parser.parse_args()

    if not args.repo:
        raise SystemExit("--repo or GITHUB_REPOSITORY is required")

    before_meta, after_meta, before_path, after_path = resolve_run_pair(
        args.repo,
        before_run=args.before_run,
        after_run=args.after_run,
    )
    regressions = compute_regressions(load_corpus(before_path), load_corpus(after_path))
    issues = list_tracker_issues(args.repo)
    merged_prs = list_merged_agent_fix_prs(args.repo)

    for regression in regressions:
        cop = regression["cop"]
        issue = issues.get(cop)
        regression["issue_number"] = issue["number"] if issue else None
        regression["issue_ref"] = f"#{issue['number']}" if issue else "(no tracker issue)"
        regression["difficulty"] = issue_difficulty(issue)
        regression["pr_candidates"] = prs_between_runs(merged_prs, cop, before_meta, after_meta)
        regression["commit_candidates"] = (
            commits_touching_cop(before_meta["head_sha"], after_meta["head_sha"], cop)
            if before_meta["head_sha"] and after_meta["head_sha"]
            else []
        )
        regression["before_run_id"] = before_meta["id"]
        regression["after_run_id"] = after_meta["id"]
        regression["action"] = recommended_action(regression)

        if args.action in {"reopen", "dispatch-simple"} and issue:
            reopen_and_comment_issue(
                args.repo,
                issue,
                build_comment(regression, before_meta, after_meta),
            )
            if args.action == "dispatch-simple" and regression["action"] == "dispatch_repair":
                dispatch_simple_repair(args.repo, issue, regression)

    report = render_report(before_meta, after_meta, regressions)
    if args.output:
        args.output.write_text(report)
    else:
        print(report)

    if args.json_output:
        args.json_output.write_text(
            json.dumps(
                {
                    "before_run": before_meta,
                    "after_run": after_meta,
                    "regressions": regressions,
                },
                indent=2,
            )
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
