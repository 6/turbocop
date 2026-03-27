#!/usr/bin/env python3
"""Tests for bench/corpus/oracle_runner.py."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).parents[2] / "bench" / "corpus" / "oracle_runner.py"


def run_cmd(*args: str, env_extra: dict | None = None) -> subprocess.CompletedProcess:
    env = os.environ.copy()
    if env_extra:
        env.update(env_extra)
    return subprocess.run(
        [sys.executable, str(SCRIPT), *args],
        capture_output=True, text=True, env=env,
    )


def test_delete_crash_files_known_repo():
    with tempfile.TemporaryDirectory() as tmpdir:
        # Create the crash-prone files
        (Path(tmpdir) / "bench" / "compiler").mkdir(parents=True)
        (Path(tmpdir) / "bench" / "compiler" / "bench_compilation.rb").write_text("x")
        (Path(tmpdir) / "test" / "jruby").mkdir(parents=True)
        (Path(tmpdir) / "test" / "jruby" / "test_regexp.rb").write_text("x")

        r = run_cmd("delete-crash-files", "--repo-id", "jruby__jruby__0303464", "--repo-dir", tmpdir)
        assert r.returncode == 0
        assert not (Path(tmpdir) / "bench" / "compiler" / "bench_compilation.rb").exists()
        assert not (Path(tmpdir) / "test" / "jruby" / "test_regexp.rb").exists()


def test_delete_crash_files_unknown_repo():
    with tempfile.TemporaryDirectory() as tmpdir:
        r = run_cmd("delete-crash-files", "--repo-id", "unknown__repo__abc", "--repo-dir", tmpdir)
        assert r.returncode == 0


def test_validate_rubocop_passes():
    with tempfile.TemporaryDirectory() as tmpdir:
        result = {"files": [{"path": f"{tmpdir}/app/foo.rb", "offenses": []}]}
        json_path = Path(tmpdir) / "result.json"
        json_path.write_text(json.dumps(result))
        r = run_cmd("validate-rubocop", str(json_path), tmpdir)
        assert r.returncode == 0


def test_validate_rubocop_rejects_outside_paths():
    with tempfile.TemporaryDirectory() as tmpdir:
        result = {"files": [{"path": "/etc/passwd", "offenses": []}]}
        json_path = Path(tmpdir) / "result.json"
        json_path.write_text(json.dumps(result))
        r = run_cmd("validate-rubocop", str(json_path), tmpdir)
        assert r.returncode == 1
        assert "POISONED" in r.stderr


def test_resolve_symlinks_nitrocop_format():
    with tempfile.TemporaryDirectory() as tmpdir:
        # Create a file and a symlink to it
        real_file = Path(tmpdir) / "real.rb"
        real_file.write_text("x = 1")
        link_file = Path(tmpdir) / "link.rb"
        link_file.symlink_to(real_file)

        data = {"offenses": [
            {"path": str(real_file), "line": 1, "cop_name": "Style/Foo"},
            {"path": str(link_file), "line": 1, "cop_name": "Style/Foo"},
        ]}
        json_path = Path(tmpdir) / "nc.json"
        json_path.write_text(json.dumps(data))

        r = run_cmd("resolve-symlinks", str(json_path))
        assert r.returncode == 0

        result = json.loads(json_path.read_text())
        # Should deduplicate: both paths resolve to the same realpath
        assert len(result["offenses"]) == 1
        assert result["offenses"][0]["path"] == str(real_file.resolve())


def test_resolve_symlinks_rubocop_format():
    with tempfile.TemporaryDirectory() as tmpdir:
        real_file = Path(tmpdir) / "real.rb"
        real_file.write_text("x = 1")
        link_file = Path(tmpdir) / "link.rb"
        link_file.symlink_to(real_file)

        data = {"files": [
            {"path": str(real_file), "offenses": [
                {"location": {"line": 1}, "cop_name": "Style/Foo"}
            ]},
            {"path": str(link_file), "offenses": [
                {"location": {"line": 1}, "cop_name": "Style/Foo"}
            ]},
        ]}
        json_path = Path(tmpdir) / "rc.json"
        json_path.write_text(json.dumps(data))

        r = run_cmd("resolve-symlinks", str(json_path))
        assert r.returncode == 0

        result = json.loads(json_path.read_text())
        assert len(result["files"]) == 1


def test_extract_context():
    with tempfile.TemporaryDirectory() as tmpdir:
        repo = Path(tmpdir) / "repo"
        repo.mkdir()
        (repo / "foo.rb").write_text("line1\nline2\nline3\nline4\nline5\n")

        nc = {"offenses": [{"path": "foo.rb", "line": 3, "cop_name": "Style/Foo"}]}
        rc = {"files": []}
        nc_path = Path(tmpdir) / "nc.json"
        rc_path = Path(tmpdir) / "rc.json"
        output = Path(tmpdir) / "ctx.json"
        nc_path.write_text(json.dumps(nc))
        rc_path.write_text(json.dumps(rc))

        r = run_cmd(
            "extract-context",
            "--nitrocop-json", str(nc_path),
            "--rubocop-json", str(rc_path),
            "--repo-dir", str(repo),
            "--output", str(output),
            "--context-lines", "2",
        )
        assert r.returncode == 0
        result = json.loads(output.read_text())
        assert "foo.rb:3" in result
        assert any("line3" in line for line in result["foo.rb:3"]["context"])


def test_consolidate_cache():
    with tempfile.TemporaryDirectory() as tmpdir:
        results_dir = Path(tmpdir) / "results"
        cache_dir = Path(tmpdir) / "cache"
        results_dir.mkdir()

        # Write a fake result
        (results_dir / "test_repo.json").write_text('{"files": []}')

        # We need a manifest — create a minimal one
        manifest = Path(tmpdir) / "manifest.jsonl"
        manifest.write_text(json.dumps({"id": "test_repo", "repo_url": "x", "sha": "abc123"}) + "\n")

        # Patch MANIFEST_PATH by running inline
        r = subprocess.run(
            [sys.executable, "-c", f"""
import sys; sys.path.insert(0, '{SCRIPT.parent}')
import oracle_runner
oracle_runner.MANIFEST_PATH = '{manifest}'
import argparse
args = argparse.Namespace(results_dir='{results_dir}', cache_dir='{cache_dir}')
oracle_runner.cmd_consolidate_cache(args)
"""],
            capture_output=True, text=True,
        )
        assert r.returncode == 0
        assert (cache_dir / "test_repo_abc123.json").exists()
