# turbocop Corpus & Oracle Harness Spec

## Goals

Build a repeatable, apples-to-apples correctness harness that:

* Assembles a **large, diverse Ruby code corpus** (500–1000 repos) pinned to **immutable commits**.
* Runs a **pinned RuboCop “oracle”** (same baseline versions turbocop targets) **without installing each repo’s dependencies**.
* Runs turbocop on the same corpus and **diffs results per cop** to drive **Stable/Preview tiering** and regression fixtures.
* Minimizes security risk: **never execute repo code**, minimize network exposure, and avoid per-repo Bundler installs.

## Non-goals

* Emulating each repo’s Gemfile.lock versions.
* Making repo-config mode succeed on every repo regardless of custom plugins.
* Executing test suites or any repo scripts.

---

## Threat model & safety constraints

### What we assume can be malicious

* Repos may contain files that would be dangerous **if executed**.
* `.rubocop.yml` may include `require:` entries that could attempt to `require` local files.
* Some repos use `inherit_gem` / `require` for third-party RuboCop plugins.

### Safety rules (hard requirements)

1. **Never run Bundler inside a repo**.

   * No `bundle install` in the repo.
   * No `bundle exec rubocop` using the repo’s Gemfile.

2. **Never allow RuboCop to `require` local repo paths.**

   * In repo-config mode, treat any `require:` that is:

     * a relative path (`./foo`, `foo.rb`, `../bar`), or
     * an absolute path, or
     * anything not in an allowlisted gem-name form
       as **unsafe**. Skip repo-config mode for that repo.

3. Run harness in a **sandboxed environment**:

   * Container/VM, unprivileged user.
   * Prefer **no network** during execution runs (fetching can be separate phase).
   * Read-only mount corpus during linting runs.

4. Explicitly **disable submodules** and avoid any `git` features that execute hooks.

   * No `--recurse-submodules`.
   * Don’t run any repo-provided hooks/scripts.

---

## Corpus construction

### Repo selection strategy (3 feeds)

Build candidates from multiple feeds for diversity, then dedupe.

1. **GitHub popular Ruby repos**

   * Search for `language:Ruby stars:>500 archived:false fork:false`
   * Also sample mid-tier: `language:Ruby stars:50..500 archived:false fork:false`

2. **RubyGems popular gems**

   * Use RubyGems “most downloaded” / “top” lists to get library-heavy code.

3. **Repos that use RuboCop configs**

   * GitHub search for `filename:.rubocop.yml language:Ruby`

**Dedupe** by canonical repo URL.

### Filters (keep runs tractable)

Apply filters during fetch:

* Skip if repo size exceeds limits (pick limits that match your hardware):

  * e.g. > 2GB checkout or > 200k files.
* Skip if last update is ancient (optional), but keep some older repos for syntax diversity.
* During file discovery, always exclude:

  * `vendor/`, `node_modules/`, `tmp/`, `coverage/`, `dist/`, `build/`, `.git/`

### Snapshotting (pin to SHA)

Each corpus entry is **pinned to a commit SHA** to prevent flakiness.

* For each chosen repo:

  * record `repo_url`, `commit_sha`, and metadata (stars, feed source, fetched_at).

### Manifest format (JSONL)

Store one line per repo:

```json
{"id":"rails__rails__<sha>","repo_url":"https://github.com/rails/rails","sha":"<sha>","source":"github_stars","fetched_at":"2026-02-21","notes":""}
```

### Storage layout

```
corpus/
  manifest.jsonl
  repos/
    rails__rails__<sha>/
      checkout/   # working tree
      meta.json
      config_scan.json
```

---

## Fetching repos efficiently (without surprises)

### Principle

Fetching source code is safe as long as you treat it as **data** and never execute it.

### Recommended fetch approach

* Fetch pinned SHAs with minimal history.
* Avoid submodules.
* Avoid large binary blobs when possible.

Examples (choose one):

#### Option 1: Shallow fetch by SHA (works for many repos)

```bash
mkdir -p corpus/repos/$ID
cd corpus/repos/$ID

git init checkout
cd checkout

git remote add origin "$URL"
# Fetch only the pinned commit
git fetch --depth 1 origin $SHA
# Checkout detached at SHA
git checkout --detach $SHA
```

#### Option 2: GitHub tarball at SHA (no git history)

* Download `archive/<sha>.tar.gz`, unpack into `checkout/`.
* Pros: very space efficient.
* Cons: requires HTTP fetch logic; no git metadata.

### Hard fetch rules

* Do not recurse submodules.
* Do not run any repo scripts.
* Keep corpus checkouts read-only during lint runs.

---

## Oracle: running RuboCop at turbocop’s baseline WITHOUT per-repo bundle installs

### Baseline bundle

Create a dedicated “bench bundle” that pins your baseline versions.

`bench/Gemfile`:

```ruby
source "https://rubygems.org"

gem "rubocop", "1.xx.x"
gem "rubocop-rails", "2.yy.y"
gem "rubocop-rspec", "3.zz.z"
gem "rubocop-performance", "1.aa.a"
# add any baseline plugins turbocop vendors
```

Install once:

```bash
cd bench
bundle install --path vendor/bundle
```

### Oracle run command (baseline bundle)

All oracle runs must use the baseline bundle, never the repo’s bundle:

```bash
BUNDLE_GEMFILE=$BENCH_DIR/Gemfile \
BUNDLE_PATH=$BENCH_DIR/vendor/bundle \
bundle exec rubocop --format json --force-exclusion --cache false --stdin < /dev/null
```

Notes:

* Run in repo working directory (so file paths are correct), but **do not use repo Gemfile**.
* `--cache false` avoids RuboCop writing caches into repos (or configure a dedicated cache dir).

### Ruby version

Pick a single Ruby version consistent with your baseline (e.g., Ruby 3.3+). Record it in results metadata.

---

## Repo-config mode safety: allowlist or skip

Repo-config mode is valuable but is where safety + dependency issues appear.

### Pre-scan `.rubocop.yml` before running oracle

Implement a lightweight YAML scan that extracts:

* `require:` entries
* `inherit_gem:` entries
* `inherit_from:` entries
* `AllCops: TargetRubyVersion` (if present)

Write result to `config_scan.json`.

### Allowlist policy

Define `allowed_rubocop_plugins` (exactly the gems included in your baseline bundle, optionally plus a small “extended” list).

Repo-config mode is **eligible** only if:

* every `require:` is either absent or a gem name in allowlist (no paths)
* every `inherit_gem:` references allowlisted gems

If not eligible, bucket repo-config mode as:

* `config_deps_missing_or_unsafe`

…and do **baseline mode only**.

This prevents accidentally executing repo-local code via `require: ./something`.

---

## Two passes (exact definitions)

### Pass 1: Baseline mode (clean semantics signal)

Purpose: measure core cop semantics without config/plugin variability.

* Ignore repo `.rubocop.yml`.
* Use a harness config you control (minimal).
* Exclude standard junk directories.

Example oracle invocation:

```bash
BUNDLE_GEMFILE=bench/Gemfile bundle exec rubocop \
  --config bench/baseline_rubocop.yml \
  --format json \
  --force-exclusion \
  --cache false \
  $REPO_ROOT
```

### Pass 2: Repo-config mode (config compatibility signal)

Purpose: measure how well turbocop matches RuboCop when using real configs.

* Only run if repo is eligible per allowlist scan.
* Use repo `.rubocop.yml`.

Example oracle invocation:

```bash
BUNDLE_GEMFILE=bench/Gemfile bundle exec rubocop \
  --format json \
  --force-exclusion \
  --cache false \
  $REPO_ROOT
```

Record per repo whether baseline mode and/or repo-config mode ran.

---

## Result storage & caching

### Per-repo result files

Store results so you can re-diff without re-running tools:

```
results/
  rubocop/
    baseline/<repo_id>.json
    repo_config/<repo_id>.json
  turbocop/
    baseline/<repo_id>.json
    repo_config/<repo_id>.json
  meta/
    <repo_id>.json
```

### Metadata per run

`meta/<repo_id>.json` includes:

* repo_id, url, sha
* mode: baseline vs repo_config
* tool versions: turbocop baseline, rubocop baseline gem versions
* ruby version
* status: ok | skipped (reason) | error (reason)
* timing stats

---

## Diffing & noise buckets

### Normalized diagnostic key (MVP)

Compare offenses by:

* `file + line + column + cop_name`

(Optionally include message normalization later.)

### Buckets (minimum viable)

Classify diffs into:

* `syntax_or_parse` (RuboCop reports syntax, turbocop doesn’t; parse errors)
* `outside_baseline` (cop not present)
* `unimplemented`
* `config_deps_missing_or_unsafe`
* `true_behavior_diff`
* `tool_crash_or_timeout`

Only `true_behavior_diff` counts against Stable promotion.

---

## Scaling & performance

### Parallelization

Parallelize by **repo**, not by file:

* each worker takes a repo_id + mode, produces one JSON output.
* avoid shared mutable state.

### Disk management

* Prefer tarball fetch or shallow fetch to reduce space.
* Keep checkouts only as long as needed; optionally “evict” old checkouts and keep only manifests + results.
* Consider storing only the subset of files needed for repro fixtures (later phase).

---

## Implementation checklist

This section is deliberately concrete: it’s the work items an engineer can execute in order, with clear “done” criteria.

### Phase 0: Foundations (plumbing + safety)

* [ ] **Define baseline versions** in one place (source of truth)

  * `bench/Gemfile` pins `rubocop` + plugins to turbocop baseline
  * `bench/baseline_versions.json` mirrors the same versions for metadata
  * **Done when**: `turbocop doctor` and harness metadata can print the same baseline set.

* [ ] **Create sandbox execution wrapper**

  * Unprivileged user, optional container/VM support
  * “no network during runs” toggle (fetch phase may use network)
  * Read-only mount of `corpus/repos/*/checkout` during tool runs
  * **Done when**: a simple repo can be linted in baseline mode with no network.

* [ ] **Implement path exclusions** shared by all runs

  * Hard exclude: `.git/`, `vendor/`, `node_modules/`, `tmp/`, `coverage/`, `dist/`, `build/`
  * **Done when**: both RuboCop and turbocop are invoked with equivalent exclusions.

---

### Phase 1: Corpus manifest + fetcher (repeatable inputs)

* [ ] **Manifest schema + loader/writer** (`manifest.jsonl`)

  * Fields: `id`, `repo_url`, `sha`, `source`, `fetched_at`, `notes`
  * Deterministic `id` format: `<owner>__<repo>__<sha7>` (or similar)
  * **Done when**: harness can read/write manifest and iterate repos deterministically.

* [ ] **Repo candidate list builder** (produces an initial manifest)

  * Accepts multiple input feeds (GitHub stars list, RubyGems list, `.rubocop.yml` search)
  * Dedup by canonical URL
  * Applies size/activity filters (configurable)
  * **Done when**: produces a manifest of N repos and logs inclusion/exclusion reasons.

* [ ] **Fetcher** (pins repos to SHAs)

  * Implement either:

    * Git tarball download at SHA, or
    * shallow git fetch by SHA
  * Must not fetch submodules
  * Writes `meta.json` per repo
  * **Done when**: `corpus/repos/<id>/checkout` exists and is at the pinned SHA.

---

### Phase 2: Config pre-scan + eligibility (safe repo-config mode)

* [ ] **Config scanner** for `.rubocop.yml`

  * Extract: `require`, `inherit_gem`, `inherit_from`, `AllCops: TargetRubyVersion`
  * Output: `config_scan.json`
  * **Done when**: scanner is fast (<100ms typical) and robust to YAML quirks.

* [ ] **Eligibility classifier**

  * Reject/mark unsafe if any `require:` looks like a path (relative/absolute) or non-allowlisted value
  * Reject/mark missing deps if `inherit_gem` / `require` references gems not in allowlist
  * Emits bucket reason: `config_deps_missing_or_unsafe`
  * **Done when**: repo-config mode only runs on allowlisted configs.

* [ ] **Allowlist definition**

  * `allowed_rubocop_plugins` defaults to exactly the gems present in `bench/Gemfile`
  * Optional: support an “extended” allowlist bundle later
  * **Done when**: eligibility uses allowlist and never expands it implicitly.

---

### Phase 3: Oracle runner (RuboCop baseline bundle, no per-repo bundler)

* [ ] **Bench bundle install (one-time)**

  * `bundle install --path bench/vendor/bundle`
  * Ensure RuboCop runs from the bench bundle with `BUNDLE_GEMFILE` and `BUNDLE_PATH`
  * **Done when**: `bench/bundle_doctor` command prints exact gem versions.

* [ ] **RuboCop runner: baseline mode**

  * Uses `--config bench/baseline_rubocop.yml`
  * Uses `--format json`, `--force-exclusion`, `--cache false` (or dedicated cache dir)
  * Writes `results/rubocop/baseline/<repo_id>.json` + `results/meta/<repo_id>__baseline.json`
  * **Done when**: can run across 10 repos without touching repo Gemfiles.

* [ ] **RuboCop runner: repo-config mode (eligible only)**

  * Uses repo’s `.rubocop.yml`
  * Same JSON output pathing under `results/rubocop/repo_config/`
  * **Done when**: runs successfully on a subset and correctly buckets ineligible repos.

---

### Phase 4: turbocop runner (same modes, same normalization)

* [ ] **turbocop JSON output schema v1**

  * Includes: diagnostics[], baseline versions, skipped summary buckets
  * **Done when**: schema is stable and can be diffed without ad-hoc parsing.

* [ ] **turbocop runner: baseline mode**

  * Mirrors RuboCop baseline-mode file discovery/exclusions as closely as possible
  * Outputs to `results/turbocop/baseline/<repo_id>.json` + meta
  * **Done when**: outputs exist for same 10 repos and include timing + status.

* [ ] **turbocop runner: repo-config mode**

  * Uses repo `.rubocop.yml` only when eligible by the same classifier
  * Outputs to `results/turbocop/repo_config/<repo_id>.json`
  * **Done when**: mode parity with RuboCop runner (eligible repos line up).

---

### Phase 5: Diff engine + reports

* [ ] **Normalizer** (RuboCop JSON → normalized diagnostics)

  * Produce comparable `DiagnosticKey = (file,line,col,cop)` records
  * **Done when**: normalization is deterministic across runs.

* [ ] **Diffing**

  * Compute per-cop: matches, FP, FN
  * Compute repo-level health: ok / skipped(reason) / error(reason)
  * **Done when**: `turbocop-bench diff --by-cop` produces a stable table.

* [ ] **Noise bucket classification (MVP)**

  * Assign diffs into: `syntax_or_parse`, `outside_baseline`, `unimplemented`, `config_deps_missing_or_unsafe`, `true_behavior_diff`, `tool_crash_or_timeout`
  * **Done when**: tier promotion excludes non-true diffs.

* [ ] **Artifacts**

  * `diff/by_cop.json` (machine-readable)
  * `diff/by_cop.md` (human table)
  * `diff/top_divergences.md`
  * **Done when**: implementers can answer “which cops diverge most?” immediately.

---

### Phase 6: Tier generation + integration

* [ ] **Tier generator** (`gen-tiers`)

  * Input: `diff/by_cop.json`
  * Output: `tiers.json` (Stable/Preview overrides)
  * Applies the Tier Promotion Criteria gates (excluding noise buckets)
  * **Done when**: running generator is deterministic and changes are reviewable in git.

* [ ] **Wire tiers into turbocop**

  * turbocop reads `tiers.json` at build-time or runtime
  * `--preview` enables preview cops
  * `migrate` reports Stable vs Preview vs skipped categories
  * **Done when**: toggling tiers changes what runs, and `migrate` reflects it.

---

### Phase 7: Regression fixtures (turn diffs into tests)

* [ ] **Fixture capture**

  * For any `true_behavior_diff`, store:

    * offending file (or minimal file set)
    * minimal config context
    * expected RuboCop diagnostics
    * observed turbocop diagnostics
  * **Done when**: a diff can be reproduced locally without re-cloning the repo.

* [ ] **(Optional) Minimization**

  * Add later; MVP is storing whole files first.

---

### Phase 8: Autocorrect oracle harness (separate lane)

Autocorrect is higher risk than linting — a wrong rewrite silently breaks code. Treat as an independent oracle lane with stricter gates and conservative defaults.

**Existing infrastructure**: `bench_turbocop autocorrect-conform` already copies bench repos, runs `rubocop -A` and `turbocop -A`, and diffs `.rb` files. Extend this with per-cop granularity and safety gates.

* [ ] **Isolated working copy mechanism**

  * Temp copy of each repo checkout (or overlayfs).
  * Never modify canonical `corpus/repos/*/checkout`.
  * **Done when**: autocorrect runs on a disposable copy and original is untouched.

* [ ] **Pre/post state capture**

  * Enumerate Ruby files, record per-file SHA-256 hash before and after autocorrect.
  * Save unified diff per changed file.
  * **Done when**: can compare pre→post for both RuboCop and turbocop.

* [ ] **Oracle autocorrect runner (safe mode)**

  * Run RuboCop baseline bundle with `--autocorrect` (safe only).
  * Restrict to cops in `autocorrect_safe_allowlist.json`.
  * Store oracle post-state hashes + diffs under `results/autocorrect/rubocop/baseline_safe/`.
  * **Done when**: oracle run completes on 10 repos without touching repo Gemfiles.

* [ ] **turbocop autocorrect runner (safe mode)**

  * Run turbocop with `-a` restricted to same cop allowlist.
  * Store post-state under `results/autocorrect/turbocop/baseline_safe/`.
  * **Done when**: turbocop run completes on same 10 repos.

* [ ] **Safety gates (must-pass before oracle comparison)**

  * **Parse gate**: every changed file parses successfully with Prism.
  * **Idempotence gate**: running autocorrect twice yields no further edits.
  * **Non-overlap gate**: edits don't overlap and have valid byte ranges.
  * **No-op gate**: if tool reports edits, at least one file hash must change.
  * **Done when**: gates run automatically and failures are bucketed as `autocorrect_invalid_output`.

* [ ] **Autocorrect diffing**

  * Compare post-state file hashes between RuboCop and turbocop.
  * Per-cop: match, mismatch, oracle_failed, tool_failed, invalid_output.
  * **Done when**: per-cop autocorrect parity table is produced.

* [ ] **Autocorrect allowlist generator**

  * Input: autocorrect diff results.
  * Output: `autocorrect_safe_allowlist.json` — only cops with 0 mismatches + 0 gate failures.
  * **Done when**: allowlist is deterministically generated and reviewable in git.

* [ ] **Autocorrect noise buckets**

  * `autocorrect_mismatch` — outputs differ
  * `autocorrect_invalid_output` — safety gate failed (higher severity)
  * `autocorrect_oracle_failed` — RuboCop crashed
  * `autocorrect_tool_failed` — turbocop crashed
  * **Done when**: buckets are assigned and only `autocorrect_mismatch` blocks promotion.

* [ ] **Autocorrect repro artifacts**

  * Store per mismatch: `pre.rb`, `rubocop_post.rb`, `turbocop_post.rb`, patches, `meta.json`.
  * Under `results/autocorrect_artifacts/<repo_id>/<cop_name>/`.
  * **Done when**: any mismatch can be reproduced locally.

---

### Acceptance criteria for “Phase 2 MVP complete”

Phase 2 MVP is complete when, on a corpus of at least ~100 repos:

* Baseline mode runs RuboCop from the **bench bundle** (no per-repo Bundler installs).
* Repo-config mode runs only on eligible repos and safely buckets the rest.
* turbocop produces normalized JSON results in both modes.
* Diffing produces a per-cop FP/FN table and highlights top divergences.
* Tier generator can output a reviewable `tiers.json` that turbocop uses.
* Autocorrect oracle (safe mode) runs on the corpus and produces per-cop pass/fail.
* `autocorrect_safe_allowlist.json` is generated from corpus results.
