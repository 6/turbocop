#!/usr/bin/env python3
"""Tests for resolve_symlink_paths.py."""

import json
import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).parents[2] / "bench" / "corpus" / "resolve_symlink_paths.py"

# Import the module directly for unit tests
sys.path.insert(0, str(SCRIPT.parent))
import resolve_symlink_paths


def make_repo_with_symlink(tmp: Path) -> tuple[Path, Path]:
    """Create a directory structure with a symlink like pagy's docs/gem -> ../gem.

    Returns (canonical_file, symlink_file) paths to the same physical file.
    """
    repo = tmp / "repo"
    gem_dir = repo / "gem"
    docs_dir = repo / "docs"
    gem_dir.mkdir(parents=True)
    docs_dir.mkdir(parents=True)

    # Create file at canonical path
    gemspec = gem_dir / "foo.gemspec"
    gemspec.write_text("spec.name = 'foo'\n")

    # Create symlink: docs/gem -> ../gem
    symlink = docs_dir / "gem"
    symlink.symlink_to("../gem")

    return gemspec, symlink / "foo.gemspec"


# ── nitrocop format ─────────────────────────────────────────────


def test_nitrocop_resolves_symlink_path():
    """Offenses reported via symlink path get resolved to canonical path."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        canonical, via_symlink = make_repo_with_symlink(tmp)

        json_path = tmp / "nitrocop.json"
        json_path.write_text(json.dumps({
            "offenses": [
                {"path": str(via_symlink), "line": 1, "cop_name": "Style/Test"},
            ]
        }))

        resolve_symlink_paths.resolve_nitrocop_json(str(json_path))
        data = json.loads(json_path.read_text())

        assert len(data["offenses"]) == 1
        assert data["offenses"][0]["path"] == str(canonical.resolve())


def test_nitrocop_deduplicates_same_file_different_paths():
    """Two offenses on the same file via different paths collapse to one."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        canonical, via_symlink = make_repo_with_symlink(tmp)

        json_path = tmp / "nitrocop.json"
        json_path.write_text(json.dumps({
            "offenses": [
                {"path": str(canonical), "line": 1, "cop_name": "Style/Test"},
                {"path": str(via_symlink), "line": 1, "cop_name": "Style/Test"},
            ]
        }))

        resolve_symlink_paths.resolve_nitrocop_json(str(json_path))
        data = json.loads(json_path.read_text())

        assert len(data["offenses"]) == 1


def test_nitrocop_keeps_different_offenses_on_same_file():
    """Different offenses (different line/cop) on same resolved file are kept."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        canonical, via_symlink = make_repo_with_symlink(tmp)

        json_path = tmp / "nitrocop.json"
        json_path.write_text(json.dumps({
            "offenses": [
                {"path": str(canonical), "line": 1, "cop_name": "Style/A"},
                {"path": str(via_symlink), "line": 2, "cop_name": "Style/B"},
            ]
        }))

        resolve_symlink_paths.resolve_nitrocop_json(str(json_path))
        data = json.loads(json_path.read_text())

        assert len(data["offenses"]) == 2


def test_nitrocop_preserves_nonexistent_paths():
    """Paths that don't exist on disk are left unchanged."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        json_path = tmp / "nitrocop.json"
        json_path.write_text(json.dumps({
            "offenses": [
                {"path": "/no/such/file.rb", "line": 1, "cop_name": "Style/Test"},
            ]
        }))

        resolve_symlink_paths.resolve_nitrocop_json(str(json_path))
        data = json.loads(json_path.read_text())

        assert len(data["offenses"]) == 1
        assert data["offenses"][0]["path"] == "/no/such/file.rb"


# ── RuboCop format ──────────────────────────────────────────────


def test_rubocop_merges_symlink_duplicate_files():
    """Two file entries for the same physical file merge into one."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        canonical, via_symlink = make_repo_with_symlink(tmp)

        json_path = tmp / "rubocop.json"
        json_path.write_text(json.dumps({
            "files": [
                {
                    "path": str(canonical),
                    "offenses": [
                        {"location": {"line": 1}, "cop_name": "Style/A"},
                    ]
                },
                {
                    "path": str(via_symlink),
                    "offenses": [
                        {"location": {"line": 1}, "cop_name": "Style/A"},
                    ]
                },
            ],
            "summary": {"inspected_file_count": 2}
        }))

        resolve_symlink_paths.resolve_rubocop_json(str(json_path))
        data = json.loads(json_path.read_text())

        assert len(data["files"]) == 1
        assert data["files"][0]["path"] == str(canonical.resolve())
        # Duplicate offense is deduplicated
        assert len(data["files"][0]["offenses"]) == 1


def test_rubocop_merges_different_offenses_from_symlink_paths():
    """Different offenses from symlink paths merge into the canonical file entry."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        canonical, via_symlink = make_repo_with_symlink(tmp)

        json_path = tmp / "rubocop.json"
        json_path.write_text(json.dumps({
            "files": [
                {
                    "path": str(canonical),
                    "offenses": [
                        {"location": {"line": 1}, "cop_name": "Style/A"},
                    ]
                },
                {
                    "path": str(via_symlink),
                    "offenses": [
                        {"location": {"line": 2}, "cop_name": "Style/B"},
                    ]
                },
            ],
            "summary": {"inspected_file_count": 2}
        }))

        resolve_symlink_paths.resolve_rubocop_json(str(json_path))
        data = json.loads(json_path.read_text())

        assert len(data["files"]) == 1
        assert len(data["files"][0]["offenses"]) == 2


def test_rubocop_preserves_unrelated_files():
    """Files that don't share a resolved path are not merged."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        file_a = tmp / "a.rb"
        file_b = tmp / "b.rb"
        file_a.write_text("")
        file_b.write_text("")

        json_path = tmp / "rubocop.json"
        json_path.write_text(json.dumps({
            "files": [
                {"path": str(file_a), "offenses": [{"location": {"line": 1}, "cop_name": "X"}]},
                {"path": str(file_b), "offenses": [{"location": {"line": 1}, "cop_name": "X"}]},
            ],
            "summary": {"inspected_file_count": 2}
        }))

        resolve_symlink_paths.resolve_rubocop_json(str(json_path))
        data = json.loads(json_path.read_text())

        assert len(data["files"]) == 2


# ── main() auto-detection ──────────────────────────────────────


def test_main_detects_nitrocop_format():
    """main() detects nitrocop format by 'offenses' key."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        canonical, via_symlink = make_repo_with_symlink(tmp)

        json_path = tmp / "nc.json"
        json_path.write_text(json.dumps({
            "offenses": [
                {"path": str(via_symlink), "line": 1, "cop_name": "Style/Test"},
            ]
        }))

        sys.argv = ["resolve_symlink_paths.py", str(json_path)]
        resolve_symlink_paths.main()

        data = json.loads(json_path.read_text())
        assert data["offenses"][0]["path"] == str(canonical.resolve())


def test_main_detects_rubocop_format():
    """main() detects RuboCop format by 'files' key."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        canonical, via_symlink = make_repo_with_symlink(tmp)

        json_path = tmp / "rc.json"
        json_path.write_text(json.dumps({
            "files": [
                {"path": str(canonical), "offenses": []},
                {"path": str(via_symlink), "offenses": []},
            ]
        }))

        sys.argv = ["resolve_symlink_paths.py", str(json_path)]
        resolve_symlink_paths.main()

        data = json.loads(json_path.read_text())
        assert len(data["files"]) == 1


# ── Edge cases ──────────────────────────────────────────────────


def test_missing_json_file():
    """Missing file is silently skipped."""
    sys.argv = ["resolve_symlink_paths.py", "/no/such/file.json"]
    resolve_symlink_paths.main()  # should not raise


def test_invalid_json():
    """Invalid JSON is silently skipped."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        bad = tmp / "bad.json"
        bad.write_text("not json {{{")

        sys.argv = ["resolve_symlink_paths.py", str(bad)]
        resolve_symlink_paths.main()  # should not raise

        # File content unchanged
        assert bad.read_text() == "not json {{{"


def test_empty_json_object():
    """Empty JSON object is handled gracefully."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        empty = tmp / "empty.json"
        empty.write_text("{}")

        sys.argv = ["resolve_symlink_paths.py", str(empty)]
        resolve_symlink_paths.main()  # should not raise


def test_subprocess_invocation():
    """Script runs successfully as a subprocess."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        canonical, via_symlink = make_repo_with_symlink(tmp)

        nc_json = tmp / "nc.json"
        nc_json.write_text(json.dumps({
            "offenses": [
                {"path": str(via_symlink), "line": 1, "cop_name": "Style/Test"},
                {"path": str(canonical), "line": 1, "cop_name": "Style/Test"},
            ]
        }))

        rc_json = tmp / "rc.json"
        rc_json.write_text(json.dumps({
            "files": [
                {"path": str(canonical), "offenses": [{"location": {"line": 1}, "cop_name": "X"}]},
                {"path": str(via_symlink), "offenses": [{"location": {"line": 1}, "cop_name": "X"}]},
            ]
        }))

        result = subprocess.run(
            [sys.executable, str(SCRIPT), str(nc_json), str(rc_json)],
            capture_output=True, text=True,
        )
        assert result.returncode == 0, f"stderr: {result.stderr}"

        nc_data = json.loads(nc_json.read_text())
        rc_data = json.loads(rc_json.read_text())

        assert len(nc_data["offenses"]) == 1
        assert len(rc_data["files"]) == 1


if __name__ == "__main__":
    # Simple test runner
    failures = 0
    for name, func in sorted(globals().items()):
        if name.startswith("test_") and callable(func):
            try:
                func()
                print(f"  PASS  {name}")
            except Exception as e:
                print(f"  FAIL  {name}: {e}")
                failures += 1
    sys.exit(1 if failures else 0)
