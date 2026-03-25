#!/usr/bin/env python3
"""Generate a per-repo RuboCop config overlay with file exclusions.

Reads repo_excludes.json and, if the given repo ID has exclusions,
writes a temporary YAML config that inherits from the base config
and adds the extra Exclude entries. Prints the path to use.

Usage:
    python3 gen_repo_config.py <repo_id> <base_config> <repo_dir>

If the repo has no exclusions, prints the base config path unchanged.
"""
import json
import sys
import tempfile
from pathlib import Path

EXCLUDES_PATH = Path(__file__).parent / "repo_excludes.json"


def overlay_config_path(repo_id: str) -> Path:
    """Return a stable temp path isolated from repo trees.

    Corpus runs pass the overlay with `--config`. Nitrocop discovers nested
    `.rubocop.yml` files under the config file's parent directory, so writing
    overlays directly under `/tmp` can accidentally pull in unrelated repo
    configs from sibling corpus clones. Keep each overlay in its own clean
    directory instead.
    """
    overlay_dir = Path(tempfile.gettempdir()) / "nitrocop_corpus_configs" / repo_id
    overlay_dir.mkdir(parents=True, exist_ok=True)
    return overlay_dir / "corpus_config.yml"


def resolve_repo_config(
    repo_id: str,
    base_config: str,
    repo_dir: str,
    excludes_path: Path = EXCLUDES_PATH,
) -> str:
    """Return the effective config path for a corpus repo."""
    if not excludes_path.exists():
        return base_config

    with excludes_path.open() as f:
        excludes = json.load(f)

    entry = excludes.get(repo_id)
    if not entry or not entry.get("exclude"):
        return base_config

    # Generate a temp YAML that inherits from the base config and adds excludes.
    # Use absolute paths since the overlay config lives outside the repo.
    abs_base = str(Path(base_config).resolve())
    abs_repo = str(Path(repo_dir).resolve())

    # RuboCop merges AllCops/Exclude by default (union), so we only need
    # to list the additional excludes here.
    lines = [f"inherit_from: {abs_base}", "", "AllCops:", "  Exclude:"]
    for pattern in entry["exclude"]:
        lines.append(f'    - "{abs_repo}/{pattern}"')

    tmp_path = overlay_config_path(repo_id)
    tmp_path.write_text("\n".join(lines) + "\n")
    return str(tmp_path)


def main():
    if len(sys.argv) != 4:
        print(f"Usage: {sys.argv[0]} <repo_id> <base_config> <repo_dir>", file=sys.stderr)
        sys.exit(1)

    repo_id, base_config, repo_dir = sys.argv[1], sys.argv[2], sys.argv[3]

    print(resolve_repo_config(repo_id, base_config, repo_dir))


if __name__ == "__main__":
    main()
