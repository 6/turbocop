# rblint Optimization Ideas

## Current Performance Profile (2026-02-18, updated)

Benchmarked with `.rblint.cache` (no bundler calls), batched single-pass AST walker,
node-type dispatch table (725 cops annotated), and pre-computed cop configs.

### Phase Breakdown by Repo

| Repo | Files | Config | File I/O | Parse | Cop Exec | filter+config | AST walk | Wall |
|------|------:|-------:|---------:|------:|---------:|--------------:|---------:|-----:|
| discourse | 5895 | 399ms | 1s | 440ms | 3s | 642ms | 2s | 460ms |
| rails | 3320 | 47ms | 232ms | 279ms | 4s | 1s | 2s | 411ms |
| mastodon | 2540 | 95ms | 723ms | 234ms | 8s | 4s | 3s | 679ms |
| chatwoot | 2247 | — | — | — | 6s | 2s | 4s | 557ms |
| rubocop | 1669 | — | — | — | 11s | 5s | 6s | 955ms |
| rubygems.org | 1239 | — | — | — | 2s | 1s | 1s | 218ms |
| docuseal | 409 | — | — | — | 1s | 449ms | 570ms | 129ms |
| activeadmin | 374 | — | — | — | 96ms | 49ms | 47ms | 22ms |
| good_job | 172 | — | — | — | 779ms | 255ms | 524ms | 118ms |
| errbit | 217 | — | — | — | 548ms | 214ms | 334ms | 73ms |

Note: File I/O, Parse, and Cop Exec are **cumulative across threads**. Wall time is
actual elapsed time (parallelized across ~10 cores). Config loading is serial.
`filter+config` includes cop filter checks, config lookups, and `check_lines`/`check_source`
calls. `AST walk` is the batched node-type-dispatched tree walk with `check_node` calls.

### Key Insight

**Cop execution dominates**: 75-95% of cumulative thread time.
After the node-type dispatch table, AST walk time is now reasonable (each cop only
called for its declared node types). The remaining `filter+config` cost is split between
the per-cop filter loop (915 cops × N files) and `check_lines`/`check_source` calls.

---

## Optimization 1: Node-Type Dispatch Table ✅ DONE

**Status:** Implemented (commit 97aeb09)
**Measured impact:** 15-38% wall-time improvement

Each cop declares which AST node types it handles via `interested_node_types()`.
The `BatchedCopWalker` builds a `[Vec<(cop, config)>; 151]` dispatch table indexed
by node type tag. Only dispatches to relevant cops per node, skipping ~95% of
no-op `check_node` calls. 725 of 745 cops auto-annotated via source scanning.

| Repo | Before | After | Change |
|------|-------:|------:|-------:|
| Discourse | 768ms | 654ms | -15% |
| Rails | 581ms | 518ms | -11% |
| Mastodon | 1.36s | 848ms | -38% |

---

## Optimization 1b: Pre-computed Cop Configs ✅ DONE

**Status:** Implemented
**Measured impact:** 5-23% CPU time reduction

### Problem

`cop_config_for_file()` was called for every enabled cop × every file, doing a
HashMap lookup + clone of `CopConfig` (which contains a `HashMap<String, serde_yml::Value>`).
This accounted for 33-56% of the `filter+config` phase.

### Solution

Pre-compute `Vec<CopConfig>` once at startup (indexed by cop registry index).
In the per-file loop, use references to pre-computed configs instead of cloning.
Only clone+merge when directory-specific overrides (nested `.rubocop.yml`) match.

| Repo | Before (User) | After (User) | Change |
|------|-------------:|------------:|-------:|
| Discourse | 2998ms | 2709ms | -10% |
| Mastodon | 7850ms | 6015ms | -23% |
| Rails | 3659ms | 3403ms | -7% |

---

## Optimization 2: Eliminate Per-Call Vec Allocation (MEDIUM IMPACT)

**Status:** Not started
**Expected impact:** 10-20% reduction in cop execution time

### Problem

Every `check_node` call returns `Vec<Diagnostic>`, even when empty (99%+ of calls):

```rust
fn check_node(&self, ...) -> Vec<Diagnostic> {
    let node = match node.as_call_node() {
        Some(n) => n,
        None => return Vec::new(),  // Hot path: allocate + return empty Vec
    };
    // ...
}
```

While `Vec::new()` doesn't heap-allocate, the pattern still involves: construct Vec,
return it across a vtable call, call `extend()` on the collector, drop the Vec.

### Solution

Change the signature to push directly into a shared collector:

```rust
fn check_node(
    &self,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    parse_result: &ruby_prism::ParseResult<'_>,
    config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,  // push directly
)
```

### Effort

Medium — requires changing the Cop trait signature and all 745+129+41 implementations.
Mechanical transformation: `vec![diagnostic]` → `diagnostics.push(diagnostic)`,
`Vec::new()` → `()`.

---

## Optimization 3: Skip Fully-Disabled Departments (NO IMPACT)

**Status:** Investigated, not worth pursuing
**Expected impact:** ~12ms total — unmeasurable

Disabled cops already short-circuit on a boolean flag check in `is_cop_match()` (~10ns each).
A department-level pre-filter would save ~100-200 boolean checks per file. At 10ns each,
that's ~2µs/file × 6000 files = 12ms total. Not worth the code complexity.

---

## Optimization 4: Faster YAML Parser (LOW IMPACT)

**Status:** Investigated, not worth pursuing
**Expected impact:** ~10-30ms for most repos, ~70-100ms for Discourse

Investigated rapidyaml (C++ SIMD, 10-15x faster than serde_yml). The Rust bindings
(`ryml` crate) are GPLv3-licensed (incompatible). Custom bindings possible but high
effort. Config loading is only 13-162ms for repos with `.rblint.cache`, so this is
diminishing returns.

---

## Optimization 5: mmap for File I/O (NO IMPACT)

**Status:** Investigated and reverted (commits f9a8cdc → eeb29dd)
**Measured impact:** 0% (768ms vs 775ms, within noise)

Ruby source files are 98.4% under 32KB. The mmap path only applied to 1.6% of files.
Kernel page cache means `read()` is already serving from memory. Added unsafe code
complexity for zero gain.

---

## Optimization 6: Config Merge Optimization (LOW IMPACT)

**Status:** Not started
**Expected impact:** ~10-30ms for Discourse, negligible for others

Replace O(n²) containment checks in exclude array deduplication with HashSet-based
dedup. Only matters for Discourse (31 nested configs × 200+ cops = many merge ops).

---

## Priority Order

1. ~~**Node-type dispatch table**~~ ✅ -15 to -38% wall time
2. ~~**Pre-computed cop configs**~~ ✅ -5 to -23% CPU time
3. **Eliminate per-call Vec allocation** — medium impact, synergizes with #1
4. ~~Skip disabled departments~~ — no impact (already short-circuited)
5. Config merge optimization — diminishing returns
6. ~~YAML parser~~ — not worth the effort
7. ~~mmap~~ — no effect
