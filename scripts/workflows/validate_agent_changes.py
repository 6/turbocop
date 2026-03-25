#!/usr/bin/env python3
"""Validate workflow agent edits against deterministic path allowlists."""

from __future__ import annotations

import argparse
import fnmatch
import subprocess
from pathlib import Path

ALLOWLISTS = {
    "agent-cop-fix": [
        "src/cop/**",
        "tests/fixtures/cops/**",
    ],
    "repair-rust-test": [
        "src/**",
        "tests/**",
        "Cargo.toml",
        "Cargo.lock",
        "build.rs",
        "bench/**",
    ],
    "repair-python-workflow": [
        "scripts/**",
        "tests/python/**",
        ".github/workflows/**",
        "pyproject.toml",
        "mise.toml",
    ],
    "repair-cop-check": [
        "src/cop/**",
        "tests/fixtures/cops/**",
        "tests/integration.rs",
    ],
    "repair-smoke": [
        "src/**",
        "tests/**",
        "bench/**",
        "scripts/**",
    ],
}


def run_git(repo_root: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", *args],
        cwd=str(repo_root),
        text=True,
        capture_output=True,
        check=True,
    )
    return result.stdout.strip()


def changed_files(repo_root: Path, base_ref: str) -> list[str]:
    tracked = run_git(repo_root, "diff", "--name-only", "--diff-filter=ACDMRTUXB", base_ref)
    untracked = run_git(repo_root, "ls-files", "--others", "--exclude-standard")
    names = {
        line.strip()
        for line in [*tracked.splitlines(), *untracked.splitlines()]
        if line.strip()
    }
    return sorted(names)


def is_allowed(path: str, patterns: list[str]) -> bool:
    return any(fnmatch.fnmatchcase(path, pattern) for pattern in patterns)


def validate(profile: str, files: list[str]) -> tuple[list[str], list[str]]:
    try:
        patterns = ALLOWLISTS[profile]
    except KeyError as exc:
        raise SystemExit(f"unknown validation profile: {profile}") from exc

    allowed: list[str] = []
    disallowed: list[str] = []
    for path in files:
        if is_allowed(path, patterns):
            allowed.append(path)
        else:
            disallowed.append(path)
    return allowed, disallowed


def render_report(profile: str, allowed: list[str], disallowed: list[str]) -> str:
    patterns = ALLOWLISTS[profile]
    lines = [
        "## Agent File Scope",
        "",
        f"- Profile: `{profile}`",
        "",
        "Allowed path patterns:",
    ]
    lines.extend(f"- `{pattern}`" for pattern in patterns)
    lines.append("")

    if allowed:
        lines.append("Changed files within scope:")
        lines.extend(f"- `{path}`" for path in allowed)
        lines.append("")

    if disallowed:
        lines.append("Disallowed changed files:")
        lines.extend(f"- `{path}`" for path in disallowed)
        lines.append("")
    else:
        lines.append("All changed files are within the allowed scope.")
        lines.append("")

    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate workflow agent edits against path allowlists")
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--base-ref", required=True)
    parser.add_argument("--profile", choices=sorted(ALLOWLISTS), required=True)
    parser.add_argument("--report-out", type=Path)
    args = parser.parse_args()

    repo_root = args.repo_root.resolve()
    files = changed_files(repo_root, args.base_ref)
    allowed, disallowed = validate(args.profile, files)
    report = render_report(args.profile, allowed, disallowed)

    if args.report_out:
        args.report_out.parent.mkdir(parents=True, exist_ok=True)
        args.report_out.write_text(report + "\n")

    print(f"valid={'true' if not disallowed else 'false'}")
    print(f"profile={args.profile}")
    print(f"changed_count={len(files)}")
    print(f"allowed_count={len(allowed)}")
    print(f"disallowed_count={len(disallowed)}")
    print(f"changed_files={','.join(files)}")
    print(f"disallowed_files={','.join(disallowed)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
