# rblint Optimization Ideas

## Current Performance Profile (2026-02-19)

Benchmarked with `.rblint.cache` (no bundler calls), batched single-pass AST walker,
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

### Per-Cop Hot Spots (RBLINT_COP_PROFILE=1)

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

---

## Investigated & Rejected

### Skip Fully-Disabled Departments (NO IMPACT)

Disabled cops already short-circuit on a boolean flag check (~10ns). Department-level
pre-filter would save ~12ms total.

### Faster YAML Parser (LOW IMPACT)

Config loading is only 13-162ms with `.rblint.cache`. Not worth the effort.

### mmap for File I/O (NO IMPACT)

98.4% of Ruby files are under 32KB. Kernel page cache means `read()` is already
serving from memory. Zero measurable difference.

---

## Proposed Optimizations

### Optimization 9: Fix Other Hot check_source Cops

**Status:** Not started
**Expected impact:** ~200-500ms cumulative reduction
**Category:** Pure speed improvement

Several `check_source` cops have per-file overhead that could be reduced:

| Cop | Cost (rubocop) | Issue |
|-----|---:|------|
| Layout/SpaceAroundKeyword | 135ms | Full source byte scan per file |
| Layout/SpaceAroundOperators | 59ms | Full source byte scan per file |
| Style/DoubleCopDisableDirective | 41ms | Scans all comments per file |
| Performance/CollectionLiteralInLoop | 27ms | Per-file config parsing |

These are lower priority — each saves 30-135ms cumulative. The fix pattern is
similar to Lint/Debugger: avoid per-file config parsing, use pre-compiled patterns.

---

### Optimization 10: File-Level Result Caching (Incremental Linting)

**Status:** Deferred — not implementing now
**Expected impact:** 10-100x faster for warm re-runs
**Category:** Caching

#### Decision: Not Now

rblint's cold-run is already faster than RuboCop's warm-cached run. Our benchmarks
now use `--no-cache` for apples-to-apples cold-run comparison. Caching adds
significant complexity (invalidation, storage management, corruption handling) and
is a common source of bugs in RuboCop (`rubocop --cache clear` is a frequent
troubleshooting step). We should exhaust pure speed optimizations (#7, #8, #9) first —
these improve both cold and warm performance without correctness risk.

When rblint reaches feature parity and production adoption, caching becomes worthwhile
for developer workflows (re-running after small edits). At that point, the RuboCop
implementation details below provide a reference design.

#### Benchmark Note

Our `bench_rblint` benchmark uses `--no-cache` when invoking RuboCop, ensuring an
apples-to-apples comparison of raw linting speed. Both tools do a full cold-run
parse + analyze on every file for each hyperfine iteration.

#### RuboCop Cache Implementation Reference

RuboCop has had file-level result caching since v0.35 (2015), enabled by default.
On a cache hit, it skips parsing and analysis entirely — deserializing cached
offenses directly.

**Storage location** (in precedence order):
1. `$RUBOCOP_CACHE_ROOT/rubocop_cache` (env var)
2. `$XDG_CACHE_HOME/<uid>/rubocop_cache` (env var)
3. `~/.cache/rubocop_cache` (default)
4. Configurable via `AllCops.CacheRootDirectory` or `--cache-root DIR`

**Directory structure** — 3-level hierarchy:
```
rubocop_cache/
└── <source_checksum>/          # RuboCop version + loaded features
    └── <context_checksum>/     # External deps + CLI options
        └── <file_checksum>     # Per-file cache entry (JSON)
```

**Cache key composition** — three independent SHA1 checksums:

| Level | Checksum of | Invalidated by |
|-------|-------------|----------------|
| Source | `$LOADED_FEATURES` + `exe/` files + RuboCop version + AST version | RuboCop gem update, plugin changes |
| Context | External dependency checksums (e.g. `db/schema.rb` for Rails cops) + relevant CLI options | Schema changes, option changes |
| File | File path + file mode + per-file config signature + file content | Any file edit, config change, permission change |

**Cache entry format**: UTF-8 JSON array of offense objects:
```json
[{"severity":"warning","location":{"begin_pos":123,"end_pos":156},
  "message":"...","cop_name":"Dept/Cop","status":"uncorrected"}]
```

**Eviction**: When cache exceeds `MaxFilesInCache` (default 20,000), removes the
oldest 50% + 1 of entries by mtime. Batched in groups of 10,000. Empty parent
directories are cleaned up.

**Config options** (in `AllCops`):
| Key | Default | Purpose |
|-----|---------|---------|
| `UseCache` | `true` | Enable/disable |
| `CacheRootDirectory` | `~` | Override root path |
| `MaxFilesInCache` | `20000` | Eviction threshold |
| `AllowSymlinksInCacheRootDirectory` | `false` | Symlink security check |

**CLI flags**: `--cache true/false`, `--cache-root DIR`

**Key source files** (in `vendor/rubocop/lib/rubocop/`):
- `result_cache.rb` — checksums, storage, cleanup
- `cached_data.rb` — JSON serialization of offenses
- `cache_config.rb` — cache root directory discovery
- `runner.rb` — cache load/save orchestration (lines 181-200)

#### Future rblint Implementation Notes

If/when we implement caching:
- Start opt-in (`--cache`), not default, until battle-tested
- Support `--no-cache` and `--cache-clear` flags
- Cache key: content SHA256 + config signature + rblint version
- Storage: `~/.cache/rblint/` (XDG-compliant), or project-local via config
- Format: bincode or MessagePack (faster than JSON for Rust)
- Consider storing diagnostic byte offsets rather than line:col to avoid
  recomputing line tables on cache load

---

## Priority Order

### Completed
1. ~~Node-type dispatch table~~ ✅ -15 to -38% wall time
2. ~~Pre-computed cop configs~~ ✅ -5 to -23% CPU time
3. ~~Eliminate per-call Vec allocation~~ ✅ -3 to -14% wall time
4. ~~Hot cop fixes (Debugger, NoExpectationExample)~~ ✅
5. ~~Pre-computed cop lists~~ ✅ -33 to -50% filter+config time
6. ~~Naming/InclusiveLanguage fix~~ ✅ -53% cumulative (1337ms → 628ms)

### Next (pure speed, no caching)
7. **Other hot check_source cops** — incremental gains (~50-100ms wall)

### Deferred
8. **File-level result caching** — not now; pure speed gains come first

### Rejected
- Skip disabled departments — no measurable impact
- Faster YAML parser — diminishing returns
- mmap file I/O — no effect
