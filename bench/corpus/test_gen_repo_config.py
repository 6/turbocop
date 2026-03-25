#!/usr/bin/env python3
"""Tests for gen_repo_config.py."""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).with_name("gen_repo_config.py")
SPEC = importlib.util.spec_from_file_location("gen_repo_config", SCRIPT)
assert SPEC and SPEC.loader
gen_repo_config = importlib.util.module_from_spec(SPEC)
sys.modules["gen_repo_config"] = gen_repo_config
SPEC.loader.exec_module(gen_repo_config)


def test_resolve_repo_config_writes_overlay_in_isolated_temp_dir():
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        excludes_path = tmp_path / "repo_excludes.json"
        excludes_path.write_text(json.dumps({"demo-repo": {"exclude": ["tmp/file.rb"]}}))

        base_config = tmp_path / "baseline.yml"
        base_config.write_text("AllCops:\n  Exclude: []\n")

        repo_dir = tmp_path / "repo"
        repo_dir.mkdir()

        resolved = Path(
            gen_repo_config.resolve_repo_config(
                "demo-repo",
                str(base_config),
                str(repo_dir),
                excludes_path=excludes_path,
            )
        )

        assert resolved == (
            Path(tempfile.gettempdir())
            / "nitrocop_corpus_configs"
            / "demo-repo"
            / "corpus_config.yml"
        )
        assert resolved.exists()
        assert resolved.parent != Path(tempfile.gettempdir())

        contents = resolved.read_text()
        assert f"inherit_from: {base_config.resolve()}" in contents
        assert f'- "{repo_dir.resolve()}/tmp/file.rb"' in contents


def test_resolve_repo_config_returns_base_config_without_repo_entry():
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        excludes_path = tmp_path / "repo_excludes.json"
        excludes_path.write_text(json.dumps({"other-repo": {"exclude": ["tmp/file.rb"]}}))

        base_config = tmp_path / "baseline.yml"
        base_config.write_text("AllCops:\n  Exclude: []\n")

        repo_dir = tmp_path / "repo"
        repo_dir.mkdir()

        resolved = gen_repo_config.resolve_repo_config(
            "demo-repo",
            str(base_config),
            str(repo_dir),
            excludes_path=excludes_path,
        )

        assert resolved == str(base_config)


if __name__ == "__main__":
    test_resolve_repo_config_writes_overlay_in_isolated_temp_dir()
    test_resolve_repo_config_returns_base_config_without_repo_entry()
    print("All tests passed.")
