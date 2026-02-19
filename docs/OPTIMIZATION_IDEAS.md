# rblint Optimization Ideas

## Current Performance Profile (2026-02-18)

Benchmarked with `.rblint.cache` (no bundler calls) and batched single-pass AST walker.

### Phase Breakdown by Repo

| Repo | Files | Config | File I/O | Parse | Cop Exec | Wall |
|------|------:|-------:|---------:|------:|---------:|-----:|
| discourse | 5895 | 162ms | 357ms | 374ms | 4.0s | 662ms |
| rails | 3320 | 119ms | 502ms | 343ms | 4.0s | 554ms |
| mastodon | 2540 | 89ms | 522ms | 164ms | 14.0s | 1.09s |
| chatwoot | 2247 | 173ms | 585ms | 174ms | 13.0s | 1.17s |
| rubocop | 1669 | 722ms | 528ms | 222ms | 20.0s | 2.72s |
| rubygems.org | 1239 | 20ms | 234ms | 85ms | 5.0s | 496ms |
| docuseal | 409 | 14ms | 179ms | 43ms | 2.0s | 269ms |
| activeadmin | 374 | 13ms | 47ms | 28ms | 150ms | 42ms |
| good_job | 172 | 13ms | 46ms | 28ms | 1.0s | 191ms |
| errbit | 217 | 13ms | 38ms | 15ms | 994ms | 131ms |

Note: File I/O, Parse, and Cop Exec are **cumulative across threads**. Wall time is
actual elapsed time (parallelized across ~10 cores). Config loading is serial.

### Key Insight

**Cop execution dominates**: 75-95% of cumulative thread time, 60-85% of wall time.
The batched walker calls all 745 `check_node` cops for every AST node, but each cop
only cares about 1-3 node types. ~95% of calls are no-ops returning `Vec::new()`.

---

## Optimization 1: Node-Type Dispatch Table (HIGH IMPACT)

**Status:** Not started
**Expected impact:** 30-50% reduction in cop execution time

### Problem

The `BatchedCopWalker` iterates over all 745 AST-checking cops for every node:

```rust
fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
    for &(cop, cop_config) in &self.cops {
        let results = cop.check_node(...);  // 745 vtable calls per node
        self.diagnostics.extend(results);
    }
}
```

A typical file has 1,000-3,000 AST nodes. That's 745,000-2,235,000 dynamic dispatch
calls per file, of which ~95% immediately return `Vec::new()`.

### Solution

Add a `fn node_types(&self) -> &'static [NodeType]` method to the `Cop` trait. Build a
dispatch table mapping each Prism node type to the cops that handle it. The walker
only calls cops registered for the current node's type.

```rust
// Each cop declares what it cares about:
fn node_types(&self) -> &'static [NodeType] {
    &[NodeType::CallNode]  // Only called for method calls
}

// Walker dispatches selectively:
fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
    let tag = node_type_tag(&node);
    if let Some(cops) = self.dispatch_table.get(&tag) {
        for &(cop, cop_config) in cops {
            cop.check_node(...);
        }
    }
}
```

### Implementation

1. Define `NodeType` enum matching Prism's node variants
2. Add `fn node_types(&self) -> &'static [NodeType]` to Cop trait (default: `&[]` = all)
3. Annotate 745 cops with their node types (mechanical — each already does `as_*_node()`)
4. Build dispatch HashMap in walker constructor
5. Add validation test: ensure declared node_types match actual `as_*_node()` usage

### Effort

High (745 cops to annotate), but each annotation is mechanical:
- Read cop's `check_node`, find the `as_*_node()` calls, add to `node_types()`
- A codegen script could auto-generate most annotations from source analysis

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

## Optimization 3: Skip Fully-Disabled Departments (LOW IMPACT)

**Status:** Not started
**Expected impact:** 5-15% reduction for projects that disable departments

### Problem

All 915 registered cops are checked for enablement individually. If an entire
department is disabled (e.g., `RSpec:` cops in a non-RSpec project), we still iterate
over those cops during walker construction.

### Solution

Pre-filter cops by department enablement before building the walker's cop list.
Currently the config already tracks department-level `Enabled: false`, but cop
filtering happens per-cop. A department-level pre-filter would skip ~100-200 cops
for projects that don't use RSpec, Rails, etc.

### Effort

Low — check department enablement once, skip all cops in disabled departments.

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

1. **Node-type dispatch table** — highest impact, reduces the dominant bottleneck
2. **Eliminate per-call Vec allocation** — medium impact, synergizes with #1
3. **Skip disabled departments** — low effort, quick win
4. Config merge optimization — diminishing returns
5. ~~YAML parser~~ — not worth the effort
6. ~~mmap~~ — no effect
