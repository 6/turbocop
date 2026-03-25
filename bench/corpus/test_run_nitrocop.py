import sys
from pathlib import Path

CORPUS_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(CORPUS_DIR))

import gen_repo_config
import run_nitrocop


def test_prepare_repo_dir_symlinks_vendor_corpus_repo(tmp_path, monkeypatch):
    monkeypatch.setattr(run_nitrocop, "PROJECT_ROOT", tmp_path)
    real_repo = tmp_path / "vendor" / "corpus" / "demo__repo__abc123"
    real_repo.mkdir(parents=True)

    lint_path, tmpdir = run_nitrocop.prepare_repo_dir(str(real_repo))
    try:
        assert tmpdir is not None
        assert lint_path.is_symlink()
        assert lint_path.name == real_repo.name
        assert lint_path.resolve() == real_repo.resolve()
        assert "vendor/corpus" not in str(lint_path)

        env = run_nitrocop.build_env(str(lint_path))
        assert env["GIT_CEILING_DIRECTORIES"] == str(lint_path.parent)
    finally:
        if tmpdir is not None:
            tmpdir.cleanup()


def test_prepare_repo_dir_leaves_non_corpus_repo_unchanged(tmp_path, monkeypatch):
    monkeypatch.setattr(run_nitrocop, "PROJECT_ROOT", tmp_path / "workspace")
    repo = tmp_path / "repos" / "demo__repo__abc123"
    repo.mkdir(parents=True)

    lint_path, tmpdir = run_nitrocop.prepare_repo_dir(str(repo))

    assert lint_path == repo
    assert tmpdir is None


def test_gen_repo_config_absolute_path_preserves_symlink(tmp_path):
    real_repo = tmp_path / "vendor" / "corpus" / "demo__repo__abc123"
    real_repo.mkdir(parents=True)
    link_root = tmp_path / "shadow" / "repos"
    link_root.mkdir(parents=True)
    link = link_root / real_repo.name
    link.symlink_to(real_repo)

    assert gen_repo_config.absolute_path(str(link)) == link
