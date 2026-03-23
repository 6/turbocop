#!/usr/bin/env python3
"""Capture and report extra git refs/worktrees created during agent runs."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from pathlib import Path


def run_git(repo_root: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        cwd=str(repo_root),
        text=True,
        capture_output=True,
        check=check,
    )


def git_stdout(repo_root: Path, *args: str, check: bool = True) -> str:
    return run_git(repo_root, *args, check=check).stdout.strip()


def capture_snapshot(repo_root: Path) -> dict[str, object]:
    head = git_stdout(repo_root, "rev-parse", "HEAD")

    head_ref_result = run_git(repo_root, "symbolic-ref", "-q", "HEAD", check=False)
    head_ref = head_ref_result.stdout.strip() or None

    refs: dict[str, str] = {}
    refs_output = git_stdout(
        repo_root,
        "for-each-ref",
        "--format=%(refname)\t%(objectname)",
        "refs/heads",
    )
    for line in refs_output.splitlines():
        if not line.strip():
            continue
        refname, sha = line.split("\t", 1)
        refs[refname] = sha

    worktrees: list[dict[str, object]] = []
    current: dict[str, object] = {}
    worktree_output = git_stdout(repo_root, "worktree", "list", "--porcelain")
    for line in worktree_output.splitlines():
        if not line.strip():
            if current:
                worktrees.append(current)
                current = {}
            continue

        key, _, value = line.partition(" ")
        if key == "worktree":
            current["path"] = value
        elif key == "HEAD":
            current["head"] = value
        elif key == "branch":
            current["branch"] = value
        elif key == "detached":
            current["detached"] = True
        elif key == "bare":
            current["bare"] = True
        elif key == "locked":
            current["locked"] = value or True
        elif key == "prunable":
            current["prunable"] = value or True

    if current:
        worktrees.append(current)

    return {
        "repo_root": str(repo_root.resolve()),
        "head": head,
        "head_ref": head_ref,
        "refs": refs,
        "worktrees": worktrees,
    }


def write_json(path: Path, payload: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")


def load_json(path: Path) -> dict[str, object]:
    return json.loads(path.read_text())


def slugify(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9._-]+", "_", value.strip("/")) or "item"


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def command_output(repo_root: Path, *args: str) -> str:
    result = run_git(repo_root, *args, check=False)
    return result.stdout


def summarize_worktree(
    out_dir: Path,
    label: str,
    path: Path,
    branch: str | None,
    old_head: str | None,
    new_head: str,
) -> list[str]:
    lines: list[str] = []
    base = out_dir / slugify(label)

    if not path.exists():
        lines.append(f"- `{label}`: path `{path}` no longer exists; captured HEAD `{new_head}` only")
        write_text(base.with_suffix(".txt"), f"path={path}\nbranch={branch or '(detached)'}\nhead={new_head}\n")
        return lines

    status = command_output(path, "status", "--short")
    log = command_output(path, "log", "--oneline", "--decorate", "-n", "20")
    write_text(base.with_suffix(".status.txt"), status)
    write_text(base.with_suffix(".log.txt"), log)
    write_text(
        base.with_suffix(".meta.txt"),
        f"path={path}\nbranch={branch or '(detached)'}\nold_head={old_head or ''}\nhead={new_head}\n",
    )

    if old_head:
        diff_stat = command_output(path, "diff", "--stat", f"{old_head}..{new_head}")
        diff_patch = command_output(path, "diff", "--binary", f"{old_head}..{new_head}")
        if diff_stat.strip():
            write_text(base.with_suffix(".diff.stat"), diff_stat)
        if diff_patch.strip():
            write_text(base.with_suffix(".diff.patch"), diff_patch)
    else:
        show_stat = command_output(path, "show", "--stat", "--summary", new_head)
        if show_stat.strip():
            write_text(base.with_suffix(".show.txt"), show_stat)

    lines.append(
        f"- `{label}`: branch `{branch or '(detached)'}`, HEAD `{new_head}`"
        + (f", previous HEAD `{old_head}`" if old_head else "")
    )
    return lines


def summarize_ref(
    repo_root: Path,
    out_dir: Path,
    refname: str,
    old_sha: str | None,
    new_sha: str,
) -> str:
    base = out_dir / slugify(refname.replace("refs/heads/", "ref-"))
    write_text(base.with_suffix(".meta.txt"), f"ref={refname}\nold_sha={old_sha or ''}\nnew_sha={new_sha}\n")

    if old_sha:
        diff_stat = command_output(repo_root, "diff", "--stat", f"{old_sha}..{new_sha}")
        diff_patch = command_output(repo_root, "diff", "--binary", f"{old_sha}..{new_sha}")
        if diff_stat.strip():
            write_text(base.with_suffix(".diff.stat"), diff_stat)
        if diff_patch.strip():
            write_text(base.with_suffix(".diff.patch"), diff_patch)
        return f"- `{refname}` advanced from `{old_sha}` to `{new_sha}`"

    show_stat = command_output(repo_root, "show", "--stat", "--summary", new_sha)
    if show_stat.strip():
        write_text(base.with_suffix(".show.txt"), show_stat)
    return f"- `{refname}` created at `{new_sha}`"


def generate_report(before_path: Path, after_path: Path, out_dir: Path) -> None:
    before = load_json(before_path)
    after = load_json(after_path)

    repo_root = Path(str(after["repo_root"]))
    current_ref = after.get("head_ref")
    current_repo_root = str(repo_root.resolve())

    out_dir.mkdir(parents=True, exist_ok=True)

    before_refs = dict(before["refs"])
    after_refs = dict(after["refs"])

    ref_lines: list[str] = []
    for refname, new_sha in sorted(after_refs.items()):
        if current_ref and refname == current_ref:
            continue
        old_sha = before_refs.get(refname)
        if old_sha == new_sha:
            continue
        ref_lines.append(summarize_ref(repo_root, out_dir, refname, old_sha, new_sha))

    before_worktrees = {
        str(item["path"]): item
        for item in before["worktrees"]
        if str(item["path"]) != current_repo_root
    }
    after_worktrees = {
        str(item["path"]): item
        for item in after["worktrees"]
        if str(item["path"]) != current_repo_root
    }

    worktree_lines: list[str] = []
    for path_str, info in sorted(after_worktrees.items()):
        before_info = before_worktrees.get(path_str)
        old_head = None
        if before_info:
            old_head = str(before_info.get("head") or "")
            if old_head == str(info.get("head") or "") and before_info.get("branch") == info.get("branch"):
                continue
        path = Path(path_str)
        label = f"worktree:{path_str}"
        worktree_lines.extend(
            summarize_worktree(
                out_dir,
                label,
                path,
                info.get("branch"),
                old_head or None,
                str(info.get("head") or ""),
            )
        )

    removed_worktrees = sorted(set(before_worktrees) - set(after_worktrees))
    report_lines = [
        "# Additional Git Activity",
        "",
        "This report covers refs/worktrees outside the current checkout. The current checkout diff is captured separately.",
        "",
    ]

    if ref_lines:
        report_lines.extend(["## Extra Local Refs", "", *ref_lines, ""])

    if worktree_lines:
        report_lines.extend(["## Extra Worktrees", "", *worktree_lines, ""])

    if removed_worktrees:
        report_lines.extend(["## Removed Worktrees", "", *[f"- `{path}`" for path in removed_worktrees], ""])

    if not ref_lines and not worktree_lines and not removed_worktrees:
        report_lines.extend(["No extra refs or worktrees detected outside the current checkout.", ""])

    write_text(out_dir / "report.md", "\n".join(report_lines).rstrip() + "\n")
    write_json(
        out_dir / "report.json",
        {
            "extra_refs": ref_lines,
            "extra_worktrees": worktree_lines,
            "removed_worktrees": removed_worktrees,
        },
    )


def main() -> int:
    parser = argparse.ArgumentParser(description="Capture/report extra git activity during agent runs")
    subparsers = parser.add_subparsers(dest="command", required=True)

    capture_parser = subparsers.add_parser("capture", help="Capture current refs/worktrees to JSON")
    capture_parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    capture_parser.add_argument("--output", type=Path, required=True)

    report_parser = subparsers.add_parser("report", help="Generate artifact files from before/after snapshots")
    report_parser.add_argument("--before", type=Path, required=True)
    report_parser.add_argument("--after", type=Path, required=True)
    report_parser.add_argument("--out-dir", type=Path, required=True)

    args = parser.parse_args()

    if args.command == "capture":
        write_json(args.output, capture_snapshot(args.repo_root.resolve()))
        return 0

    if args.command == "report":
        generate_report(args.before, args.after, args.out_dir)
        return 0

    return 1


if __name__ == "__main__":
    raise SystemExit(main())
