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


def main():
    if len(sys.argv) != 4:
        print(f"Usage: {sys.argv[0]} <repo_id> <base_config> <repo_dir>", file=sys.stderr)
        sys.exit(1)

    repo_id, base_config, repo_dir = sys.argv[1], sys.argv[2], sys.argv[3]

    if not EXCLUDES_PATH.exists():
        print(base_config)
        return

    with open(EXCLUDES_PATH) as f:
        excludes = json.load(f)

    entry = excludes.get(repo_id)
    repo_patterns = entry.get("exclude", []) if entry else []

    # Exclude vendor-ish directories with absolute paths. The baseline config
    # has these as relative patterns, but they resolve relative to the config
    # file's parent (bench/corpus/), not the repo dir. The overlay converts
    # them to absolute paths so they actually match.
    global_patterns = [
        "vendor/**/*",
        "vendor*/**/*",
        "_vendor/**/*",
        "cookbooks/**/*",
    ]

    all_patterns = global_patterns + repo_patterns
    if not all_patterns:
        print(base_config)
        return

    # Generate a temp YAML that inherits from the base config and adds excludes.
    # Keep the overlay in its own temp subdirectory instead of directly under
    # /tmp so explicit --config runs do not recurse into cloned repos under
    # /tmp/nitrocop_cop_check_*/ and load vendored .rubocop.yml overrides.
    abs_base = str(Path(base_config).resolve())
    abs_repo = str(Path(repo_dir).resolve())

    # RuboCop merges AllCops/Exclude by default (union), so we only need
    # to list the additional excludes here.
    lines = [f"inherit_from: {abs_base}", "", "AllCops:", "  Exclude:"]
    for pattern in all_patterns:
        lines.append(f'    - "{abs_repo}/{pattern}"')

    tmp_dir = Path(tempfile.gettempdir()) / "nitrocop_corpus_configs"
    tmp_dir.mkdir(parents=True, exist_ok=True)
    tmp_path = tmp_dir / f"corpus_config_{repo_id}.yml"
    tmp_path.write_text("\n".join(lines) + "\n")
    print(str(tmp_path))


if __name__ == "__main__":
    main()
