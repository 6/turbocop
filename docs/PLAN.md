# turbocop Plan (Authoritative)

> For plan history and earlier iterations, see `docs/archive/PLAN_HIGH_LEVEL_v1.md`
> and `docs/archive/PLAN_DETAILED_v1.md`. For pre-Phase-1 progress tracking, see
> `docs/archive/PROGRESS_v1.md`.

---

## 0) Product contract (must be in README + `--help`)

### What turbocop is

* A **fast Ruby linter** with a **RuboCop-inspired ruleset** and **RuboCop-style configuration syntax** (reads `.rubocop.yml`).
* turbocop behavior is defined by the **turbocop baseline** (the vendored RuboCop + plugins snapshot), not the project's Gemfile.lock.

### What turbocop is not

* Not "perfect drop-in parity for arbitrary RuboCop/plugin versions."
* Per-repo version detection (Gemfile.lock) may be used for **warnings**, not emulation.

### Hard guarantees

* turbocop reads `.rubocop.yml` (RuboCop-style config) and supports a documented subset of RuboCop config semantics.
* turbocop's behavior is defined against turbocop's **baseline versions**, not the repo's Gemfile.lock.
* turbocop supports two tiers: `stable` (default) and `preview` (opt-in).

### Explicit non-guarantees

* Not guaranteed to match RuboCop for arbitrary plugin versions or edge cases.
* `verify` requires Ruby (and/or Bundler); turbocop remains single-binary for `check`.

### Bridge for skeptics

* `turbocop verify` runs RuboCop as an **oracle** and shows diffs (requires Ruby). This is the "don't trust us? prove it" path.
* `turbocop migrate` gives a clear answer to "what will run?" before you commit to anything.

---

## Glossary

* **Baseline**: the vendored RuboCop + plugin snapshot turbocop targets (versions are part of turbocop's identity).
* **Cop**: a rule.
* **Tier**: `stable` or `preview`.
* **Skipped cop**: referenced/enabled by config but not run (preview-gated, unimplemented, outside baseline).

---

## 1) Configuration & first-run experience

### Config file support (Phase 1)

* Read **`.rubocop.yml` directly** (no conversion).
* **No `.turbocop.yml`** initially. Prefer CLI flags/env vars until there's clear demand.

### Cop resolution categories (must be user-visible)

When `.rubocop.yml` enables a cop, turbocop classifies it:

1. **Implemented & Stable** → runs
2. **Implemented but Preview-gated** → skipped unless `--preview`
3. **Known in baseline but not implemented** → skipped + warned
4. **Outside baseline (unknown to this turbocop release)** → skipped + warned

**Default behavior:** run what you can, skip what you can't, **tell the user once**.

### One grouped notice (no spam)

At end of run, print a single grouped line if anything was skipped:

> Skipped 41 cops (22 preview-gated, 11 unimplemented, 8 outside baseline). Run `turbocop migrate` for details.

Add `--quiet-skips` to suppress.

---

## 2) CLI surface (exact behavior)

### `turbocop check [PATH]`

**Purpose**: run linting using `.rubocop.yml`.

**Flags**

* `--preview` (bool): allow running preview-tier cops.
* `--strict[=SCOPE]`: skipped cops cause a coverage failure exit code.
  Scope: `coverage` (default), `implemented-only`, `all`. See section 5.
* `--fail-level <level>`: set failure threshold. Levels: `refactor|convention|warning|error|fatal`.
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

* Baseline: rubocop X.Y, rubocop-rails A.B, ...
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
* Gem version mismatch warnings: compare Gemfile.lock plugin versions against baseline and warn if they differ
* Summary of skipped cops (same 4 categories as `check`)
* Autocorrect mode (if relevant)

### `turbocop rules`

**Purpose**: list all cops turbocop knows about.

**Flags**

* `--tier <stable|preview>`: filter by tier.
* `--format <table|json>`: output format (default: table).

**Output columns**: name, tier, implemented?, baseline presence, short description, default enabled?, known divergence count (if corpus data available).

### `turbocop verify [PATH]` (Ruby required)

**Purpose**: "oracle mode" for skeptical teams. Not part of core single-binary story.

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
* That directory is the "config root".

### Supported config features (phase 1)

* `AllCops: Exclude/Include` patterns
* `inherit_from` (local file paths)
* `inherit_gem` (resolves gem paths via `bundle info --path`)
* `inherit_mode` (merge/override behavior)
* per-cop `Enabled`, and per-cop key-value settings

### Unknown config keys

* Do not fail by default; warn once in `doctor`/`migrate` (grouped).
* Add `--strict-config` later if needed; don't block phase 1.

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
* "Mostly stable" initial experience is default.

### Autocorrect allowlist (separate from lint tiers)

Check in: `resources/autocorrect_safe_allowlist.json`

```json
{
  "schema": 1,
  "cops": ["Layout/TrailingWhitespace", "Style/StringLiterals", "..."]
}
```

A cop must be in this allowlist for `-a` to apply its corrections. `-A` applies all cops that implement autocorrect regardless of allowlist (explicit unsafe opt-in).

`migrate` reports autocorrect eligibility alongside lint status:
* Stable + autocorrect-safe: will lint and autocorrect with `-a`
* Stable + autocorrect-unsafe-only: will lint; autocorrect requires `-A`
* Stable + no autocorrect: lint only

### Initial tier assignment policy

Before corpus oracle exists:

* Default stable for all implemented cops.
* Maintain a small curated preview override list:

  * cops with known divergence reports
  * cops recently changed/bugfixed in turbocop
  * cops with known Prism-vs-Parser sensitivity
  * cops with risky/autocorrect complexity

### Demotion workflow

* Any confirmed Stable FP/FN → add to overrides as preview in the next patch release.
* Promotion is data-driven (via corpus stats).

### Tier promotion criteria (Preview → Stable)

A cop may be promoted to **Stable** only when all applicable gates pass:

#### Gate A: End-to-end parity (required)

Run turbocop vs the **pinned RuboCop baseline** on the corpus. For this cop:

* **True diffs = 0** across the corpus (FP = 0, FN = 0, excluding noise buckets)
* **Crashes/timeouts = 0** attributable to this cop

If the corpus is still small, require the above across all bench repos + at least N additional repos (e.g. 50-100), and at least M total opportunities (e.g. >= 1,000 occurrences).

#### Gate B: NodePattern verifier (required when applicable)

If the cop uses `def_node_matcher` / NodePattern-derived matching:

* Compiled matcher == NodePattern interpreter on harvested AST nodes
* **0 verifier mismatches** in CI across the node corpus

#### Gate C: Autocorrect safety (required if cop supports autocorrect)

Autocorrect is a **separate maturity track** from lint parity. A cop can be Stable for linting but still have autocorrect disabled.

If the cop can autocorrect, it may enter `autocorrect_safe_allowlist.json` only when:

* **Parse gate**: every file changed by autocorrect parses successfully with Prism.
* **Idempotence gate**: running autocorrect twice yields no further edits.
* **Non-overlap gate**: edits don't overlap and have valid byte ranges.
* **Oracle parity gate**: on the corpus, turbocop's corrected output matches RuboCop baseline output with **0 mismatches**.

Any autocorrect bug report immediately removes the cop from the allowlist until fixed.

#### Gate D: Noise bucket exclusions (defined up front)

These do **not** count as "true diffs" for Gate A (but must be tracked separately):

* Parser recovery / syntax differences (`Lint/Syntax`, parse failures due to Prism vs Parser)
* "Outside baseline" cops (cop doesn't exist in baseline snapshot)
* "Unimplemented" cops (exists in baseline but not implemented)
* Config features explicitly marked "unsupported" (if any)

Important: if a cop's behavior diff is *caused by your config loader diverging* (not an explicitly unsupported feature), it **does** count as a true diff.

#### Practical thresholds (if you want numbers)

If "0 diffs" is too strict early on, use a temporary policy:

* Stable requires **0 diffs on bench + 0 diffs on >= 100 repos**, and
* Preview may have diffs but must be below a small rate (e.g. < 1 per 50k LOC) to be considered "near-stable."
  Then tighten over time toward 0-diff Stable.

#### Demotion rule (Stable → Preview)

A Stable cop is demoted to **Preview** immediately if any of the following occur:

* Any confirmed FP/FN vs baseline (not in a noise bucket)
* Any crash/timeout attributable to the cop
* Any NodePattern verifier mismatch (if applicable)
* Any autocorrect regression (if autocorrect is enabled for Stable)

---

## 5) Exit codes + `--fail-level` (define now, don't change later)

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
  implements (Stable or Preview) was skipped.

* **`--strict=all`**:
  Any skipped cop for any reason (preview-gated, unimplemented, outside
  baseline) triggers coverage failure. Most conservative.

### `--fail-level`

* Offenses have a severity. Offenses below fail-level do not affect exit code.

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
  corrected: bool,
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

Catch matcher-layer drift: "compiled matcher == NodePattern interpreter" on real AST nodes.

### What it does and doesn't prove

* **Does prove:** matcher equivalence to NodePattern (given your interpreter)
* **Does not prove:** cop logic, config semantics, autocorrect behavior

### Inputs

* Extract NodePattern strings from vendored RuboCop source.
* AST nodes from:

  * existing bench repos (phase 1)
  * later, corpus repos (phase 2+)

### Verifier API

For each cop matcher:

* `compiled_matches(node) -> bool`
* `interpreted_matches(node, pattern) -> bool`

Assert equal over a node corpus. On mismatch, dump cop name, pattern string, node kind + stable serialization, file path + location, and a minimal reproduction artifact.

### Where it runs

* `cargo test verifier` in CI.
* Gate merges that modify matching logic/mapping tables.

**Existing work**: `src/bin/node_pattern_codegen.rs` contains a complete NodePattern lexer/parser (~1,880 lines) that can be adapted into the interpreter.

---

## 8) Corpus oracle tooling (phase 2, but define interfaces now)

**Execution model**: runs in **GitHub Actions CI**, not locally. Public repos get unlimited free minutes on standard runners. Matrix jobs fan out per repo batch. Results are uploaded as workflow artifacts (`results.md` + `corpus_results.json`). See `docs/CORPUS_PLAN.md` for full CI workflow design.

**Existing infrastructure**: `bench/bench.rs` (`bench_turbocop` binary) already implements `setup`, `bench`, `conform`, `report`, `autocorrect-conform`, and `autocorrect-validate` subcommands. Extend this for use in CI, don't rewrite.

### New subcommands to add

* `bench_turbocop corpus fetch --list repos.txt` — clone/update repos from manifest
* `bench_turbocop gen-tiers --diff bench/conform.json --out resources/tiers.json` — generate tier assignments from conformance data

### Corpus scale (phased)

* **Phase 2**: ~100 repos (current 14 public + 14 private → expand to 100)
* **Phase 3**: ~300 repos (only if Phase 2 still producing novel diffs)
* **Phase 4**: 500-1000 repos (optional, marketing value)

Core frozen set (~50 repos) pinned to exact commit hashes; rotating set (~50) refreshed quarterly.

### RuboCop invocation

* Pin RuboCop versions to turbocop baseline (preferred) OR run `bundle exec rubocop` and accept noise.
* The existing bench harness already handles both modes and detects version mismatches.

### Two passes (separates bug categories)

1. **Baseline mode:** run both tools with a controlled config (or none)
2. **Repo-config mode:** run with each repo's `.rubocop.yml`

### Diffing rules

Normalize diagnostics, then compare by key: `file + line + col + cop` (and optionally message normalization). Compute FP/FN/matches.

### Tier updates are data-driven

* Promote Preview → Stable when corpus shows near-zero diffs and no crashes.
* Demote Stable → Preview when diffs appear or a regression is reported.

Output artifacts:

* `tiers.json` (checked in)
* `compatibility.md` (generated table)
* "Top diverging cops" list (roadmap)

### Noise buckets

* parse/syntax bucket (Lint/Syntax, Prism vs Parser recovery differences)
* gem version mismatch bucket (already detected by bench harness)
* outside baseline / unimplemented bucket
* true diffs
* crashes/timeouts

---

## 8b) Autocorrect oracle harness (separate lane from lint parity)

Autocorrect is higher risk than linting: a wrong rewrite can silently break code. Treat it as an independent oracle lane with stricter gates and conservative defaults.

**Existing infrastructure**: `bench_turbocop autocorrect-conform` already copies each bench repo, runs `rubocop -A` on one copy and `turbocop -A` on the other, and diffs all `.rb` files.

### Harness workflow per repo

1. **Create isolated working copy** — temp copy of checkout.
2. **Capture pre-state** — enumerate Ruby files, record per-file SHA-256 hash.
3. **Run oracle autocorrect** (RuboCop baseline bundle) — restrict to allowlisted cops only.
4. **Capture oracle post-state** — per-file hash + unified diff of changed files.
5. **Reset to pre-state**, run turbocop autocorrect with same file set + cop allowlist.
6. **Capture turbocop post-state** similarly.
7. **Compare** — primary: file content equality (hash match).
8. **On mismatch** — bucket as `autocorrect_mismatch`, store repro artifact.

### Safety gates (must-pass before oracle equality)

For turbocop's autocorrect output:

* **Parse gate**: every changed file must parse successfully with Prism.
* **Idempotence gate**: running turbocop autocorrect twice yields no further edits.
* **Non-overlap gate**: edits must not overlap and must have valid byte ranges.
* **No-op gate**: if turbocop reports edits, at least one file hash must change.

If any gate fails, bucket as `autocorrect_invalid_output` (higher severity than mismatch).

### Noise buckets (autocorrect-specific)

* `autocorrect_mismatch` — RuboCop and turbocop outputs differ
* `autocorrect_invalid_output` — parse/idempotence/non-overlap gate failed
* `autocorrect_oracle_failed` — RuboCop crashed during autocorrect
* `autocorrect_tool_failed` — turbocop crashed during autocorrect

### Result storage

```
results/
  autocorrect/
    rubocop/baseline_safe/<repo_id>.json
    turbocop/baseline_safe/<repo_id>.json
  autocorrect_artifacts/
    <repo_id>/<cop_name>/
      pre.rb / rubocop_post.rb / turbocop_post.rb
      diff_rubocop.patch / diff_turbocop.patch
      meta.json
```

### Allowlist promotion criteria

A cop enters `autocorrect_safe_allowlist.json` only when:

* Zero `autocorrect_invalid_output` across the corpus
* Zero `autocorrect_mismatch` across the corpus
* Idempotence gate passes on all touched files
* Any autocorrect bug report immediately removes the cop until fixed

### Phasing

* **Phase 2**: safe-mode autocorrect oracle on corpus (baseline mode only)
* **Phase 3**: expand to repo-config mode autocorrect
* **Phase 4+**: unsafe autocorrect oracle (optional)

---

## 8c) Regression flywheel (turn diffs into tests)

For every true diff found by corpus or verify:

1. Save repro fixture (file + minimal config context)
2. Add to test suite
3. (Later) implement minimization

MVP: store the whole file first; minimizing comes after.

---

## 8d) Messaging & docs (ship with Phase 1)

### README top section must include

* Baseline-defined behavior ("RuboCop-inspired" not perfect drop-in)
* Stable vs Preview contract + how to enable preview
* `turbocop migrate` and `turbocop verify` as the first-run answers

### Compatibility table

Even before corpus is perfect:

* Publish tier list + baseline versions.
* Update it as corpus runs.

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
* Autocorrect off by default; `-a`/`-A` flags working for existing cops
* (Optional but recommended) `verify` command

Acceptance:

* Running `turbocop migrate` on a repo answers "what will run?" clearly.
* `check` produces deterministic skip summaries.
* `--strict=coverage` correctly distinguishes implemented-but-skipped from unimplemented.
* Verifier catches intentional mismatch in a test case.
* `-a` applies only allowlisted cops; `-A` applies all that implement autocorrect.

### Phase 2 (measurement, ~100 repos)

Deliverables:

* Corpus manifest + fetch tooling for ~100 repos
* Extend `bench_turbocop conform` with noise bucketing
* `gen-tiers` subcommand to produce `tiers.json` from conformance data
* Generated compatibility table (`docs/compatibility.md`)
* Start promoting/demoting cops based on data
* **Autocorrect oracle lane**: safe-mode autocorrect comparison on corpus
* Initial `autocorrect_safe_allowlist.json` generated from corpus results
* `migrate` reports autocorrect eligibility per cop

Acceptance:

* Can produce per-cop FP/FN table across 100 repos.
* Can regenerate `tiers.json` deterministically from corpus data.
* Gem version mismatch diffs are bucketed separately from true diffs.
* Autocorrect oracle produces per-cop pass/fail with safety gate results.

### Phase 3 (flywheel + polish, ~300 repos)

Deliverables:

* Regression fixture extraction (save repro for each true diff)
* Expand corpus to ~300 repos (only if still producing novel diffs)
* Better noise bucketing + diff categorization
* Expand autocorrect allowlist as cops pass oracle gates
* (Optional) repo-config mode autocorrect oracle
* (Optional later) fixture minimizer

Acceptance:

* Any newly discovered diff becomes a checked-in fixture and stays fixed.
* Corpus expansion produces diminishing returns (validates that 100 was sufficient, or catches the tail).
* Autocorrect allowlist grows as cops are verified.

### Phase 4 (scale, optional)

Deliverables:

* Corpus to 500-1000 repos (tarball-based, automated maintenance)
* Core frozen set (~50 repos) + rotating set for exploration
* Fully automated pipeline ("add rows to manifest file")
* (Optional) unsafe autocorrect oracle lane

Acceptance:

* Pipeline runs unattended on new repos without manual intervention.
* Core frozen set metrics never regress across releases.

---

## 10) First-time user flow

1. User runs:

```bash
turbocop check .
```

* turbocop reads `.rubocop.yml`
* runs Stable cops
* skips Preview cops unless `--preview`
* prints one grouped skip summary

2. User runs:

```bash
turbocop migrate .
```

* sees a table: "these will run, these need --preview, these aren't implemented, these are outside baseline"
* gets a suggested CI command line

3. Skeptical team runs:

```bash
turbocop verify .
```

* gets a diff against RuboCop oracle (Ruby required), per cop
