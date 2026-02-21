# Architecture Alternatives: Evaluating turbocop's Approach

## What is turbocop?

turbocop is a Ruby linter written in Rust that aims to be a drop-in replacement
for [RuboCop](https://github.com/rubocop/rubocop), the standard Ruby static
analysis tool. RuboCop is written in Ruby and defines ~915 "cops" (lint rules)
across several gems (rubocop, rubocop-rails, rubocop-rspec, rubocop-performance,
etc.). It reads `.rubocop.yml` config files and checks Ruby source code for
style violations, potential bugs, and performance issues.

turbocop reimplements these cops in Rust, using:
- **[Prism](https://github.com/ruby/prism)** (via `ruby-prism` crate) for Ruby
  parsing — the same parser now built into Ruby 3.3+
- **[rayon](https://github.com/rayon-rs/rayon)** for parallel file processing
  across all CPU cores
- **RuboCop-compatible config** loading (`.rubocop.yml`, `inherit_from`,
  `inherit_gem`, per-cop settings)
- **Result caching** for incremental runs (only re-lint changed files)

The goal is identical output to RuboCop (same offenses, same locations, same
autocorrect) but 10-100x faster. It ships as a single static binary with no
Ruby runtime dependency at lint time.

### Why rewrite in Rust instead of speeding up RuboCop?

RuboCop's performance is bounded by:
- **Ruby startup** (~200-500ms to load the framework and all cops)
- **Single-threaded execution** (Ruby's GIL prevents true parallelism;
  `--parallel` forks processes, which is heavy and doesn't share caches)
- **Object allocation per AST node** (GC pressure on large files)
- **Dynamic dispatch overhead** (NodePattern matching, cop callbacks)

These are fundamental to Ruby's execution model, not fixable by optimizing
RuboCop's Ruby code. A Rust reimplementation sidesteps all of them.

### The porting challenge

RuboCop cops are defined in Ruby using a DSL:
- **NodePattern** — a mini-language for AST matching
  (`(send nil? :puts ...)` matches `puts(...)` calls)
- **Callbacks** — `on_send`, `on_class`, etc. invoked during AST traversal
- **Arbitrary Ruby logic** — conditionals, string manipulation, config lookups
- **Autocorrect lambdas** — code that rewrites the source to fix violations

turbocop must reimplement all of this in Rust. The matching translates
NodePattern definitions to Rust if-let chains. The cop logic is hand-ported.
The autocorrect is reimplemented per-cop.

An additional complication: RuboCop historically used the
[Parser](https://github.com/whitequark/parser) gem which produces a different
AST than Prism. NodePattern definitions reference Parser's node types
(`send`, `const`, `hash`), which map to different (and sometimes split) Prism
node types (`CallNode`, `ConstantReadNode`/`ConstantPathNode`,
`HashNode`/`KeywordHashNode`). This mapping is a significant source of bugs.

## Current State (as of 2026-02-21)

turbocop implements 915 cops in hand-written Rust. Conformance results:

- **Public bench repos (14 repos):** 100% match rate across all repos. Zero FPs, zero FNs.
- **Internal repos:** Mixed. Several at 100%, others with divergence — predominantly
  from gem version mismatches (turbocop firing cops that the project's installed gem
  version doesn't include), not cop logic bugs.

The remaining FP/FN sources:
- **Gem version mismatches** — turbocop applies cops from its vendor submodule version
  (e.g., rubocop-rails 2.34.3) but the project uses an older version (e.g., 2.31.0)
  where a cop doesn't exist yet. The version-awareness system is supposed to catch
  this but has gaps.
- **`Lint/Syntax` differences** — Prism and the Parser gem have different error recovery
  behavior. RuboCop reports syntax offenses that turbocop doesn't.
- **Genuine cop logic edge cases** — patterns not exercised by the public bench repos.
  Each new repo tested tends to surface a handful of these.

### The "New Repo" Problem

When adding a new repo to the bench suite, FPs/FNs typically appear. After investigation
and fixes, conformance reaches 100%. But this cycle repeats with each new repo —
suggesting the public bench repos don't cover the full diversity of Ruby patterns and
config setups found in real-world codebases.

The question: is this converging (each new repo surfaces fewer new issues because
the prior fixes generalize) or is it a treadmill (each repo has unique edge cases)?

### Performance

turbocop is 4-500x+ faster than RuboCop depending on repo size and cache state:

| Scenario | Range | Typical |
|----------|-------|---------|
| Small repo, cached | 20-60ms vs 1-1.2s | ~25x |
| Large repo, partial invalidation | 100-500ms vs 2-6s | ~15x |
| Very large monorepo | 400ms vs 200s | ~500x |

## The Maintenance Question

915 cops x ongoing upstream changes = significant maintenance surface. Each
RuboCop release can:

1. Add new cops (3-10 per minor release)
2. Change existing cop behavior (NodePattern changes, new config keys)
3. Deprecate/rename cops

Currently all of this is handled by manual porting. The question is whether a
different architecture would reduce this burden while preserving the speed advantage.

## Alternative Architectures

### Option A: Stay the Course (Hand-Ported Rust Cops)

**What it is:** Current approach. Every cop is hand-written Rust matching against
Prism AST nodes. Vendor submodules pin specific RuboCop releases. Config audit
and prism pitfall tests catch gaps.

**Strengths:**
- Working today with 100% conformance on public repos
- Maximum performance (pure Rust, zero Ruby overhead)
- Single static binary, no Ruby runtime dependency at lint time
- Full control over every cop's behavior
- Autocorrect can be implemented per-cop with full confidence

**Weaknesses:**
- Linear maintenance cost: every new upstream cop needs manual porting
- Upstream behavior changes require re-reading Ruby source and updating Rust
- 36% of historical bugs came from Parser-to-Prism AST mapping mistakes
- No automated way to verify a hand-ported cop matches Ruby behavior
  (only integration-level conformance benchmarks)
- Each new codebase tested tends to surface new edge cases, requiring
  another round of fixes before conformance is clean

**Maintenance cost:** High per-release, but bounded. ~5-10 new cops per RuboCop
minor release, each taking 30-120 min to port. Behavior changes in existing cops
are harder to detect — requires running conformance benchmarks after each update.

**Long-term scaling:** The initial porting effort (0 to 915 cops) is the hard part.
Incremental maintenance (tracking 5-10 new cops per release, fixing regressions)
is manageable if conformance benchmarks are run regularly. The risk is drift during
periods of low attention.

**Verdict:** Viable if the project has sustained development effort. The
conformance bench is the safety net. Main risk is that the "new repo" edge case
cycle never fully converges.

---

### Option B: NodePattern Codegen (Auto-Generate Matching Layer)

**What it is:** Use the existing `node_pattern_codegen` prototype to automatically
translate RuboCop's `def_node_matcher` / `def_node_search` patterns into Rust
matching functions. Hand-write only the cop logic around the patterns (config
checks, message formatting, autocorrect).

**Current state:** Prototype exists (1,880 lines). Lexer and parser are complete.
Code generator handles simple patterns. Not integrated into the build or the
cop-writing workflow. No cop currently uses it.

**The 82/18 split:** 82% of the ~1,010 patterns in vendor repos are purely
structural (node types, literals, wildcards, alternatives) and can be generated
mechanically. 18% use `#helper_method` calls (47 unique helpers) that need
manual implementation.

**How it would work in practice:**
1. Developer runs `cargo run --bin node_pattern_codegen -- generate vendor/rubocop/lib/rubocop/cop/style/foo.rb`
2. Tool outputs Rust matching functions for each `def_node_matcher` / `def_node_search`
3. Developer pastes into the cop module, fills in `todo!()` helper stubs
4. Developer writes the 5-20 lines of cop logic (config checks, diagnostics, autocorrect)
5. On upstream updates, re-run codegen and diff against existing code

**Strengths:**
- Eliminates the #1 bug source (AST shape mismatches) by construction
- Mapping table defined once, tested once, reused everywhere
- Upstream pattern changes can be re-generated and diffed
- Helpers are shared across cops (47 total, not 915)
- Could be used retroactively to verify existing hand-written matchers

**Weaknesses:**
- Only addresses the matching layer, not cop logic or autocorrect
- Codegen prototype needs work to be production-ready: alternatives on
  node types, literal matching, nil?/cbase, captures are incomplete
- Still requires manual porting of cop logic per cop
- Generated code may be harder to debug than hand-written code
- Build system complexity (build.rs codegen vs checked-in generated code)
- Doesn't help with the "new repo surfaces edge cases" problem — those
  edge cases are usually in cop logic, not in pattern matching

**Maintenance cost:** Lower for matching (re-run codegen on upstream changes),
same for cop logic. Net reduction of maybe 30-40% of porting effort per new cop.

**Verdict:** Worth finishing as a developer tool, but doesn't fundamentally change
the maintenance story. Most conformance issues come from cop logic edge cases
and config handling, not from pattern matching bugs.

---

### Option C: Embedded Ruby (Run Actual Cops via FFI)

**What it is:** Embed CRuby via FFI (e.g., `magnus` or `rutie` crate). Load
RuboCop's cop classes into the embedded interpreter. Parse with Prism in Rust,
convert AST to Ruby objects, run Ruby cop code, collect results back in Rust.

**How it would work:**
1. Rust side: file discovery, config loading, Prism parsing, result caching
2. FFI bridge: convert Prism AST nodes to Ruby AST objects
3. Ruby side: run RuboCop cop classes against the Ruby AST objects
4. FFI bridge: collect diagnostics back as Rust structs

**Strengths:**
- Perfect correctness by definition (running the actual cop code)
- Zero porting effort for existing or new cops
- Zero maintenance for upstream changes (just `bundle update rubocop`)
- Autocorrect is automatically correct

**Weaknesses:**
- Requires Ruby runtime installed (loses single-binary story)
- Ruby cop execution speed: back to Ruby performance for the cop layer
- AST conversion overhead: Prism Rust nodes must be wrapped as Ruby objects
  for every node in every file — potentially millions of allocations
- Ruby GIL prevents true thread parallelism within a single process.
  Would need process forking like RuboCop `--parallel`, losing rayon's
  advantage of shared-memory, low-overhead work stealing
- GC pressure from creating Ruby AST objects, especially on large files
- Complex FFI boundary: error handling, Ruby exception propagation,
  object lifecycle management, GC root registration
- Startup cost: loading RuboCop framework + all cops takes ~200-500ms
- Version coupling: embedded Ruby version must match project's Ruby version
  for compatibility (native extensions, syntax support)

**Performance estimate:** Parsing in Rust (fast) + AST conversion (overhead) +
cop execution in Ruby (slow) + FFI marshaling (overhead). Likely 1.5-3x faster
than RuboCop due to faster parsing, smarter caching, and avoiding RuboCop's
own config/file-discovery overhead. Not the 15-500x we see today.

**Precedent:** `rust-analyzer` embeds proc-macro execution by running a separate
process, not by embedding the Rust compiler. `tree-sitter` parsers generate C
code rather than embedding the grammar runtime. The pattern in the ecosystem is
to avoid embedding language runtimes.

**Verdict:** Defeats the purpose. The speed advantage comes from Rust cop
execution and rayon parallelism, not just from faster parsing. Embedding Ruby
reintroduces the bottleneck that turbocop exists to eliminate.

---

### Option D: Ruby Subprocess Batching (Smart RuboCop Wrapper)

**What it is:** Keep Rust for file discovery, config loading, caching, and
orchestration. Shard files into batches and run RuboCop in long-lived Ruby
subprocesses (one per CPU core) via stdin/stdout JSON protocol.

```
turbocop (Rust)                    rubocop workers (Ruby x N)
  - file discovery                   - load RuboCop once
  - config loading & caching         - receive file batch via stdin
  - result caching (skip             - lint files
    unchanged files entirely)        - return JSON results via stdout
  - output formatting                - stay alive for next batch
  - parallelism orchestration
```

**How it would work:**
1. Rust orchestrator discovers files, checks result cache
2. Uncached files are sharded into N batches (N = CPU cores)
3. Each batch is sent to a persistent RuboCop worker process
4. Workers lint files and return JSON diagnostics
5. Rust side caches results, formats output, handles `--fail-fast`

**Strengths:**
- Perfect conformance (running actual RuboCop cops)
- True parallel execution via multiple processes (avoids GIL)
- Amortized startup: each worker loads RuboCop once, processes many batches
- Result caching still works — Rust side skips unchanged files entirely,
  so incremental runs only send changed files to Ruby workers
- Smarter than `rubocop --parallel` (which forks per-run, no caching)
- Could be much faster than RuboCop for incremental runs (cache hits
  are pure Rust, zero Ruby involvement)
- Autocorrect works correctly (RuboCop does the rewriting)

**Weaknesses:**
- Requires Ruby + bundled RuboCop gem installed
- Process startup: N Ruby processes x ~500ms each on first run
  (amortized across files, but adds latency for small repos)
- Memory: N Ruby processes x ~100-200MB each
- IPC protocol complexity: serializing file paths, config overrides,
  results. Need to handle worker crashes, timeouts, encoding issues
- Loses the single-binary distribution story
- Cold run speed bounded by Ruby: even with N workers, each worker
  processes files at Ruby speed (~5-50ms per file depending on size)
- Config synchronization: Rust and Ruby must agree on which cops are
  enabled, what config overrides apply. Divergence = subtle bugs

**Performance estimate:**
- Cold run (no cache): N workers x Ruby speed. For 2000 files on 8 cores,
  ~250 files/worker at ~20ms each = ~5s. Similar to `rubocop --parallel`.
- Warm incremental run (90% cache hits): 200 uncached files across 8
  workers = ~0.5s Ruby time + Rust overhead. Maybe 3-5x faster than
  stock RuboCop for incremental, 1-2x for cold runs.
- Fully cached: pure Rust, ~50-100ms. Same as current turbocop.

**Verdict:** A smarter RuboCop wrapper, not a replacement. The fully-cached
path matches current turbocop speed. The cold path is bounded by Ruby.
Interesting if the goal shifts from "replace RuboCop" to "make RuboCop
faster with caching and parallelism."

---

### Option E: Hybrid (Rust Fast Path + RuboCop Fallback)

**What it is:** Implement a curated subset of cops in Rust (the most common,
well-understood ones). For remaining cops, fall back to a RuboCop subprocess.
Report combined results.

```
turbocop --hybrid .
  Phase 1: Rust cops lint all files (fast, parallel)
  Phase 2: RuboCop subprocess lints all files for remaining cops
  Phase 3: Merge and deduplicate results
```

**How it would work:**
1. Classify cops as "Rust-native" or "Ruby-fallback"
2. Run Rust cops in parallel (current architecture)
3. Run RuboCop with `--only <ruby-fallback-cops>` for remaining cops
4. Merge results by file + line + cop name
5. Cache both Rust and Ruby results independently

**Strengths:**
- Fast for the common case (most offenses come from a small set of cops)
- Perfect correctness for the long tail (running actual RuboCop)
- Incremental migration: move cops from Ruby to Rust as confidence grows
- Reduces maintenance to the curated Rust subset
- Could start with 100-200 high-confidence cops in Rust and grow

**Weaknesses:**
- Two code paths = double the testing surface
- Still needs Ruby installed for the fallback
- Users see two different performance profiles depending on which cops fire
- Result merging is subtle: file ordering, severity reconciliation,
  config interactions between Rust and Ruby paths
- The RuboCop subprocess still needs full config loading, so the "Ruby
  fallback" isn't just for a few cops — it's a full RuboCop run with
  a filtered cop list
- Minimum latency = max(Rust path, Ruby path), so the slow path dominates
  unless the Ruby fallback cops are rarely relevant
- Autocorrect across two paths: which tool rewrites the file? Conflicts?
- Hard to explain to users ("some cops are fast, some aren't")

**Performance estimate:** If 200 cops are Rust-native and 715 fall back:
Rust phase completes in ~100ms. Ruby phase runs 715 cops and takes ~2-4s.
Total: ~2-4s — no better than RuboCop for most users, since the Ruby phase
dominates. Only faster if most repos only trigger Rust-native cops.

**A better variant:** Instead of running both phases on every invocation,
make the Ruby fallback opt-in (`turbocop --strict` runs both, default runs
only Rust cops). This gives fast-by-default with an option for full coverage.
But then the default mode has coverage gaps, which is confusing.

**Verdict:** Architecturally messy and hard to reason about. The performance
win disappears as soon as Ruby enters the picture. Better as a migration
strategy than a steady-state architecture.

---

### Option F: RuboCop Server Mode (Leverage RuboCop's Built-in Daemon)

**What it is:** RuboCop 1.31+ includes `rubocop --server` which starts a
long-lived daemon process. Subsequent invocations connect to the daemon via
Unix socket, avoiding Ruby startup and gem loading overhead. turbocop could
use this as its Ruby backend instead of managing subprocess workers directly.

**How it would work:**
1. Ensure `rubocop --server` is running (start if needed)
2. Send lint requests via the RuboCop server protocol
3. Server returns results without restart overhead
4. turbocop adds its caching layer on top

**Strengths:**
- Uses RuboCop's own optimized server protocol
- No subprocess management complexity
- RuboCop team maintains the server mode
- Startup cost eliminated after first run

**Weaknesses:**
- RuboCop server mode is designed for editor integration, not batch linting.
  It processes one file at a time and doesn't expose a batch API
- Still single-threaded Ruby execution within the server
- Server protocol is an implementation detail, not a stable public API
- Limited control over parallelism (one server = one Ruby thread)
- Requires RuboCop >= 1.31

**Performance estimate:** Similar to Option D but without the parallelism
benefit of multiple worker processes. Single-threaded Ruby execution means
one file at a time. For 2000 files at ~20ms each = ~40s (worse than
`rubocop --parallel`).

**Verdict:** Not useful for batch linting. RuboCop server mode is designed
for single-file editor checks, not the batch use case turbocop targets.

---

### Option G: Compile RuboCop Cops to Native Code (GraalVM / TruffleRuby)

**What it is:** Run RuboCop on TruffleRuby (the GraalVM Ruby implementation)
which JIT-compiles Ruby to native machine code. RuboCop cops run at near-native
speed after warmup, without any porting effort.

**How it would work:**
1. Install TruffleRuby instead of CRuby
2. Run standard RuboCop via TruffleRuby
3. After JIT warmup, cop execution approaches native speed
4. turbocop wrapper handles caching, parallelism, output formatting

**Strengths:**
- Zero porting effort (running stock RuboCop)
- Zero maintenance for upstream changes
- JIT-compiled Ruby can approach 50-80% of native C speed for
  compute-bound code after warmup
- TruffleRuby supports real threading (no GIL equivalent for Ruby code,
  though IO may still be serialized)

**Weaknesses:**
- TruffleRuby startup is slow (~2-5s just for the runtime)
- JIT warmup: first ~100-200 files are slow while the JIT compiles
  hot paths. Peak performance only after processing many files
- GraalVM is a large dependency (~500MB)
- TruffleRuby compatibility: not all Ruby gems work. RuboCop itself
  may have issues (C extensions, FFI, native gems)
- Memory: GraalVM uses significantly more memory than CRuby (~500MB-1GB
  for the runtime alone)
- Ecosystem: TruffleRuby is a niche runtime. Requiring users to install
  it is a significant adoption barrier
- Parallelism story is unclear: TruffleRuby's threading model is
  different from CRuby's. RuboCop wasn't designed for it

**Performance estimate after warmup:**
- Parsing: similar to CRuby (Prism is already fast)
- Cop execution: potentially 2-5x faster than CRuby after JIT warmup
- Total: maybe 2-5x faster than CRuby RuboCop, but with 2-5s startup penalty
- Still 5-50x slower than turbocop's pure Rust path

**Verdict:** Interesting academically but impractical. The startup penalty,
memory usage, and ecosystem friction make it worse than just running CRuby
RuboCop. And it still can't match Rust performance.

---

### Option H: Tree-Sitter Based Matching

**What it is:** Instead of Prism, use tree-sitter's Ruby grammar for parsing.
Tree-sitter has a built-in S-expression query language that's conceptually
similar to NodePattern. Cops could be expressed as tree-sitter queries with
Rust callback handlers.

**How it would work:**
1. Parse Ruby files with tree-sitter-ruby (Rust bindings via `tree-sitter` crate)
2. Express cop patterns as tree-sitter S-expression queries
3. Run queries against the parse tree; tree-sitter handles matching
4. Rust callbacks process matches: check config, emit diagnostics, autocorrect

**Example tree-sitter query (equivalent to NodePattern):**
```scheme
;; Style/NegatedIf: `if !condition` -> `unless condition`
(if
  condition: (unary
    operator: "!"
    operand: (_) @cond)
  consequence: (_) @body) @if_node
```

**Strengths:**
- Tree-sitter queries are a well-tested, well-documented pattern language
- The tree-sitter runtime is written in C, extremely fast
- Incremental parsing: tree-sitter can re-parse only changed regions,
  potentially faster than full Prism re-parse for editor integration
- Tree-sitter is widely adopted (GitHub, Neovim, Helix, Zed all use it)
- Query predicates (#match?, #eq?) handle some cop logic directly

**Weaknesses:**
- **Different AST shape.** Tree-sitter-ruby produces a CST (concrete syntax
  tree), not an AST. It preserves all tokens including whitespace, commas,
  parens. This is a fundamentally different tree than Prism's AST. Every
  cop would need re-porting to the tree-sitter node types
- **Query expressiveness gap.** Tree-sitter queries can match structural
  patterns but lack NodePattern features like recursive descent (`\`),
  computed predicates, and helper method calls. Complex cops still need
  Rust logic
- **Prism is the canonical Ruby parser.** Prism is developed by the Ruby
  core team (Shopify), integrated into Ruby 3.3+, and is the parser
  RuboCop is migrating to. Tree-sitter-ruby is a third-party grammar
  that may not handle all Ruby edge cases identically
- **Two porting efforts.** Would need to re-port all 915 cops from
  Prism node types to tree-sitter node types. This is a lateral move,
  not a simplification
- **Autocorrect complexity.** Tree-sitter's CST includes exact byte
  offsets for every token, which is good for autocorrect. But the cop
  logic that decides *what* to correct still needs hand-porting

**Performance estimate:** Tree-sitter parsing is ~2-5x faster than Prism for
initial parse, but the bottleneck is cop execution, not parsing. Query
execution is fast but limited — complex cops still need Rust callbacks.
Net: similar performance to current architecture.

**Verdict:** A lateral move that solves the wrong problem. The parsing layer
isn't the bottleneck or the bug source. Switching parsers means re-porting
all cops to a different AST shape without gaining correctness or reducing
maintenance. Prism is the better choice given RuboCop's own Prism migration.

---

### Option I: NodePattern Interpreter (Run Patterns Directly, No Codegen)

**What it is:** Instead of compiling NodePattern to Rust code (Option B) or
hand-porting (Option A), implement a NodePattern interpreter in Rust that
reads pattern strings at runtime and matches them against Prism AST nodes.

**How it would work:**
1. At startup, read `def_node_matcher` / `def_node_search` patterns from
   vendor Ruby cop files (or from a pre-extracted pattern database)
2. Parse patterns into an in-memory representation (the existing lexer/parser
   from `node_pattern_codegen` already does this)
3. At lint time, run the pattern interpreter against each AST node
4. Cop logic (config, messages, autocorrect) is still hand-written Rust,
   but instead of hand-written matchers, cops call `pattern.matches(node)`

**Example:**
```rust
impl SomeNewCop {
    fn check_node(&self, node: &Node, ...) {
        // Pattern loaded from vendor/rubocop at startup
        static PATTERN: LazyLock<Pattern> = LazyLock::new(||
            Pattern::parse("(send (const nil? :Struct) :new ...)")
        );
        if PATTERN.matches(node) {
            // Hand-written cop logic here
        }
    }
}
```

**Strengths:**
- Patterns stay in sync with upstream automatically (read from vendor source)
- No codegen build step — patterns are interpreted at runtime
- The existing lexer/parser prototype handles the parsing
- Changes to upstream patterns are picked up by updating the vendor submodule
  and re-running — no Rust code changes needed for the matching layer
- Cop logic is still hand-written Rust, keeping full performance for the
  non-matching parts
- Easier to debug than generated code (can print pattern + node mismatch)
- Could add a "verify" mode: run both the interpreter and hand-written
  matcher, assert they agree

**Weaknesses:**
- Interpreter overhead: pattern matching involves dynamic dispatch
  (interpreting the pattern tree) vs static dispatch (compiled Rust code).
  Maybe 3-10x slower per match than compiled code
- The interpreter needs to handle the full NodePattern feature set:
  alternatives, captures, negation, predicates, helper calls, recursion.
  This is essentially building a small VM
- Helper method calls (`#helper?`) still need Rust implementations
- Cop logic still needs hand-porting (same as Option A/B)
- Performance impact depends on how hot the matching path is. If matching
  is 10% of cop execution time, a 5x slowdown = 40% overall slowdown.
  If matching is 50% of cop time, it's a 2.5x overall slowdown

**Performance estimate:** Depends heavily on pattern complexity. Simple
patterns (type check + child check) add ~100-500ns per match vs ~10-50ns
for compiled code. For 2000 files x 915 cops x ~10 nodes per file = ~18M
pattern evaluations. At 200ns each = ~3.6s interpreter overhead. At 30ns
each (compiled) = ~0.5s. This is significant but not fatal — the overhead
could be mitigated by only interpreting new/changed patterns and keeping
hand-written matchers for hot cops.

**A hybrid approach:** Use the interpreter for correctness verification
(assert interpreter agrees with hand-written matcher) rather than as the
primary matching engine. This gives the best of both worlds: hand-written
performance + automated correctness checking.

**Verdict:** The interpreter-as-verifier variant is compelling. It doesn't
change the runtime architecture but adds an automated correctness check
that catches the 36% of bugs from AST mapping mistakes. Could be integrated
into `cargo test` as a verification pass.

---

## Comparison Matrix

| | Speed | Correctness | Maintenance | Single Binary | Complexity |
|---|---|---|---|---|---|
| **A: Hand-ported Rust** | Best (15-500x) | High (100% on public bench) | High | Yes | Low |
| **B: NodePattern codegen** | Best | Higher (fewer matching bugs) | Medium | Yes | Medium |
| **C: Embedded Ruby** | Poor (1.5-3x) | Perfect | None | No | High |
| **D: Subprocess batching** | OK cached, poor cold (2-5x) | Perfect | None | No | High |
| **E: Hybrid** | Bounded by Ruby path | Perfect for fallback | Medium | No | Very High |
| **F: RuboCop server** | Poor (single-threaded) | Perfect | None | No | Medium |
| **G: TruffleRuby** | OK (2-5x after warmup) | Perfect | None | No | Low (but large dep) |
| **H: Tree-sitter** | Same as A | Lateral move | Same as A | Yes | High (re-port all) |
| **I: Pattern interpreter** | Good (minor overhead) | High + verifiable | Medium-Low | Yes | Medium |

## Recommendation

The data doesn't support a fundamental architecture change. The current approach
(Option A) is working — 100% conformance on public benchmarks, 15-500x speed
advantage, single binary distribution.

The FP/FN reports from new repos are real but they're predominantly from:
1. **Gem version mismatches** (config bug, not architecture problem)
2. **`Lint/Syntax` differences** (known Prism vs Parser gap, bounded)
3. **Cop logic edge cases** (fixable incrementally, converging over time)

None of these indicate a fundamental problem with hand-porting.

### What would actually help

1. **Fix the gem version mismatch problem.** This is the #1 source of FPs on
   internal repos. The version-awareness system should be catching these.
   Investigate the gaps.

2. **Invest in the conformance bench.** Add more diverse repos to the bench
   suite. The public repos are at 100% — the signal is in repos with unusual
   config setups, monorepo layouts, and older gem versions.

3. **Build the pattern interpreter as a verifier (Option I hybrid).**
   Don't replace hand-written matchers — verify them. Run `cargo test` with
   a pass that checks "does the hand-written Rust matcher agree with the
   NodePattern interpreter on a corpus of AST nodes?" This catches the 36%
   of bugs from AST mapping mistakes automatically, without runtime overhead.

4. **Consider NodePattern codegen (Option B) as a developer tool.** Use it
   to generate first drafts of new cops, reducing manual porting effort.
   Not as a replacement for hand-written code, but as a starting point.

5. **Monitor RuboCop's Prism migration (Option F context).** When upstream
   cops target Prism natively, the Parser-to-Prism mapping problem disappears.
   This simplifies both hand-porting and the codegen/interpreter.

6. **Don't reintroduce Ruby into the hot path (Options C/D/E/F/G).** The
   speed advantage is turbocop's core value proposition. Every option that
   involves Ruby execution converges to "slightly faster RuboCop" — not
   the 15-500x that makes the project compelling.
