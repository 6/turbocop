---
name: fix-cops
description: Fix a batch of high-impact corpus false-positive cops for nitrocop using triage, investigation, TDD, and per-cop corpus verification.
allowed-tools: Bash(*), Read, Write, Edit, Grep, Glob
---

# Fix Cops (Codex)

This skill runs after a corpus oracle run. It triages the results, chooses the
highest-impact cops, and fixes them directly in this workspace using TDD.

## Workflow

### Phase 1: Triage

1. Download corpus results and rank candidates:
   ```bash
   python3 .codex/skills/triage/scripts/triage.py --fp-only --limit 20 $ARGUMENTS
   ```

2. Select up to 4 cops for this batch. Prioritize:
   - FP-only cops (FN=0)
   - Highest FP count
   - High match rate (>90%)
   - Skip complex Layout alignment cops unless necessary

3. Investigate each selected cop:
   ```bash
   python3 scripts/investigate-cop.py Department/CopName --context --fp-only --limit 10
   ```

4. Reduce up to 3 FP examples per cop to minimal reproductions:
   ```bash
   python3 scripts/reduce-mismatch.py Department/CopName repo_id filepath:line
   ```
   Read reduced files from `/tmp/nitrocop-reduce/` and capture a root-cause hypothesis.

### Phase 2: Fix Each Cop (TDD)

For each selected cop, run the full loop before moving to the next one:

1. Read implementation and vendor behavior:
   - `src/cop/<dept>/<cop_name>.rs`
   - `vendor/rubocop*/spec/rubocop/cop/<dept>/<cop_name>_spec.rb`

2. Add failing test first:
   - FP fix: add case to `tests/fixtures/cops/<dept>/<cop_name>/no_offense.rb`
   - Run targeted test and confirm it fails:
     ```bash
     cargo test --release -p nitrocop --lib -- <cop_name_snake>
     ```

3. Implement fix in `src/cop/<dept>/<cop_name>.rs`.

4. Re-run targeted test; it must pass.

5. Keep existing fixtures unless they are factually incorrect.

### Phase 3: Batch Verification

After all selected cops are fixed:

1. Run project checks:
   ```bash
   cargo fmt
   cargo clippy --release -- -D warnings
   cargo test --release
   ```

2. Verify corpus conformance for each fixed cop:
   ```bash
   python3 scripts/check-cop.py Department/CopName --verbose --rerun
   ```
   All fixed cops must report `PASS` with zero excess offenses.

3. Regenerate coverage artifacts after cop fixes:
   ```bash
   cargo run --release --bin bench_nitrocop -- conform
   cargo run --bin coverage_table -- --show-missing --output docs/coverage.md
   ```

### Phase 4: Report

Report:
- Cops fixed and their FP reductions
- Any cops deferred and why
- Verification status (`fmt`, `clippy`, `test`, `check-cop`, coverage docs)

## Notes

- Never use `git stash` or `git stash pop`.
- Do not copy identifiers from private repos into fixtures or source.
- Prefer generic minimal repros and generic naming in tests.

## Arguments

- `$fix-cops` — default top FP-only cops
- `$fix-cops --department Style` — only Style cops
- `$fix-cops --limit 10` — consider more candidates
- `$fix-cops --input /path/to/corpus-results.json` — use local file
