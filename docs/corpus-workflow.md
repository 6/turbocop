# Corpus Workflow

## Investigation

Use the cached corpus tools before rerunning expensive checks:

```bash
python3 scripts/investigate_cop.py Department/CopName
python3 scripts/investigate_cop.py Department/CopName --repos-only
python3 scripts/investigate_cop.py Department/CopName --context
python3 scripts/investigate_repo.py rails
python3 scripts/reduce_mismatch.py Department/CopName repo_id path/to/file.rb:line
```

`investigate_cop.py` and `investigate_repo.py` download the latest corpus artifact automatically. Do not manually download artifacts first.

## Regression Checks

Use `check_cop.py` for aggregate corpus validation after a fix:

```bash
python3 scripts/check_cop.py Department/CopName
python3 scripts/check_cop.py Department/CopName --verbose
python3 scripts/check_cop.py Department/CopName --verbose --rerun
python3 scripts/check_cop.py Department/CopName --verbose --rerun --quick
python3 scripts/verify_cop_locations.py Department/CopName
```

Important:

- `check_cop.py` is count-based. It does not prove exact location matches.
- `verify_cop_locations.py` checks the known oracle FP/FN locations.
- “file-drop noise” is not an excuse for real FN gaps. Investigate the actual missed examples.

## Corpus Bundle Notes

`check_cop.py --rerun` and `--corpus-check` require the active Ruby bundle under `bench/corpus/vendor/bundle/`.

```bash
cd bench/corpus
BUNDLE_PATH=vendor/bundle bundle install
```

If `bundle info rubocop` fails, config resolution falls back to defaults and counts become unreliable.

## Dispatch And Repair

Issue-backed cop dispatch:

```bash
python3 scripts/dispatch_cops.py issues-sync --binary target/debug/nitrocop
python3 scripts/dispatch_cops.py dispatch-issues --max-active 5
python3 scripts/dispatch_cops.py backend --cop Department/CopName --binary target/debug/nitrocop
```

Regression triage:

```bash
python3 scripts/investigate_regression.py --action report
```

## Bench And Conformance

Use the full bench/conformance flow only when you explicitly need repo-wide regeneration:

```bash
cargo run --release --bin bench_nitrocop -- conform
cargo run --release --bin bench_nitrocop -- quick
cargo run --release --bin bench_nitrocop -- autocorrect-conform
```

Avoid regenerating bench outputs during routine cop-fix loops unless the task explicitly calls for it.
