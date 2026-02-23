# Future Ideas

Extracted from retired PLAN.md and PLAN2.md. None of these are committed work.

## Autocorrect (M7)

`--fix` / `-a` for safe-only corrections: trailing whitespace, frozen string literal,
string quotes, redundant return, etc. Start with cops where the fix is unambiguous
and non-overlapping. RuboCop's chained autocorrect conflicts are a known hazard.

## LSP Server

Serve diagnostics via LSP for editor integration. Single-file lint is already <50ms,
so the speed is there. Main work is the protocol layer and incremental re-lint on save.

## Strict Mode

`--mode=strict` — exit non-zero if any enabled cops are external (not implemented in
turbocop). For teams that want CI to enforce full coverage. Currently turbocop silently
skips unimplemented cops.

## External Cop Reporting

Print a summary line: "312 enabled, 287 supported, 25 external". Optionally list the
external cops with `--show-external`. Helps users understand coverage gaps.

## Doctor Command

`turbocop doctor` — diagnose common problems: stale lockfile, missing gems,
unsupported config keys, external cops breakdown by source (plugin vs custom require).

## Hybrid Wrapper

A `bin/lint` recipe that runs turbocop for covered cops, then `bundle exec rubocop --only <remaining>`
for the rest. Documents the incremental adoption path for teams that can't drop RuboCop yet.

---

# Optimization Ideas

## Current Performance Profile (2026-02-19)

Benchmarked with `.turbocop.cache` (no bundler calls), batched single-pass AST walker,
node-type dispatch table (915 cops annotated), pre-computed cop configs, and
optimized hot cops (Lint/Debugger, RSpec/NoExpectationExample).

### Phase Breakdown by Repo

All times are **cumulative across threads** except Wall (actual elapsed).

| Repo | Files | Cops/file | File I/O | Parse | filter+config | AST walk | Wall |
|------|------:|----------:|---------:|------:|--------------:|---------:|-----:|
| discourse | 5895 | 57 | 865ms | 386ms | 4s | 486ms | 509ms |
| rails | 3320 | 85 | 497ms | 305ms | 2s | 1s | 333ms |
| mastodon | 2540 | 675 | 337ms | 106ms | 5s | 2s | 599ms |
| rubocop | 1669 | 614 | 272ms | 139ms | 5s | 4s | 719ms |

### Filter+Config Sub-Phase Decomposition

The `filter+config` phase includes four distinct costs, profiled per-repo:

| Repo | is_cop_match | dir_override | check_lines | check_source |
|------|---:|---:|---:|---:|
| discourse | 333ms | 8ms | 28ms | 260ms |
| rails | 183ms | 80ms | 80ms | 1s |
| mastodon | 1s | 956ms | 119ms | 1s |
| rubocop | 402ms | 24ms | 3s | 2s |

- **is_cop_match**: Glob/regex matching per cop per file (915 cops × N files).
- **dir_override**: Path comparison + CopConfig cloning for nested `.rubocop.yml` overrides.
- **check_lines**: Line-by-line scanning (42 cops have implementations).
- **check_source**: Byte-level source scanning (130 cops have implementations).

### Per-Cop Hot Spots (TURBOCOP_COP_PROFILE=1)

Top cops by cumulative single-threaded time on the rubocop repo (1669 files):

| Cop | check_lines | check_source | AST | Total |
|-----|---:|---:|---:|---:|
| Naming/InclusiveLanguage | 1337ms | 0 | 6ms | 1343ms |
| RSpec/NoExpectationExample | 0 | 0 | 499ms | 499ms |
| Layout/EmptyLinesAroundArguments | 0 | 0 | 221ms | 221ms |
| Layout/EmptyLinesAroundBlockBody | 0 | 0 | 200ms | 200ms |
| Layout/HeredocIndentation | 0 | 0 | 184ms | 184ms |
| Style/PercentLiteralDelimiters | 0 | 0 | 151ms | 151ms |
| Layout/SpaceAroundKeyword | 0 | 135ms | 4ms | 139ms |

### Key Observations

1. **Cops/file varies 12x**: mastodon=675, rubocop=614, rails=85, discourse=57. Repos
   using rubocop-rspec + rubocop-rails enable far more cops per file.
2. **check_lines/check_source dominate filter+config**: On rubocop repo, they account
   for 5s of the 5s filter+config time. Matching overhead (402ms) is secondary.
3. **Naming/InclusiveLanguage is the #1 hot cop**: 1337ms re-compiling `fancy_regex`
   patterns for every file. Pre-compiling once would eliminate this.
4. **is_cop_match + dir_override = 2s on mastodon**: Glob matching 675 cops × 2540
   files = 1.7M match calls, plus path comparisons for 3 nested config directories.

---

## Completed Optimizations

### Optimization 1: Node-Type Dispatch Table ✅ DONE

**Status:** Implemented (commit 97aeb09)
**Measured impact:** 15-38% wall-time improvement

Each cop declares which AST node types it handles via `interested_node_types()`.
The `BatchedCopWalker` builds a `[Vec<(cop, config)>; 151]` dispatch table indexed
by node type tag. Only dispatches to relevant cops per node, skipping ~95% of
no-op `check_node` calls. 915 cops annotated via source scanning.

| Repo | Before | After | Change |
|------|-------:|------:|-------:|
| Discourse | 768ms | 654ms | -15% |
| Rails | 581ms | 518ms | -11% |
| Mastodon | 1.36s | 848ms | -38% |

### Optimization 1b: Pre-computed Cop Configs ✅ DONE

**Status:** Implemented
**Measured impact:** 5-23% CPU time reduction

Pre-compute `Vec<CopConfig>` once at startup (indexed by cop registry index).
In the per-file loop, use references to pre-computed configs instead of cloning.
Only clone+merge when directory-specific overrides (nested `.rubocop.yml`) match.

| Repo | Before (User) | After (User) | Change |
|------|-------------:|------------:|-------:|
| Discourse | 2998ms | 2709ms | -10% |
| Mastodon | 7850ms | 6015ms | -23% |
| Rails | 3659ms | 3403ms | -7% |

### Optimization 2: Eliminate Per-Call Vec Allocation ✅ DONE

**Status:** Implemented
**Measured impact:** 3-14% wall-time improvement

Changed all Cop trait methods to take `diagnostics: &mut Vec<Diagnostic>` instead
of returning `Vec<Diagnostic>`. Eliminates temporary Vec construction/destruction
across vtable calls for the 99%+ of no-op invocations.

| Repo | Before | After | Change |
|------|-------:|------:|-------:|
| Discourse | 460ms | 425ms | -8% |
| Rails | 411ms | 397ms | -3% |
| Mastodon | 679ms | 581ms | -14% |

### Optimization 2b: Hot Cop Fixes ✅ DONE

**Status:** Implemented (commit 4669272)
**Measured impact:** Lint/Debugger ~1000ms→~49ms, RSpec/NoExpectationExample ~264ms→~138ms

- **Lint/Debugger**: Static `HashSet<&[u8]>` of default leaf method names for O(1)
  rejection. Only touches config when method name matches a known debugger leaf.
- **RSpec/NoExpectationExample**: Narrowed `interested_node_types` to `CALL_NODE` only,
  compile `AllowedPatterns` regexes once per example instead of twice.

### Optimization 3: Pre-computed Cop Lists (Eliminate Filter Loop) ✅ DONE

**Status:** Implemented (commit 0755276)
**Measured impact:** filter+config -33% on mastodon, -50% on rails

At startup, cops are partitioned into `universal_cop_indices` (enabled, no Include/Exclude
patterns) and `pattern_cop_indices` (enabled, has patterns). Universal cops skip
`is_cop_match()` entirely. Directory override lookup (`find_override_dir_for_file`) runs
once per file instead of once per cop.

| Repo | Before (filter+config) | After | Change |
|------|---:|---:|---:|
| mastodon | 3s | 2s | -33% |
| rails | 2s | 1s | -50% |

### Optimization 4: Fix Naming/InclusiveLanguage Per-File Regex Compilation ✅ DONE

**Status:** Implemented
**Measured impact:** 1337ms → 628ms cumulative on rubocop repo (-53%)

Added a global `Mutex<HashMap<usize, Arc<Vec<FlaggedTerm>>>>` cache keyed by
`CopConfig` pointer. Since base configs are stable for the entire lint run, compiled
`fancy_regex::Regex` patterns are built once per config and reused for all files.
The remaining 628ms is inherent line scanning cost (lowercasing + substring search).

### Optimization 9: Fix Other Hot check_source Cops ✅ DONE

**Status:** Implemented
**Measured impact:** SpaceAroundKeyword 135ms→20ms (-85%), CollectionLiteralInLoop 27ms→~0ms

Three fixes applied:
- **Layout/SpaceAroundKeyword**: Replaced 8 separate full-file scans with single-pass
  first-byte dispatch. Keywords grouped by first letter (`c`→case, `e`→elsif, etc.).
- **Performance/CollectionLiteralInLoop**: Moved 3× per-file `HashSet<Vec<u8>>` builds
  (~150 inserts each) to `static LazyLock` sets compiled once at startup.
- **Layout/SpaceAroundOperators**: Converted `default_param_offsets` from `Vec` to
  `HashSet` for O(1) contains checks (minimal impact — byte scan dominates).

| Cop | Before | After | Change |
|-----|---:|---:|---:|
| Layout/SpaceAroundKeyword | 135ms | 20ms | -85% |
| Performance/CollectionLiteralInLoop | 27ms | ~0ms | eliminated |
| Layout/SpaceAroundOperators | 59ms | 66ms | ~same |

### Optimization 10: File-Level Result Caching (Incremental Linting) ✅ DONE

**Status:** Implemented
**Measured impact:** 983ms → 356ms warm re-run on turbocop repo (4369 files), ~2.8x speedup

Opt-in `--cache` flag enables per-file result caching. Two-level directory hierarchy:
`~/.cache/turbocop/<session_hash>/<file_hash>`. Session hash incorporates turbocop version,
deterministic config fingerprint (sorted HashMap keys), and `--only`/`--except` args.
File hash uses path + content SHA-256.

- Cache entries are compact JSON (no path field — implied by key)
- Atomic writes via temp file + rename for parallel safety
- XDG-compliant storage (`$TURBOCOP_CACHE_DIR` > `$XDG_CACHE_HOME/turbocop/` > `~/.cache/turbocop/`)
- Age-based eviction at 20K files (delete oldest 50% of session directories)
- `--cache-clear` removes the entire cache directory

| Scenario | Wall time |
|----------|----------|
| Cold run (no cache) | 983ms |
| Warm run (all hits) | 356ms |

---

## Investigated & Rejected

- **Skip fully-disabled departments** — cops already short-circuit on boolean flag (~10ns). Would save ~12ms total.
- **Faster YAML parser** — config loading is only 13-162ms with `.turbocop.cache`. Diminishing returns.
- **mmap for file I/O** — 98.4% of files under 32KB. Kernel page cache means `read()` already serves from memory. No measurable difference.

---

## Historical Baseline (2026-02-18, pre-optimization)

Before `.turbocop.cache` and the batched AST walker, bundler shell-outs dominated wall time.

### Bundler overhead

Each `bundle info --path <gem>` spawned a Ruby process (130-440ms each). A project with
5-8 plugins paid 1-2s just for gem path resolution — 41-48% of total wall time.

| Repo | Files | Bundler | Linting | Total | Bundler % |
|------|------:|--------:|--------:|------:|----------:|
| Discourse | 5,895 | 1.2s | 723ms | 2.9s | 41% |
| Mastodon | 2,540 | 1.6s | 1.0s | 3.3s | 48% |

### RuboCop comparison (pre-optimization)

| Repo | turbocop | RuboCop | Speedup |
|------|-------:|--------:|--------:|
| Discourse (with bundler) | 2.9s | 3.45s | 1.2x |
| Mastodon (with bundler) | 3.3s | 2.49s | 0.8x (slower) |
| Discourse (linting only) | 723ms | ~3.0s | 4.1x |
| Mastodon (linting only) | 1.0s | ~2.0s | 2.0x |

The `.turbocop.cache` mechanism (resolves gem paths once, caches to disk) eliminated
bundler from the hot path entirely.
