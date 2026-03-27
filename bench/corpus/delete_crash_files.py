#!/usr/bin/env python3
"""Delete known crash-prone files from a corpus repo before linting.

rescue_parser_crashes.rb prevents crashes from killing the run, but crashy
files produce 0-offense entries that create phantom FN. Deleting them before
running is simpler than config overlays.

Usage:
    python3 delete_crash_files.py <repo_id> <repo_dir>
"""

import shutil
import sys
from pathlib import Path

# repo_id -> list of paths to delete (relative to repo root).
# Directories are removed recursively.
CRASH_FILES: dict[str, list[str]] = {
    "jruby__jruby__0303464": [
        "bench/compiler/bench_compilation.rb",
        "test/jruby/test_regexp.rb",
    ],
    "ruby__logger__00796ec": [
        "test/",
    ],
    "infochimps-labs__wukong__437eff1": [
        "old/",
    ],
}


def delete_crash_files(repo_id: str, repo_dir: str) -> int:
    """Delete crash-prone files for the given repo. Returns count of deletions."""
    paths = CRASH_FILES.get(repo_id, [])
    deleted = 0
    for rel in paths:
        target = Path(repo_dir) / rel
        if target.is_dir():
            shutil.rmtree(target, ignore_errors=True)
            deleted += 1
        elif target.exists():
            target.unlink()
            deleted += 1
    return deleted


def main() -> int:
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <repo_id> <repo_dir>", file=sys.stderr)
        return 1
    repo_id, repo_dir = sys.argv[1], sys.argv[2]
    deleted = delete_crash_files(repo_id, repo_dir)
    if deleted:
        print(f"Deleted {deleted} crash-prone path(s) from {repo_id}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
