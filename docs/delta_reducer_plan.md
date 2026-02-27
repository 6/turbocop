# Delta Reducer for Corpus Mismatches

## Problem

When `investigate-cop.py` surfaces an FP/FN in a 400-line file, finding the root cause is manual and slow. You read the source, guess which syntax motif triggers the mismatch, and iterate. This is the main bottleneck in the fix loop.

## Goal

A script that takes a corpus mismatch (cop + file) and automatically shrinks the file to a minimal reproduction — ideally 5–20 lines — that still exhibits the same FP or FN. Minimal repros make the root cause obvious and double as regression test fixtures.

## Design

### Input

```bash
python3 scripts/reduce-mismatch.py Style/SymbolProc mastodon__mastodon__c1f398a app/models/user.rb:42
```

Arguments:
- Cop name
- Repo ID (from corpus)
- File path + line (from `fp_examples` / `fn_examples` in `corpus-results.json`)

Or with `--input` to auto-pick the first example:
```bash
python3 scripts/reduce-mismatch.py Style/SymbolProc --auto
```

This reads `corpus-results.json`, picks the first FP example for that cop, and reduces it.

### "Interesting" Predicate

The predicate defines what we're trying to preserve during reduction.

**For FP** (nitrocop fires, rubocop doesn't):
1. Run `nitrocop --only CopName --format json --no-cache --config bench/corpus/baseline_rubocop.yml file.rb`
2. Run `bundle exec rubocop --only CopName --format json file.rb` (in the repo's bundler context)
3. FP preserved = nitrocop reports ≥1 offense AND rubocop reports 0

**For FN** (rubocop fires, nitrocop doesn't):
1. Same two runs
2. FN preserved = rubocop reports ≥1 offense AND nitrocop reports 0

**Parseability gate**: Before checking the predicate, verify the reduced file parses:
```bash
ruby -e "require 'prism'; exit(Prism.parse(File.read(ARGV[0])).errors.empty? ? 0 : 1)" file.rb
```
If the file doesn't parse, the reduction candidate is rejected.

### Algorithm

Classic delta debugging (ddmin), adapted for Ruby source:

**Phase 1 — Block deletion (coarse)**
1. Split file into N chunks (start with N=2)
2. For each chunk, try deleting it
3. If the result is parseable AND still "interesting", accept the deletion
4. If no single chunk deletion works, increase N (double it) and retry
5. Repeat until N > number of remaining lines

**Phase 2 — Line deletion (fine)**
1. For each line (bottom to top), try deleting it
2. If still parseable + interesting, keep the deletion
3. Single pass is usually sufficient after Phase 1

**Phase 3 — Simplification (optional)**
1. Replace string literals with `"x"`
2. Replace integer literals with `1`
3. Replace variable names with short names (`a`, `b`, `c`)
4. Each simplification: accept if still parseable + interesting

Phase 3 is nice-to-have. Phases 1+2 get you 90% of the value.

### Performance

Each predicate check requires running both nitrocop and rubocop. Nitrocop on a single file with `--only` is ~50ms. RuboCop is ~1–2s (bundler overhead). Expect:

- Phase 1: ~20–40 predicate checks (log₂ of line count × chunk iterations)
- Phase 2: ~N checks where N = remaining lines after Phase 1

For a 400-line file → roughly 50–80 rubocop invocations → **1–3 minutes total**. Acceptable for a dev tool.

**Optimization**: Cache rubocop's baseline result. If the original file's rubocop output shows 0 offenses for this cop (FP case), we only need to recheck nitrocop during reduction — rubocop will still show 0 on any subset. This cuts predicate cost in half for FP cases.

### Output

```
Reduced 387 lines → 11 lines (34 iterations, 47s)
Wrote: /tmp/nitrocop-reduce/Style_SymbolProc_reduced.rb
```

The reduced file is:
1. Written to a temp directory
2. Optionally copied into the fixture directory with `--save-fixture`:
   ```bash
   python3 scripts/reduce-mismatch.py Style/SymbolProc --auto --save-fixture
   # → appends to tests/fixtures/cops/style/symbol_proc/offense.rb (for FN)
   # → appends to tests/fixtures/cops/style/symbol_proc/no_offense.rb (for FP)
   ```

### Batch Mode

Reduce all FP examples for a cop at once:

```bash
python3 scripts/reduce-mismatch.py Style/SymbolProc --all-fps --parallel 4
```

This produces a set of minimal repros, then deduplicates (if two examples reduce to structurally similar code, keep one). Output is a summary:

```
Style/SymbolProc: 12 FPs → 4 distinct minimal repros
  1. lambda with proc argument (8 lines)
  2. method with safe-nav receiver (6 lines)
  3. block inside conditional (11 lines)
  4. nested block with multiple args (9 lines)
```

This directly gives you the "few root causes" view that the feedback doc was asking for.

### Integration with Existing Tools

- **`investigate-cop.py`**: Add `--reduce` flag that pipes each example into the reducer
- **`check-cop.py`**: After `--verbose` shows regressions, offer to reduce new FPs
- **`/fix-cops` skill**: Before handing a cop to a teammate, pre-reduce the top 3 FP examples so the teammate gets minimal repros instead of 400-line files

### Key Files

| File | Role |
|------|------|
| `scripts/reduce-mismatch.py` | Main reducer script (new) |
| `bench/corpus/baseline_rubocop.yml` | Config used for nitrocop invocations |
| `vendor/corpus/{repo_id}/{filepath}` | Source files to reduce |
| `scripts/investigate-cop.py` | FP/FN example source; integration point |
| `scripts/check-cop.py` | Regression verification; integration point |

### Risks / Open Questions

1. **RuboCop bundler context**: Reducing a file outside the original repo may change rubocop's behavior if the cop depends on project config. Mitigation: run rubocop with `--config bench/corpus/baseline_rubocop.yml` too, or copy the reduced file into the original repo for checking.

2. **Context-dependent cops**: Some cops (e.g., `Rails/HasManyOrHasOneDependent`) need class context to fire. Deletion of surrounding class/module could eliminate the offense. Mitigation: Phase 1 should try preserving the innermost enclosing class/module/def structure.

3. **Multi-offense files**: A file might have multiple FPs for the same cop. The predicate checks for ≥1, so reduction naturally converges on preserving at least one. To get distinct repros per offense, run reduction once per target line (filter predicate to require offense on or near the target line).

4. **Rubocop startup cost**: ~1-2s per invocation is the main bottleneck. Could use `rubocop --server` (daemon mode) to amortize startup, cutting per-check cost to ~200ms.

## Implementation Order

1. **V1**: Single-file FP reducer with Phases 1+2. No batch mode, no fixture saving. Just shrinks a file and prints the result. This alone is useful.
2. **V2**: Add `--auto` (pick from corpus-results.json), `--save-fixture`, and FN support.
3. **V3**: Batch mode with deduplication. Integration with investigate-cop.py.
