# turbocop Plan vNext (Stable + Preview, verifier-first, adoption-first)

## 0) Product contract (explicit, prominent)

### What turbocop is

* A **fast Ruby linter** with a **RuboCop-inspired ruleset** and **RuboCop-style configuration syntax** (reads `.rubocop.yml`).
* turbocop behavior is defined by the **turbocop baseline** (the vendored RuboCop + plugins snapshot), not the project’s Gemfile.lock.

### What turbocop is not

* Not “perfect drop-in parity for arbitrary RuboCop/plugin versions.”
* Per-repo version detection (Gemfile.lock) may be used for **warnings**, not emulation.

### Bridge for skeptics (promote this)

* `turbocop verify` runs RuboCop as an **oracle** and shows diffs (requires Ruby). This is the “don’t trust us? prove it” path.

---

## 1) Configuration & first-run experience

### Config file support (Phase 1)

* Read **`.rubocop.yml` directly** (no conversion).
* **No `.turbocop.yml`** initially. Prefer CLI flags/env vars until there’s clear demand.

### Cop resolution categories (must be user-visible)

When `.rubocop.yml` enables a cop, turbocop classifies it:

1. **Implemented & Stable** → runs
2. **Implemented but Preview-gated** → skipped unless `--preview`
3. **Known in baseline but not implemented** → skipped + warned
4. **Outside baseline (unknown to this turbocop release)** → skipped + warned

**Default behavior:** run what you can, skip what you can’t, **tell the user once**.

### One grouped notice (no spam)

At end of run (or top), print a single grouped line if anything was skipped:

> Skipped 41 cops (22 preview-gated, 11 unimplemented, 8 outside baseline). Run `turbocop migrate` for details.

Add `--quiet-skips` to suppress.

---

## 2) Tiering model (Stable + Preview only)

### Goals

* Default experience should be strong: turbocop should lint “most things” out of the box.
* Preview is the safety valve for rules with known diffs or high churn.

### Initial tier assignment (not overly conservative)

Start with **most cops in Stable**, demote to Preview based on:

* cops with known open divergence issues
* cops touched by bug fixes recently (your repo history signal)
* cops with known Prism-vs-Parser sensitivity
* cops with risky/autocorrect complexity

Then use corpus oracle to correct mistakes quickly.

### Demotion policy (safety net)

* Any confirmed Stable FP/FN → demote cop to Preview immediately (until fixed).
* This keeps CI adoption safe even if initial Stable set is aggressive.

### Autocorrect policy tied to tiers

* Stable autocorrect: allowed in `--autocorrect=safe` (default can be `off` or `safe`, your call).
* Preview autocorrect: requires `--autocorrect=unsafe` (explicit opt-in).

---

## 3) CLI commands (minimal but adoption-focused)

### Phase 1 commands

* `turbocop check [PATH]`

  * reads `.rubocop.yml`
  * runs Stable cops; Preview cops only with `--preview`
  * supports `--format json` for tooling
* `turbocop migrate [PATH]`  ✅ ship early

  * prints a table of cops enabled by config grouped by:

    * Stable (will run)
    * Preview (requires `--preview`)
    * Unimplemented
    * Outside baseline
  * optionally prints “recommended command line” for CI, e.g.:

    * `turbocop check --preview --fail-level warning .`
* `turbocop doctor`

  * prints baseline versions + config sources + skip summary + autocorrect mode
* `turbocop rules [--tier stable|preview] [--format table|json]`

  * lists rule name, tier, short description, known divergence count (if available)

### Bridge tool (Ruby required — be explicit)

* `turbocop verify [PATH]` (optional but promoted)

  * runs RuboCop as oracle (requires Ruby + gems)
  * diffs outputs per cop and prints summary
  * intended for migration/CI confidence, not your “single binary” story

---

## 4) Exit codes, fail levels, and CI semantics (define now)

### Offense handling

* `--fail-level` (RuboCop-like): `refactor|convention|warning|error|fatal` (or whatever categories you use)
* Exit code is non-zero if any offense at or above fail-level exists.

### Skipped cops handling (important)

Introduce `--strict` with a clear contract:

* **Default (non-strict):**

  * Skipped cops **do not** affect exit code.
  * They produce the grouped “skipped cops” notice (unless suppressed).

* **Strict (`--strict`):**

  * If config enables any cop that is skipped for *any* reason (preview-gated, unimplemented, outside baseline), exit code is **2** (distinct from lint failures).
  * This lets CI enforce “no gaps” when teams want that.

Suggested exit codes:

* `0`: no offenses at/above fail-level, and (in strict mode) no skips
* `1`: lint offenses at/above fail-level
* `2`: strict-mode skip/coverage failure (config references cops turbocop didn’t run)
* `3`: internal tool error/crash

(You can pick numbers, but the *separation* is the key.)

---

## 5) NodePattern verifier (prioritized, but scoped correctly)

### Purpose

Prevent matcher-layer drift and catch Prism mapping mistakes early:

* Compare **NodePattern interpreter** match results vs **compiled Rust matcher** on real AST nodes.

### What it does and doesn’t prove

* **Does prove:** matcher equivalence to NodePattern (given your interpreter)
* **Does not prove:** cop logic, config semantics, autocorrect behavior

### Implementation plan (fast path)

* Finish interpreter + matcher equivalence harness.
* Feed it AST nodes harvested from:

  * your existing bench repos first
  * then the larger corpus as it comes online
* Run it in CI as a regression guardrail.

---

## 6) RuboCop oracle at scale (the backbone for tiers)

### Build a corpus (Phase 2)

* Clone/snapshot 500–1000 Ruby repos (gems + popular repos).
* Record repo + commit hash.

### Two passes (separates bug categories)

1. **Baseline mode:** run both tools with a controlled config (or none)
2. **Repo-config mode:** run with each repo’s `.rubocop.yml`

### Diffing outputs

* Normalize both tools to comparable JSON:

  * file, line, column, cop name, message, severity, corrected? etc.
* Compute per cop:

  * total RuboCop offenses, turbocop offenses, matches, FPs, FNs, crashes

### Noise bucketing (MVP)

Bucket diffs into:

* syntax/parser-related
* missing/outside baseline
* unimplemented
* true behavior diffs
* crashes/timeouts

### Tier updates are data-driven

* Promote Preview → Stable when corpus shows near-zero diffs and no crashes.
* Demote Stable → Preview when diffs appear or a regression is reported.

Output artifacts:

* `tiers.json` (checked in)
* `compatibility.md` (generated table)
* “Top diverging cops” list (roadmap)

---

## 7) Regression flywheel (turn diffs into tests)

For every true diff found by corpus or verify:

1. save repro fixture (file + minimal config context)
2. add to test suite
3. (later) implement minimization

MVP: store the whole file first; minimizing comes after.

---

## 8) Messaging & docs (ship with Phase 1)

### README top section must include

* baseline-defined behavior (“RuboCop-inspired” not perfect drop-in)
* Stable vs Preview contract + how to enable preview
* `turbocop migrate` and `turbocop verify` as the first-run answers

### Compatibility table

Even before corpus is perfect:

* Publish tier list + baseline versions.
* Update it as corpus runs.

---

## 9) Shipping sequence (ruthless, low scope creep)

### Phase 1 (adoption + safety)

* Stable/Preview tiers (mostly Stable initially)
* `.rubocop.yml` reading + skip classification + grouped notice
* `migrate` (ship early) + `doctor`
* exit code + strict semantics nailed down
* NodePattern verifier completed + CI integrated
* (Optional but recommended) `verify` command (Ruby required, very explicit)

### Phase 2 (measurement)

* corpus runner + basic diffing + per-cop stats
* generate `tiers.json` + compatibility table
* start promoting/demoting based on data

### Phase 3 (flywheel + polish)

* regression fixtures + minimizer
* better bucketing
* refine docs and UX based on adoption feedback

---

## 10) First-time user flow (what happens)

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

* sees a table: “these will run, these need --preview, these aren’t implemented, these are outside baseline”
* gets a suggested CI command line

3. Skeptical team runs:

```bash
turbocop verify .
```

* gets a diff against RuboCop oracle (Ruby required), per cop
