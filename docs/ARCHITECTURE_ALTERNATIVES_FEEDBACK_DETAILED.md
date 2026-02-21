# turbocop Implementation Plan vNext (Implementer Spec)

## Glossary

* **Baseline**: the vendored RuboCop + plugin snapshot turbocop targets (versions are part of turbocop’s identity).
* **Cop**: a rule.
* **Tier**: `stable` or `preview`.
* **Skipped cop**: referenced/enabled by config but not run (preview-gated, unimplemented, outside baseline).

---

## 1) Product contract (must be in README + `--help`)

### Hard guarantees

* turbocop reads `.rubocop.yml` (RuboCop-style config) and supports a documented subset of RuboCop config semantics.
* turbocop’s behavior is defined against turbocop’s **baseline versions**, not the repo’s Gemfile.lock.
* turbocop supports two tiers: `stable` (default) and `preview` (opt-in).

### Explicit non-guarantees

* Not guaranteed to match RuboCop for arbitrary plugin versions or edge cases.
* `verify` requires Ruby (and/or Bundler); turbocop remains single-binary for `check`.

---

## 2) CLI surface (exact behavior)

### `turbocop check [PATH]`

**Purpose**: run linting using `.rubocop.yml`.

**Flags**

* `--preview` (bool): allow running preview-tier cops.
* `--strict[=SCOPE]`: skipped cops cause a coverage failure exit code.
  Scope: `coverage` (default), `implemented-only`, `all`. See section 5.
* `--fail-level <level>`: set failure threshold. Levels (example): `refactor|convention|warning|error|fatal`.
* `--format <text|json>`: output format.
* `--quiet-skips` (bool): suppress grouped skip notice.
* `-a` / `--autocorrect=safe`: apply safe corrections only.
* `-A` / `--autocorrect=all`: apply all corrections including unsafe.
  (Matches current CLI; `off` is the default when neither flag is given.)

**Algorithm**

1. Discover config root (see config section).
2. Load `.rubocop.yml` + inherits (`inherit_from`, `inherit_gem` if supported).
3. Determine enabled cops and per-cop settings.
4. For each enabled cop:

   * If not in baseline → mark `skipped_outside_baseline`.
   * Else if not implemented → mark `skipped_unimplemented`.
   * Else if tier == preview and `--preview` not set → mark `skipped_preview`.
   * Else → schedule for execution.
5. Run scheduled cops across files (parallel).
6. Apply `--fail-level` to compute lint failures.
7. Print results + (unless `--quiet-skips`) one grouped skip summary.
8. Determine exit code (see section 5).

**Grouped skip notice (exact contract)**
Printed once per run if any skipped:

> Skipped N cops (A preview-gated, B unimplemented, C outside baseline). Run `turbocop migrate` for details.

### `turbocop migrate [PATH]`

**Purpose**: first-run evaluator. No linting required; purely config analysis.

**Output (text)**
A deterministic table, grouped counts + top examples:

* Baseline: rubocop X.Y, rubocop-rails A.B, …
* Enabled by config:

  * Stable: ### (runs by default)
  * Preview: ### (requires `--preview`)
  * Unimplemented: ###
  * Outside baseline: ###

Then list up to K cops per category with short reason.

**Output (json)**
Add `--format json` with schema:

```json
{
  "baseline": {"rubocop":"1.xx.x","rubocop-rails":"2.yy.y", "...": "..."},
  "counts": {"stable": 712, "preview": 103, "unimplemented": 21, "outside_baseline": 9},
  "cops": [
    {"name":"Style/Foo", "status":"stable"},
    {"name":"Rails/Bar", "status":"preview"},
    {"name":"Lint/Baz", "status":"unimplemented"},
    {"name":"RSpec/Qux", "status":"outside_baseline"}
  ]
}
```

### `turbocop doctor`

**Purpose**: support/debug output.

Must include:

* Baseline versions (vendored RuboCop + plugin versions turbocop targets)
* Config root + config files loaded (full inheritance chain)
* Gem version mismatch warnings: compare Gemfile.lock plugin versions against baseline and warn if they differ (this is already implemented in the bench harness — promote to user-facing output)
* Summary of skipped cops (same 4 categories as `check`)
* Autocorrect mode (if relevant)

### `turbocop rules`

**Purpose**: list all cops turbocop knows about.

**Flags**

* `--tier <stable|preview>`: filter by tier.
* `--format <table|json>`: output format (default: table).

**Output columns**: name, tier, implemented?, baseline presence, short description, default enabled?, known divergence count (if corpus data available).

### `turbocop verify [PATH]` (Ruby required)

**Purpose**: “oracle mode” for skeptical teams. Not part of core single-binary story.

**Flags**

* `--rubocop-cmd <string>` optional override (default `bundle exec rubocop`)
* `--format <text|json>` output diff
* `--by-cop` summary

**Behavior**

1. Run turbocop with `--format json` for PATH.
2. Run RuboCop producing JSON (`rubocop --format json`) on same PATH.
3. Normalize both outputs and diff:

   * per-cop FP/FN/matches
   * optionally per-file details
4. Exit code:

   * `0` if no diffs and rubocop ran successfully
   * `1` if diffs exist
   * `3` if verify tool error (rubocop missing, rubocop crashed, parse error)

**Important**: verify is not required for normal use; it is a migration/confidence tool.

---

## 3) Config resolution (exact)

### Root selection

* Starting at PATH (or CWD), walk up until `.rubocop.yml` found.
* That directory is the “config root”.

### Supported config features (phase 1)

Implementer must mark explicitly what’s supported:

* `AllCops: Exclude/Include` patterns ✅ implemented
* `inherit_from` (local file paths) ✅ implemented
* `inherit_gem` ✅ implemented (resolves gem paths via `bundle info --path`)
* `inherit_mode` (merge/override behavior) ✅ implemented
* per-cop `Enabled`, and per-cop key-value settings ✅ implemented

### Unknown config keys

* Do not fail by default; warn once in `doctor`/`migrate` (grouped).
* Add `--strict-config` later if needed; don’t block phase 1.

---

## 4) Tiering system (stable/preview only)

### Data model

Check in a file: `resources/tiers.json` (embedded at compile time)

```json
{
  "schema": 1,
  "default_tier": "stable",
  "overrides": {
    "Lint/Syntax": "preview",
    "Rails/SomeFragileCop": "preview"
  }
}
```

Rules:

* If cop missing from overrides → `stable`.
* “Mostly stable” initial experience is default.

### Initial tier assignment policy

Before corpus oracle exists:

* Default stable for all implemented cops.
* Maintain a small curated preview override list:

  * cops with known divergence reports
  * cops recently changed/bugfixed in turbocop
  * cops known to depend on Prism/Parser recovery differences

(Implementer can generate this list semi-automatically from git history + a manual allowlist.)

### Demotion workflow

* Any confirmed FP/FN on a stable cop → add to overrides as preview in the next patch release.
* Promotion is data-driven (via corpus stats).

## Tier promotion criteria (Preview → Stable)

A cop may be promoted to **Stable** only when all applicable gates pass:

### Gate A: End-to-end parity (required)

Run turbocop vs the **pinned RuboCop baseline** on the corpus (baseline mode and, if available, repo-config mode). For this cop:

* **True diffs = 0** across the corpus

  * FP = 0, FN = 0
  * Excluding “noise buckets” (see Gate D)
* **Crashes/timeouts = 0** attributable to this cop (or any run that enables it)

If the corpus is still small, require the above across:

* all bench repos + at least N additional repos (choose N, e.g. 50–100), and
* at least M total opportunities (e.g. ≥ 1,000 occurrences of candidate nodes or ≥ 100 offenses in RuboCop), to avoid “stable by lack of coverage.”

### Gate B: NodePattern verifier (required when applicable)

If the cop uses `def_node_matcher` / NodePattern-derived matching:

* Compiled matcher == NodePattern interpreter on harvested AST nodes
* **0 verifier mismatches** in CI across the node corpus

If the cop has no NodePattern patterns, Gate B is “not applicable.”

### Gate C: Autocorrect safety (required if cop supports autocorrect)

If the cop can autocorrect:

* Autocorrect is either **disabled in Stable by default** *or* it passes an autocorrect gate:

  * On a fixture set (from corpus diffs + hand tests), turbocop’s corrected output matches RuboCop baseline (or matches a defined normalization) with **0 diffs**.
* No “unsafe edit” class bugs open for this cop (crashes, corrupt output, wrong offsets).

### Gate D: Noise bucket exclusions (defined up front)

These do **not** count as “true diffs” for Gate A (but must be tracked separately):

* Parser recovery / syntax differences (`Lint/Syntax`, parse failures due to Prism vs Parser)
* “Outside baseline” cops (cop doesn’t exist in baseline snapshot)
* “Unimplemented” cops (exists in baseline but not implemented)
* Config features explicitly marked “unsupported” (if any)

Important: if a cop’s behavior diff is *caused by your config loader diverging* (not an explicitly unsupported feature), it **does** count as a true diff.

### Demotion rule (Stable → Preview)

A Stable cop is demoted to **Preview** immediately if any of the following occur:

* Any confirmed FP/FN vs baseline (not in a noise bucket)
* Any crash/timeout attributable to the cop
* Any NodePattern verifier mismatch (if applicable)
* Any autocorrect regression (if autocorrect is enabled for Stable)

### Practical thresholds (if you want numbers)

If “0 diffs” is too strict early on, use a temporary policy:

* Stable requires **0 diffs on bench + 0 diffs on ≥ 100 repos**, and
* Preview may have diffs but must be below a small rate (e.g. < 1 per 50k LOC) to be considered “near-stable.”
  Then tighten over time toward 0-diff Stable.

---

## 5) Exit codes + `--fail-level` (define now, don’t change later)

### Exit codes (final)

* `0`: success — no offenses at/above fail-level, and (if `--strict`) no coverage failures
* `1`: lint failure — offenses exist at/above fail-level
* `2`: strict coverage failure — skipped cops exist that violate the strict scope (only when no lint failures; if both lint and strict fail, exit `1` and print both summaries)
* `3`: internal error — panic, IO error, config parse failure, etc.

**When both lint and strict fail:** exit `1` (lint takes priority), print both the lint results and a strict coverage warning. Rationale: lint failures are more immediately actionable.

### Strict mode semantics

`--strict` accepts a scope (default: `coverage`):

* **`--strict=coverage`** (default when bare `--strict` is used):
  Fail (exit 2) for cops turbocop implements (Stable or Preview) that were
  skipped (e.g., preview-gated cops without `--preview`). Unimplemented and
  outside-baseline cops are informational — they don't trigger failure.

* **`--strict=implemented-only`**:
  Ignore unknown/outside-baseline cops entirely. Only fail if a cop turbocop
  implements (Stable or Preview) was skipped. Useful for teams that know they
  use unsupported plugins and don't want noise.

* **`--strict=all`**:
  Any skipped cop for any reason (preview-gated, unimplemented, outside
  baseline) triggers coverage failure. Most conservative; only useful when
  the project's config is fully within turbocop's baseline.

### `--fail-level`

* Offenses have a severity. Map rubocop-ish severities into your internal enum.
* Offenses below fail-level do not affect exit code.

---

## 6) Output formats + normalization (enables corpus + verify)

### Internal diagnostic struct (single source of truth)

```rust
struct Diagnostic {
  file: PathBuf,
  line: u32,
  column: u32,
  cop: String,
  message: String,
  severity: Severity,
  corrected: bool, // optional
  // maybe: end_line/end_col, replacement, etc.
}
```

### JSON format for `check`

* Stable schema versioned:

```json
{"schema":1,"diagnostics":[ ... ],"skipped":{...},"baseline":{...}}
```

This same schema is what `verify` and corpus tooling diff.

---

## 7) NodePattern verifier (prioritize; scoped)

### Goal

Catch matcher-layer drift: “compiled matcher == NodePattern interpreter” on real AST nodes.

### Inputs

* Extract NodePattern strings from vendored RuboCop source (or your existing extraction step).
* AST nodes from:

  * existing bench repos (phase 1)
  * later, corpus repos (phase 2+)

### Verifier API

For each cop matcher:

* `compiled_matches(node) -> bool`
* `interpreted_matches(node, pattern) -> bool`

Assert equal over a node corpus. On mismatch, dump:

* cop name
* pattern string
* node kind + a stable node serialization (your own S-expr/JSON)
* file path + location
* a minimal reproduction artifact file written to `target/verifier_failures/...`

### Where it runs

* `cargo test verifier` in CI (test module within the main crate, not a separate workspace crate).
* Gate merges that modify matching logic/mapping tables.

**Note**: this does not replace end-to-end correctness measurement; it prevents a big bug class cheaply.

**Existing work**: `src/bin/node_pattern_codegen.rs` contains a complete NodePattern lexer/parser (~1,880 lines) that can be adapted into the interpreter. The lexer/parser is fully functional; only the code generation backend needs to be replaced with an interpreter evaluation loop.

---

## 8) Corpus oracle tooling (phase 2, but define interfaces now)

**Existing infrastructure**: `bench/bench.rs` (`bench_turbocop` binary) already implements `setup`, `bench`, `conform`, `report`, `autocorrect-conform`, and `autocorrect-validate` subcommands. The `conform` subcommand runs both tools and produces `bench/conform.json` with per-cop FP/FN/match data. Extend this, don't rewrite.

### New subcommands to add

* `bench_turbocop corpus fetch --list repos.txt` — clone/update repos from manifest
* `bench_turbocop gen-tiers --diff bench/conform.json --out resources/tiers.json` — generate tier assignments from conformance data

The existing `conform` subcommand already handles running both tools + diffing + noise detection (including gem version mismatch attribution). It needs:

* phased corpus manifest support (core frozen set + rotating set)
* noise bucketing categories aligned with the skip classification

### Corpus scale (phased, matching high-level plan)

* **Phase 2**: ~100 repos (current 14 public + 14 private → expand to 100)
* **Phase 3**: ~300 repos (only if Phase 2 still producing novel diffs)
* **Phase 4**: 500-1000 repos (optional, marketing value)

Core frozen set (~50 repos) pinned to exact commit hashes; rotating set (~50) refreshed quarterly.

### RuboCop invocation

* Pin RuboCop versions to turbocop baseline (preferred) OR run `bundle exec rubocop` and accept noise (not preferred).
* If pinning: maintain a Bundler Gemfile in the bench harness and install to a cache directory.
* The existing bench harness already handles both modes and detects version mismatches.

### Diffing rules

Normalize diagnostics, then compare by key:

* `file + line + col + cop` (and optionally message normalization)
  Compute FP/FN/matches.

### Noise buckets

At minimum:

* parse/syntax bucket (Lint/Syntax, Prism vs Parser recovery differences)
* gem version mismatch bucket (already detected by bench harness)
* outside baseline / unimplemented bucket
* true diffs
* crashes/timeouts

---

## 9) Phase plan (deliverables & acceptance criteria)

### Phase 1 (adoption + safety)

Deliverables:

* Skip classification (4 categories) + grouped notice in `check` output
* `tiers.json` support (default stable + curated preview overrides)
* `migrate` command (config analysis, no linting)
* `doctor` command (debug/support output)
* Exit code contract (0/1/2/3) + `--strict` with scope categories
* NodePattern verifier in CI (bench-repo node corpus)
* (Optional but recommended) `verify` command

Acceptance:

* Running `turbocop migrate` on a repo answers “what will run?” clearly.
* `check` produces deterministic skip summaries.
* `--strict=coverage` correctly distinguishes implemented-but-skipped from unimplemented.
* Verifier catches intentional mismatch in a test case.

### Phase 2 (measurement, ~100 repos)

Deliverables:

* Corpus manifest + fetch tooling for ~100 repos
* Extend `bench_turbocop conform` with noise bucketing
* `gen-tiers` subcommand to produce `tiers.json` from conformance data
* Generated compatibility table (`docs/compatibility.md`)
* Start promoting/demoting cops based on data

Acceptance:

* Can produce per-cop FP/FN table across 100 repos.
* Can regenerate `tiers.json` deterministically from corpus data.
* Gem version mismatch diffs are bucketed separately from true diffs.

### Phase 3 (flywheel + polish, ~300 repos)

Deliverables:

* Regression fixture extraction (save repro for each true diff)
* Expand corpus to ~300 repos (only if still producing novel diffs)
* Better noise bucketing + diff categorization
* (Optional later) fixture minimizer

Acceptance:

* Any newly discovered diff becomes a checked-in fixture and stays fixed.
* Corpus expansion produces diminishing returns (validates that 100 was sufficient, or catches the tail).

### Phase 4 (scale, optional)

Deliverables:

* Corpus to 500-1000 repos (tarball-based, automated maintenance)
* Core frozen set (~50 repos) + rotating set for exploration
* Fully automated pipeline (“add rows to manifest file”)

Acceptance:

* Pipeline runs unattended on new repos without manual intervention.
* Core frozen set metrics never regress across releases.

---

## 10) What implementers should *not* build yet (to prevent scope creep)

* `.turbocop.yml` (until real demand)
* Per-repo version emulation (explicitly out of scope — behavior is baseline-defined)
* Fancy fixture minimizer (store full repros first)
* Subcommand-level binaries (keep everything in `bench_turbocop` for now, not separate crates)
