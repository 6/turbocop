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
python3 scripts/reduce-mismatch.py Style/SymbolProc --all-fns --parallel 4
python3 scripts/reduce-mismatch.py Style/SymbolProc --all --parallel 4     # both FP + FN
```

This produces a set of minimal repros, deduplicates them into clusters, and reports a summary. The whole point: 50 corpus mismatches collapse into 3–5 distinct root causes.

### Structural Deduplication

Raw reduction produces one minimal repro per corpus example. But many examples share the same root cause — e.g., 15 FPs that all reduce to "block with safe-nav receiver." We need to collapse these into distinct patterns.

**Fingerprinting approach** (compare reduced files structurally, not textually):

1. **Parse each reduced file with Prism** and extract a normalized AST skeleton:
   - Strip all identifiers → replace with positional placeholders (`v1`, `v2`, `m1`)
   - Strip all literals → replace with type tokens (`STR`, `INT`, `SYM`)
   - Keep structural nodes: `CallNode`, `BlockNode`, `IfNode`, `ClassNode`, etc.
   - Keep node nesting depth and child count

2. **Hash the skeleton** to get a fingerprint per reduced file:
   ```python
   def fingerprint(source: str) -> str:
       skeleton = normalize_ast(prism_parse(source))
       return hashlib.sha256(skeleton.encode()).hexdigest()[:12]
   ```

3. **Group by fingerprint** — identical fingerprints = same root cause.

4. **For near-duplicates** (similar but not identical skeletons), compute tree edit distance between AST skeletons. Merge clusters where distance < threshold (e.g., ≤2 node edits).

**Example normalization**:

```ruby
# Original reduced file:
users.select { |u| u.active? }

# Normalized skeleton:
v1.m1 { |v2| v2.m2 }

# Fingerprint: "a3f7c2e1b9d4"
```

```ruby
# Different original:
items.reject { |item| item.valid? }

# Same normalized skeleton:
v1.m1 { |v2| v2.m2 }

# Same fingerprint: "a3f7c2e1b9d4" → merged into one cluster
```

**What goes into the fingerprint (and what doesn't)**:

| Included | Excluded |
|----------|----------|
| AST node types | Variable/method names |
| Nesting structure | String/number literal values |
| Argument counts | Comments |
| Block vs lambda vs proc | Whitespace/formatting |
| Safe-nav (`&.`) vs regular call | Line positions |
| Receiver presence/absence | |

### Cluster Output Format

After batch reduction + dedup, the output looks like:

```
Style/SymbolProc: 47 FPs across 23 repos → 4 root causes

Cluster 1 (31 examples, 14 repos):
  Pattern: v1.m1 { |v2| v2.m2 }
  Trigger: single-method block on collection call
  Representative (8 lines):
    users.select { |u| u.active? }
  Repos: mastodon, discourse, rails, chatwoot, ...

Cluster 2 (9 examples, 7 repos):
  Pattern: v1.m1(&v2) { |v3| v3.m2 }
  Trigger: block with existing block-pass argument
  Representative (6 lines):
    process(items, &formatter) { |x| x.to_s }
  Repos: rubocop, good_job, ...

Cluster 3 (5 examples, 4 repos):
  Pattern: v1&.m1 { |v2| v2.m2 }
  Trigger: block on safe-navigation call
  Representative (7 lines):
    user&.roles { |r| r.name }
  Repos: chatwoot, docuseal, ...

Cluster 4 (2 examples, 1 repo):
  Pattern: v1.m1(v2) { |v3| v3.m2(v4, v5) }
  Trigger: block with multi-arg method call
  Representative (11 lines):
    records.map { |r| r.serialize(format, options) }
  Repos: rails
```

This is the "finite roadmap" — fix clusters 1–4 and you've eliminated all 47 FPs for this cop.

### Batch Workflow

```
┌─────────────────┐
│ corpus-results   │  (47 FP examples for cop X)
│    .json         │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Parallel        │  (reduce each example independently)
│  Reduction       │  (~3 min per example × 4 workers = ~35 min)
│  (--parallel 4)  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Fingerprint +   │  (AST normalization + hashing)
│  Cluster         │  (< 1 sec)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Pick best       │  (shortest repro per cluster)
│  representative  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Output:         │
│  - cluster       │  (JSON for tooling)
│    summary       │
│  - repro files   │  (/tmp/nitrocop-reduce/cop_name/)
│  - fixture       │  (optional --save-fixture)
│    candidates    │
└─────────────────┘
```

### Structured Output (for tooling integration)

Batch mode writes JSON alongside the human summary:

```json
{
  "cop": "Style/SymbolProc",
  "total_examples": 47,
  "total_clusters": 4,
  "clusters": [
    {
      "id": 1,
      "fingerprint": "a3f7c2e1b9d4",
      "example_count": 31,
      "repos": ["mastodon", "discourse", "rails"],
      "representative_file": "/tmp/nitrocop-reduce/Style_SymbolProc/cluster_1.rb",
      "representative_lines": 8,
      "pattern_description": "v1.m1 { |v2| v2.m2 }"
    }
  ]
}
```

This JSON can be consumed by `/fix-cops` to hand each cluster (not each raw FP) to a teammate.

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

1. **V1**: Single-file reducer with Phases 1+2. Takes cop + repo_id + file:line, shrinks the file, prints the result. FP and FN support. This alone is useful — paste the output into a fixture and you've got a test case.

2. **V2**: Add `--auto` (pick from corpus-results.json), `--save-fixture` (append to offense.rb/no_offense.rb with annotations), and the Prism parseability gate.

3. **V3**: Batch mode (`--all-fps`, `--all-fns`, `--all`) with parallel reduction. AST fingerprinting and cluster dedup. JSON output for tooling. Integration with `investigate-cop.py --reduce` and `/fix-cops` skill (hand clusters to teammates instead of raw examples).
