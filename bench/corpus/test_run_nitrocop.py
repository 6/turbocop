from __future__ import annotations

import importlib.util
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("run_nitrocop.py")
SPEC = importlib.util.spec_from_file_location("bench_corpus_run_nitrocop", MODULE_PATH)
assert SPEC and SPEC.loader
RUN_NITROCOP = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RUN_NITROCOP)


def test_deduplicate_offenses_resolves_symlink_aliases(tmp_path):
    real_dir = tmp_path / "real"
    real_dir.mkdir()
    real_file = real_dir / "thing.rb"
    real_file.write_text("def is_ready\nend\n")

    alias_dir = tmp_path / "alias"
    alias_dir.symlink_to(real_dir, target_is_directory=True)

    offenses = [
        {
            "path": str(real_file),
            "line": 1,
            "cop_name": "Naming/PredicatePrefix",
        },
        {
            "path": str(alias_dir / "thing.rb"),
            "line": 1,
            "cop_name": "Naming/PredicatePrefix",
        },
    ]

    assert RUN_NITROCOP.deduplicate_offenses(offenses) == 1


def test_deduplicate_offenses_keeps_distinct_locations(tmp_path):
    real_file = tmp_path / "thing.rb"
    real_file.write_text("def is_ready\nend\n")

    offenses = [
        {
            "path": str(real_file),
            "line": 1,
            "cop_name": "Naming/PredicatePrefix",
        },
        {
            "path": str(real_file),
            "line": 2,
            "cop_name": "Naming/PredicatePrefix",
        },
    ]

    assert RUN_NITROCOP.deduplicate_offenses(offenses) == 2
