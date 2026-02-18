# Optimization Ideas

Performance optimization ideas for rblint, inspired by [qj](https://github.com/6/qj)'s approach to making a Rust CLI tool dramatically faster than its Ruby/C predecessor.

qj's core techniques — single-pass processing, on-demand field extraction, and zero-copy I/O — have direct analogs in a linter's architecture. These ideas are ordered by expected impact.

## Current performance baseline

| Repo | .rb files | rblint | rubocop | Speedup |
|------|----------:|-------:|--------:|--------:|
| mastodon | 2524 | 1.61s | 2.49s | 1.5x |
| discourse | 5831 | 1.26s | 3.45s | 2.7x |

rblint has 743 registered cops. Of these, 620 implement `check_node` (AST-based), 91 implement `check_source`, and 36 implement `check_lines`.

---

## 1. Batched Multi-Cop Walker

**qj parallel:** qj walks JSON data once, extracting all needed fields in a single pass. rblint currently walks the AST 620 separate times per file — once per AST-based cop.

### Current behavior

In `lint_source_inner` (`src/linter.rs`), for each file:

```rust
for (i, cop) in registry.cops().iter().enumerate() {
    // ... filtering ...
    diagnostics.extend(cop.check_lines(source, &cop_config));
    diagnostics.extend(cop.check_source(source, &parse_result, &code_map, &cop_config));

    // This creates a fresh walker and traverses the ENTIRE AST
    let mut walker = CopWalker { cop: &**cop, source, parse_result, ... };
    walker.visit(&parse_result.node());
    diagnostics.extend(walker.diagnostics);
}
```

For a file with N AST nodes: 620 full recursive tree traversals = 620*N node visits. Each traversal involves recursive function calls down the tree, virtual dispatch at each child, and pointer dereferencing through the entire AST structure. The AST is loaded into CPU cache, evicted, then reloaded 619 more times.

### Proposed change

Walk the AST once. At each node, dispatch to all eligible cops:

```rust
// src/cop/walker.rs — new struct alongside existing CopWalker

pub struct BatchedCopWalker<'a, 'pr> {
    cops: Vec<(&'a dyn Cop, &'a CopConfig)>,
    source: &'a SourceFile,
    parse_result: &'a ruby_prism::ParseResult<'pr>,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for BatchedCopWalker<'_, 'pr> {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        for (cop, config) in &self.cops {
            self.diagnostics.extend(
                cop.check_node(self.source, &node, self.parse_result, config)
            );
        }
    }

    fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        for (cop, config) in &self.cops {
            self.diagnostics.extend(
                cop.check_node(self.source, &node, self.parse_result, config)
            );
        }
    }
}
```

And restructure `lint_source_inner`:

```rust
let mut diagnostics = Vec::new();
let mut ast_cops: Vec<(&dyn Cop, CopConfig)> = Vec::new();

for (i, cop) in registry.cops().iter().enumerate() {
    // ... same filtering as before ...
    let cop_config = config.cop_config_for_file(name, &source.path);
    diagnostics.extend(cop.check_lines(source, &cop_config));
    diagnostics.extend(cop.check_source(source, &parse_result, &code_map, &cop_config));
    ast_cops.push((&**cop, cop_config));
}

// Single AST walk for all cops
if !ast_cops.is_empty() {
    let refs: Vec<_> = ast_cops.iter().map(|(c, cfg)| (*c, cfg)).collect();
    let mut walker = BatchedCopWalker { cops: refs, source, parse_result: &parse_result, diagnostics: Vec::new() };
    walker.visit(&parse_result.node());
    diagnostics.extend(walker.diagnostics);
}
```

### Why this works with zero cop changes

- The `Cop` trait and `check_node` signature are unchanged
- Cops with the default no-op `check_node` still get called but return an empty Vec immediately — negligible cost compared to eliminating 619 tree traversals
- Diagnostic ordering changes (node-order instead of cop-order within a file), but the existing `sort_by(sort_key)` at the end of `run_linter` normalizes output to be identical
- No cop depends on running before/after another cop (the trait is stateless)

### Difficulty: Low

~80 lines of new/modified code across 2 files (`walker.rs`, `linter.rs`). Zero changes to any of the 743 cop implementations. The only subtlety is CopConfig ownership — configs must be collected into a Vec before the walk so the walker can borrow them. The code sketch above handles this.

### Worth it: Yes, clearly

This is the single highest-impact optimization available. It eliminates 619 redundant recursive tree traversals per file, which means:
- 619 fewer full recursive descents (function call overhead, stack pressure)
- Massively better CPU cache locality (the AST stays hot in L1/L2 cache during one walk instead of being evicted and reloaded 619 times)
- Less branch predictor pollution

Conservative estimate: **20-40% wall-clock improvement**. The actual gain depends on the ratio of tree-traversal overhead to check_node logic, which profiling would pin down. Even at the low end, getting 20% faster by changing 80 lines is an excellent trade.

The risk is near-zero since the Cop trait doesn't change and output is deterministic after sorting.

---

## 2. Node-Type Dispatch Table

**qj parallel:** qj's on-demand API extracts only the fields a filter needs, bypassing full document tree construction. Most rblint cops only care about 1-3 of the 151 Prism node types, but every cop is called for every node.

### Current behavior

After the batched walker (idea #1), each node visit dispatches to all ~620 cops. The vast majority immediately return an empty Vec because the node type doesn't match:

```rust
// Typical cop check_node — immediately bails on wrong node type
fn check_node(&self, source: &SourceFile, node: &Node<'_>, ...) -> Vec<Diagnostic> {
    let Some(call) = node.as_call_node() else { return vec![] };
    // ... actual logic for CallNode only ...
}
```

Node type interest is heavily concentrated:
- ~362 cops check for `CallNode`
- ~40 cops check for `DefNode`
- ~32 cops check for `IfNode`
- ~30 cops check for `ClassNode`
- Remaining cops spread across ~40 other types

For a `SymbolNode`, only ~2 cops care, but all 620 are called.

### Proposed change

Add a method to the `Cop` trait:

```rust
pub trait Cop: Send + Sync {
    // ... existing methods ...

    /// Node types this cop checks in check_node. None = all nodes (wildcard).
    fn interested_node_types(&self) -> Option<&'static [u16]> {
        None
    }
}
```

Build a dispatch table in the batched walker:

```rust
struct DispatchTable {
    /// type_id -> indices into cops vec
    by_type: Vec<Vec<usize>>,  // indexed by node type u16 (~151 entries)
    /// cops that returned None (interested in all types)
    wildcard: Vec<usize>,
}
```

At each node, look up the type and only dispatch to interested cops:

```rust
fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
    let type_id = node.node_type_id();
    for &idx in &self.dispatch.by_type[type_id as usize] {
        let (cop, config) = &self.cops[idx];
        self.diagnostics.extend(cop.check_node(self.source, &node, self.parse_result, config));
    }
    for &idx in &self.dispatch.wildcard {
        // same
    }
}
```

### Difficulty: Medium

The infrastructure (~100 lines) is straightforward. The bulk of the work is adding `interested_node_types` to each of the 620 AST cops. This is mechanical — look at the first `as_xxx_node()` call in each cop's `check_node` — but it's 620 files. A script could generate most of it by parsing the match patterns, and the ~36 cops with complex/indirect node matching can return `None` (wildcard) as a safe default.

A CI test should verify that declared interests match actual `as_xxx_node()` usage to catch drift.

### Worth it: Probably, but profile first

After idea #1, the dominant cost shifts from tree traversal to the per-node cop dispatch loop. Whether that loop is the new bottleneck depends on how expensive "call check_node on 620 cops that immediately return empty Vec" actually is.

The no-op case is: one virtual function call (dynamic dispatch through `dyn Cop`) → one `as_xxx_node()` attempt (effectively a type tag comparison) → return empty Vec. This is fast — maybe 5-10ns per cop per node. For a file with 500 nodes: 620 × 500 × ~7ns ≈ 2ms overhead per file. Across 5831 files (Discourse): ~12s total... wait, that's actually significant relative to the 1.26s total. But much of this is pipelined by the CPU and the branch predictor gets very good at predicting the no-match path.

**Honest take:** The dispatch table would reduce per-node work from 620 calls to ~10-20 calls on average — a 30-60x reduction in dispatch overhead. But the *absolute* overhead might only be 100-300ms out of 1.26s. It's worth doing if profiling confirms dispatch is a measurable chunk, but don't expect it to cut runtime in half. Implement idea #1 first, profile, then decide.

The 620-file annotation burden is the real cost. It's mechanical but annoying.

---

## 3. Push-Based Diagnostics

**qj parallel:** qj avoids intermediate allocations by writing output directly to a buffer. rblint's `check_node` returns `Vec<Diagnostic>` — almost always empty — which means millions of empty Vec constructions and drops per run.

### Current behavior

```rust
fn check_node(&self, ...) -> Vec<Diagnostic> {
    let Some(call) = node.as_call_node() else { return vec![] };
    // ...
    vec![self.diagnostic(source, line, col, msg)]
}
```

Every call allocates and returns a `Vec<Diagnostic>`. The `vec![]` macro for an empty vec doesn't heap-allocate (Rust optimizes empty Vecs), but there's still the overhead of constructing and destructing a zero-capacity Vec, plus `extend()` on the receiving side. Across 620 cops × N nodes × thousands of files, this adds up.

### Proposed change

Change the signature to push directly into a shared output:

```rust
fn check_node(
    &self,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    parse_result: &ruby_prism::ParseResult<'_>,
    config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,  // push into this
) {
    let Some(call) = node.as_call_node() else { return };
    // ...
    diagnostics.push(self.diagnostic(source, line, col, msg));
}
```

The batched walker passes its `diagnostics` Vec directly:

```rust
fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
    for (cop, config) in &self.cops {
        cop.check_node(self.source, &node, self.parse_result, config, &mut self.diagnostics);
    }
}
```

Same change applies to `check_lines` and `check_source`.

### Difficulty: Medium-High

The infrastructure change is trivial (trait signature update). But it requires updating all 743 cop implementations — every `check_node`, `check_lines`, and `check_source` method. Each change is simple (replace `return vec![...]` with `diagnostics.push(...)`, replace `return vec![]` with `return`), but touching 743 files is tedious and error-prone.

### Worth it: Marginal, do it if you're already touching every cop

An empty `Vec::new()` in Rust is a zero-cost no-op (no heap allocation, just sets ptr/len/cap to 0). The `extend()` of an empty iterator is also nearly free. The real overhead is in the non-empty case: a `vec![diagnostic]` allocates 24+ bytes on the heap, copies the Diagnostic in, returns it, then `extend()` copies it again into the target Vec. Pushing directly saves one allocation + copy per diagnostic emitted.

But diagnostics are rare — the typical file has maybe 0-10 across all cops. The hot path (no diagnostic) already avoids allocation. The savings are probably in the low single-digit milliseconds across a full run.

**Honest take:** This is a clean API improvement that happens to be slightly faster. The performance gain is tiny. Do it if you're already doing idea #2 (touching every cop file anyway), but don't do it on its own just for performance.

---

## 4. mmap for File I/O

**qj parallel:** qj uses mmap with progressive munmap for zero-copy file access, avoiding the allocation + memcpy of reading files into userspace buffers. This is one of qj's biggest wins for large NDJSON files (3.4GB).

### Current behavior

```rust
// src/parse/source.rs
pub fn from_path(path: &Path) -> Result<Self> {
    let content = std::fs::read(path)?;  // allocates Vec<u8>, copies file contents
    let line_starts = compute_line_starts(&content);
    Ok(Self { path: path.to_path_buf(), content, line_starts })
}
```

For each of the 5831 files in Discourse: one `Vec<u8>` allocation + one full file copy from kernel buffer to userspace.

### Proposed change

Use `memmap2` for files above a size threshold:

```rust
use memmap2::Mmap;

enum ContentStorage {
    Owned(Vec<u8>),
    Mapped(Mmap),
}

impl SourceFile {
    pub fn from_path(path: &Path) -> Result<Self> {
        let metadata = std::fs::metadata(path)?;
        let storage = if metadata.len() > 65536 {
            let file = std::fs::File::open(path)?;
            let mmap = unsafe { Mmap::map(&file)? };
            ContentStorage::Mapped(mmap)
        } else {
            ContentStorage::Owned(std::fs::read(path)?)
        };
        let line_starts = compute_line_starts(storage.as_bytes());
        Ok(Self { path: path.to_path_buf(), storage, line_starts })
    }

    pub fn as_bytes(&self) -> &[u8] {
        match &self.storage {
            ContentStorage::Owned(v) => v,
            ContentStorage::Mapped(m) => m,
        }
    }
}
```

`Mmap` implements `Deref<Target=[u8]>` and is `Send + Sync`, so it drops into existing code. Prism's parser takes `&[u8]`, so it works transparently.

### Difficulty: Low

~30 lines in `source.rs`. Add `memmap2` to Cargo.toml. The `unsafe` in `Mmap::map` is well-established and safe as long as the file isn't modified during parsing (which it won't be — rblint doesn't modify source files).

### Worth it: Probably not

Unlike qj, which processes multi-gigabyte files where mmap is transformative, rblint processes thousands of small Ruby files. The average Ruby file is maybe 2-10KB. At that size, `std::fs::read()` is essentially a single page read + memcpy, and the kernel's page cache means the actual disk I/O is amortized across runs.

The Discourse benchmark processes 5831 files in 1.26s. Even if file I/O is 10% of that (~126ms), cutting it in half with mmap saves ~60ms. That's measurable but not meaningful.

**Honest take:** The implementation is trivial and risk-free. It might shave 50-100ms off a large codebase run. Worth doing if you've already done ideas #1-2 and want to squeeze out every last bit, but on its own it's not going to move the needle. The reason mmap is transformative for qj (190x speedup on NDJSON) is that qj processes gigabytes of data where avoiding the copy matters. rblint processes megabytes total.

---

## Summary

| Idea | Impact | Difficulty | Verdict |
|------|--------|------------|---------|
| 1. Batched walker | High (20-40%) | Low (~80 lines, 2 files) | Do it. Best ROI in the codebase. |
| 2. Node-type dispatch | Medium (5-15%) | Medium (620 files) | Profile after #1, then decide. |
| 3. Push-based diagnostics | Low (<5%) | Medium-High (743 files) | Only if already touching every cop. |
| 4. mmap | Low (<5%) | Low (~30 lines) | Trivial to add, marginal gain. |

The recommended path: implement #1, benchmark, profile to find the next bottleneck, then decide if #2 is worth the annotation effort. #3 and #4 are nice-to-haves.
