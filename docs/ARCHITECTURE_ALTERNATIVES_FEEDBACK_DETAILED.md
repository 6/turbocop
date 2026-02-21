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
* `--strict` (bool): skipped cops cause a coverage failure exit code.
* `--fail-level <level>`: set failure threshold. Levels (example): `refactor|convention|warning|error|fatal`.
* `--format <text|json>`: output format.
* `--quiet-skips` (bool): suppress grouped skip notice.
* `--autocorrect <off|safe|all|unsafe>` (optional; if you support autocorrect now)

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

* Baseline versions
* Config root + config files loaded
* Ruby version detection (optional) + Gemfile.lock detected versions (warnings only)
* Summary of skipped cops (same categories as above)
* Autocorrect mode (if relevant)

### `turbocop rules`

Lists all cops turbocop knows about:

* name, tier, implemented?, baseline presence, short description, default enabled? (optional)

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

* `AllCops: Exclude/Include` patterns
* `inherit_from` (local file paths)
* `inherit_gem` (optional in phase 1; if unsupported, classify as “config unsupported” and show in migrate/doctor)
* per-cop `Enabled`, and per-cop key-value settings you already support

### Unknown config keys

* Do not fail by default; warn once in `doctor`/`migrate` (grouped).
* Add `--strict-config` later if needed; don’t block phase 1.

---

## 4) Tiering system (stable/preview only)

### Data model

Check in a file: `crates/turbocop_core/resources/tiers.json`

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

### Exit codes (recommended)

* `0`: success (no offenses at/above fail-level) AND (if `--strict`) no skipped cops
* `1`: lint failure (>= fail-level offenses exist)
* `2`: strict coverage failure (skipped cops exist) **and** no lint failure at/above fail-level (or even if there is—pick one rule; recommended: coverage failure can still be `2` even if lint fails? Better: prefer `1` for lint, `2` for coverage, or provide combined bitmask. Simpler: `1` for lint, `2` for strict skip; if both, return `1` and print both summaries.)
* `3`: internal error (panic, IO error, config parse failure, etc.)

### Strict mode semantics

If `--strict`:

* Any skipped cop of any category triggers “coverage failure” behavior.
* Still print lint output; exit code per rule above.

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

* `cargo test -p turbocop_verifier` in CI
* Gate merges that modify matching logic/mapping tables.

**Note**: this does not replace end-to-end correctness measurement; it prevents a big bug class cheaply.

---

## 8) Corpus oracle tooling (phase 2, but define interfaces now)

Create a separate binary crate: `turbocop-bench`

### Commands

* `turbocop-bench corpus fetch --list repos.txt --dest corpus/`
* `turbocop-bench run rubocop --corpus corpus/ --out results/rubocop/`
* `turbocop-bench run turbocop --corpus corpus/ --out results/turbocop/`
* `turbocop-bench diff --rubocop results/rubocop --turbocop results/turbocop --by-cop --out diff.json`
* `turbocop-bench gen-tiers --diff diff.json --out tiers.json`

### RuboCop invocation

* Pin RuboCop versions to turbocop baseline (preferred) OR run `bundle exec rubocop` and accept noise (not preferred).
* If pinning: maintain a Bundler Gemfile in the bench harness and install to a cache directory.

### Diffing rules

Normalize diagnostics, then compare by key:

* `file + line + col + cop` (and optionally message normalization)
  Compute FP/FN/matches.

### Noise buckets

At minimum:

* parse/syntax bucket
* outside baseline / unimplemented bucket
* true diffs
* crashes/timeouts

---

## 9) Phase plan (deliverables & acceptance criteria)

### Phase 1 (ship tiers + migration UX + verifier)

Deliverables:

* `check`, `migrate`, `doctor`, JSON output, skip categories
* `tiers.json` support (default stable + overrides)
* exit code contract implemented
* NodePattern verifier in CI (bench-repo node corpus)

Acceptance:

* Running `turbocop migrate` on a repo answers “what will run?” in <5s.
* `check` produces deterministic skip summaries.
* `--strict` behaves per spec.
* Verifier catches intentional mismatch in a test case.

### Phase 2 (oracle-at-scale MVP)

Deliverables:

* `turbocop-bench` corpus runner + diff per cop
* generate tiers from diff
* start promoting/demoting by data

Acceptance:

* Can produce per-cop FP/FN table across N repos.
* Can regenerate tiers.json deterministically.

### Phase 3 (regression flywheel)

Deliverables:

* store repro fixtures for each diff
* (optional later) minimizer

Acceptance:

* Any newly discovered diff becomes a checked-in fixture and stays fixed.

---

## 10) What implementers should *not* build yet (to prevent scope creep)

* `.turbocop.yml` (until real demand)
* per-repo version emulation (explicitly out of scope)
* fancy minimizer (store full repros first)
* deep `inherit_gem` unless you already have it (otherwise document as unsupported and surface in migrate/doctor)
