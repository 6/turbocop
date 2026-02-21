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

* Stable autocorrect: allowed with `-a` / `--autocorrect=safe` (default is off).
* Preview autocorrect: requires `-A` / `--autocorrect=all` (explicit opt-in for unsafe corrections).

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

  * prints baseline versions + config sources + skip summary + autocorrect mode + gem version mismatch warnings
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

* **Strict mode** with configurable scope:

  * `--strict=coverage` **(recommended default for `--strict`):**
    Fail (exit 2) only for cops turbocop claims to support but didn’t run
    (e.g., preview-gated cops the user didn’t opt into). Cops that are
    unimplemented or outside baseline are treated as informational skips,
    not failures. This is the useful default — “turbocop ran everything
    it should have.”

  * `--strict=implemented-only`:
    Ignore unknown/outside-baseline cops entirely. Only fail if a cop
    turbocop implements (Stable or Preview) was skipped. Useful for teams
    that know they use unsupported plugins and don’t want noise.

  * `--strict=all`:
    Any skipped cop for any reason (preview-gated, unimplemented, outside
    baseline) causes exit 2. The most conservative option. Only useful for
    teams whose config is fully within turbocop’s baseline.

  Without the `=` form, `--strict` defaults to `coverage`.

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

### Build a corpus (phased, not heroic)

**Phase 2 target: ~100 repos** (diverse + curated). This is enough to cover
the major config patterns, gem combinations, and Ruby idioms. The current 14
public + 14 internal repos already surface most issues — 100 is the next step.

**Phase 3 target: ~300 repos** — only if Phase 2 is still producing novel
diffs. If 100 repos stopped surfacing new issues at repo 60, 300 isn’t worth it.

**Phase 4 target: 500-1000 repos** — optional. Only if using tarballs/eviction
and the pipeline is fully automated. This is a “nice to have” for marketing
(“tested against 1000 repos”) not a correctness necessity.

**Corpus maintenance strategy:**

* **Core frozen set (~50 repos):** never rotated. These are the stability
  baseline — metrics against these repos should never regress. Pin exact
  commit hashes.
* **Rotating set (~50 repos):** refreshed quarterly to find new patterns.
  Swap in repos with unusual configs, new gem versions, different Ruby
  versions. This is the exploration arm.

The harness should be built so scaling up is “add rows to a manifest file,”
not “rewrite the pipeline.”

Record repo + commit hash for reproducibility.

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

* corpus runner + basic diffing + per-cop stats (~100 repos)
* generate `tiers.json` + compatibility table
* start promoting/demoting based on data

### Phase 3 (flywheel + polish)

* regression fixtures + minimizer
* expand corpus to ~300 repos (only if still producing novel diffs)
* better bucketing
* refine docs and UX based on adoption feedback

### Phase 4 (scale, optional)

* corpus to 500-1000 repos (tarball-based, automated maintenance)
* core frozen set (~50) + rotating set for exploration

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
