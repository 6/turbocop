#!/usr/bin/env python3
"""Render commit and PR metadata for corpus-oracle updates."""

from __future__ import annotations

import argparse
import json
import subprocess
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Artifact:
    path: str
    label: str
    detail: str


STANDARD_ARTIFACTS = (
    Artifact(
        "src/resources/tiers.json",
        "tiers.json",
        "Stable and preview tiers regenerated from the latest standard corpus results.",
    ),
    Artifact(
        "README.md",
        "README",
        "Top-level conformance summary refreshed from the latest standard corpus results.",
    ),
    Artifact(
        "docs/corpus.md",
        "standard corpus report",
        "Full standard corpus report regenerated.",
    ),
)

EXTENDED_ARTIFACTS = (
    Artifact(
        "docs/corpus_extended.md",
        "extended corpus report",
        "Full extended corpus report regenerated.",
    ),
    Artifact(
        "docs/cop_coverage.md",
        "cop coverage report",
        "Cop coverage summary refreshed from the latest extended corpus results.",
    ),
)


def run_git(repo_root: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", *args],
        cwd=str(repo_root),
        text=True,
        capture_output=True,
        check=True,
    )
    return result.stdout


def staged_files(repo_root: Path) -> list[str]:
    output = run_git(repo_root, "diff", "--cached", "--name-only")
    return [line.strip() for line in output.splitlines() if line.strip()]


def human_join(items: list[str]) -> str:
    if not items:
        return ""
    if len(items) == 1:
        return items[0]
    if len(items) == 2:
        return f"{items[0]} and {items[1]}"
    return f"{', '.join(items[:-1])}, and {items[-1]}"


def catalog_for(corpus_size: str) -> tuple[Artifact, ...]:
    if corpus_size == "standard":
        return STANDARD_ARTIFACTS
    if corpus_size == "extended":
        return EXTENDED_ARTIFACTS
    raise ValueError(f"Unsupported corpus size: {corpus_size}")


def select_artifacts(corpus_size: str, changed_files: list[str]) -> list[Artifact]:
    catalog = catalog_for(corpus_size)
    known = {artifact.path: artifact for artifact in catalog}
    selected = [artifact for artifact in catalog if artifact.path in changed_files]

    for path in changed_files:
        if path in known:
            continue
        selected.append(
            Artifact(
                path=path,
                label=Path(path).name,
                detail="Generated corpus-oracle artifact refreshed.",
            )
        )
    return selected


def build_metadata(
    *,
    corpus_size: str,
    repo_filter: str,
    run_number: str,
    run_url: str,
    changed_files: list[str],
) -> dict[str, str]:
    artifacts = select_artifacts(corpus_size, changed_files)
    if artifacts:
        artifact_summary = human_join([artifact.label for artifact in artifacts])
        changed_artifacts_cell = human_join([f"`{artifact.label}`" for artifact in artifacts])
    else:
        artifact_summary = f"{corpus_size} corpus artifacts"
        changed_artifacts_cell = f"`{artifact_summary}`"

    subject = f"Corpus oracle ({corpus_size}): refresh {artifact_summary}"
    scope = "all repos" if repo_filter == "all" else repo_filter

    lines = [
        "## Summary",
        "",
        f"Automated corpus oracle refresh from [run #{run_number}]({run_url}).",
        "",
        "| | |",
        "|---|---|",
        f"| **Corpus** | `{corpus_size}` |",
        f"| **Scope** | `{scope}` |",
        f"| **Changed artifacts** | {changed_artifacts_cell} |",
        "| **CI** | intentionally skipped via `[skip ci]` |",
        "",
        "## Refreshed Artifacts",
        "",
    ]

    if artifacts:
        for artifact in artifacts:
            lines.append(f"- `{artifact.path}`: {artifact.detail}")
    else:
        lines.append("- No staged corpus artifacts were detected.")

    lines.extend([
        "",
        "This PR only updates generated corpus artifacts.",
    ])

    return {
        "commit_message": f"[skip ci] {subject}",
        "pr_title": f"[skip ci] {subject}",
        "pr_body": "\n".join(lines),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Render metadata for corpus-oracle PRs")
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--corpus-size", choices=["standard", "extended"], required=True)
    parser.add_argument("--repo-filter", default="all")
    parser.add_argument("--run-number", required=True)
    parser.add_argument("--run-url", required=True)
    args = parser.parse_args()

    metadata = build_metadata(
        corpus_size=args.corpus_size,
        repo_filter=args.repo_filter,
        run_number=args.run_number,
        run_url=args.run_url,
        changed_files=staged_files(args.repo_root.resolve()),
    )
    print(json.dumps(metadata))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
