# turbocop Corpus & Oracle Harness Spec

## Goals

Build a repeatable, apples-to-apples correctness harness that:

* Assembles a **large, diverse Ruby code corpus** (~100 repos, scaling to 300+) pinned to **immutable commits**.
* Runs a **pinned RuboCop "oracle"** (same baseline versions turbocop targets) **without installing each repo's dependencies**.
* Runs turbocop on the same corpus and **diffs results per cop** to drive **Stable/Preview tiering** and regression fixtures.
* Runs entirely in **GitHub Actions CI** — no local setup required, sandboxing handled by ephemeral VMs.

## Non-goals

* Emulating each repo's Gemfile.lock versions.
* Making repo-config mode succeed on every repo regardless of custom plugins.
* Executing test suites or any repo scripts.

---

## Why GitHub Actions (not local execution)

Running the corpus harness in CI rather than locally solves several problems:

1. **Sandboxing is free.** Each job runs in a fresh ephemeral VM that's destroyed after. No need for containers, unprivileged users, overlayfs, or read-only mounts.
2. **Reproducibility.** Pinned runner images, no local machine state drift, results tied to commit SHAs.
3. **Parallelism.** Matrix jobs fan out across repos. Free tier for public repos: 20 concurrent jobs, 2000 min/month.
4. **No local disk management.** Repos are fetched fresh each run (shallow clones are fast). Results are stored as GitHub Artifacts.
5. **Accessible to contributors.** Anyone with a fork can run the corpus without special setup.

### CI budget

For **public repos** (turbocop is open source): GitHub Actions minutes are **unlimited** on standard runners. No budget constraints — run as often as needed.

For private forks: 2000 min/month on the free tier. A full 100-repo run takes ~200–500 min (RuboCop is the bottleneck at ~30s–5min per repo), so ~4–10 runs/month.

Since turbocop is a public repo, this is effectively free. Run nightly, on cop-changing PRs, and on manual trigger without worrying about budget.

---

## Safety constraints (simplified for CI)

GitHub Actions VMs are ephemeral and unprivileged. The main remaining safety rules:

1. **Never run Bundler inside a repo.**

   * No `bundle install` in the repo.
   * No `bundle exec rubocop` using the repo's Gemfile.
   * Always use the bench bundle (pinned baseline versions).

2. **Never allow RuboCop to `require` local repo paths.**

   * In repo-config mode, pre-scan `.rubocop.yml` and skip repos with unsafe `require:` entries (relative/absolute paths, non-allowlisted gems).

3. **Disable submodules** — fetch with `--no-recurse-submodules`.

4. **Never execute repo code** — repos are data (source to lint), not code to run.

---

## Corpus construction

### Repo selection strategy (3 feeds)

Build candidates from multiple feeds for diversity, then dedupe.

1. **GitHub popular Ruby repos**

   * Search for `language:Ruby stars:>500 archived:false fork:false`
   * Also sample mid-tier: `language:Ruby stars:50..500 archived:false fork:false`

2. **RubyGems popular gems**

   * Use RubyGems "most downloaded" / "top" lists to get library-heavy code.

3. **Repos that use RuboCop configs**

   * GitHub search for `filename:.rubocop.yml language:Ruby`

**Dedupe** by canonical repo URL.

### Filters (keep runs tractable)

* Skip if repo has > 50k `.rb` files or > 1GB checkout size.
* Skip if last update is ancient (optional), but keep some older repos for syntax diversity.
* During file discovery, always exclude: `vendor/`, `node_modules/`, `tmp/`, `coverage/`, `dist/`, `build/`, `.git/`

### Manifest format (JSONL, checked in)

Store one line per repo in `bench/corpus/manifest.jsonl`:

```json
{"id":"rails__rails__abc1234","repo_url":"https://github.com/rails/rails","sha":"abc1234...","source":"github_stars","set":"frozen","notes":""}
```

Fields:

* `id`: deterministic `<owner>__<repo>__<sha7>`
* `repo_url`, `sha`: pinned commit
* `source`: which feed found it (`github_stars`, `rubygems`, `rubocop_config`)
* `set`: `frozen` (core ~50 repos, never rotated) or `rotating` (refreshed quarterly)
* `notes`: optional

The manifest is checked into the repo. Adding repos = adding lines to this file.

---

## CI workflow design

### Workflow: `corpus-oracle.yml`

**Triggers:**

* `schedule`: weekly (e.g., Sunday night)
* `workflow_dispatch`: manual trigger with optional params (repo subset, mode)
* `pull_request`: only when paths match `src/cop/**`, `src/config/**`, `resources/tiers.json` (cop implementation changes)

### Job structure

```
┌─────────────────────┐
│   build-turbocop    │  Build release binary, upload as artifact
└──────────┬──────────┘
           │
┌──────────▼──────────┐
│   setup-bench       │  Install bench bundle (Ruby + baseline gems), cache it
└──────────┬──────────┘
           │
┌──────────▼──────────┐
│  corpus-matrix      │  Matrix: one job per repo (or batch of ~5 repos)
│  [repo_1]           │  - Shallow clone repo at pinned SHA
│  [repo_2]           │  - Run RuboCop baseline + repo-config (if eligible)
│  [repo_3]           │  - Run turbocop baseline + repo-config (if eligible)
│  ...                │  - Upload per-repo JSON results as artifacts
└──────────┬──────────┘
           │
┌──────────▼──────────┐
│   collect-results   │  Download all per-repo artifacts
│                     │  Run diff engine + noise bucketing
│                     │  Generate results.md + corpus_results.json
│                     │  Upload as workflow artifacts
│                     │  (Optional) commit results to a branch or create PR
└─────────────────────┘
```

### Matrix batching

With 100 repos and 20 concurrent jobs:

* **Option A**: 1 repo per job (100 jobs, max parallelism, ~5 min each).
* **Option B**: 5 repos per job (20 jobs, better cache reuse, ~25 min each).

Start with Option B (20 jobs × 5 repos) to stay within reasonable job counts. Switch to Option A if individual repos are slow enough to warrant it.

### Caching

* **Bench bundle**: `actions/cache` keyed on `bench/Gemfile.lock` hash. Saves ~2 min/job.
* **turbocop binary**: built once in `build-turbocop` job, shared via `actions/upload-artifact`.
* **Repo checkouts**: NOT cached (shallow clones are fast, and caching 100 repos uses too much cache space).

### Per-repo job steps

```yaml
# Inside each matrix job:
- uses: actions/checkout@v4          # turbocop repo (for bench config)
- uses: actions/download-artifact@v4  # turbocop binary
- uses: ruby/setup-ruby@v1           # Ruby for RuboCop
- name: Restore bench bundle
  uses: actions/cache@v4
  with:
    path: bench/vendor/bundle
    key: bench-bundle-${{ hashFiles('bench/Gemfile.lock') }}

- name: Fetch corpus repos (this batch)
  run: |
    # For each repo in this batch:
    #   git clone --depth 1 --no-recurse-submodules <url> -b <sha> repos/<id>

- name: Pre-scan configs
  run: |
    # Scan .rubocop.yml for require/inherit_gem safety
    # Write eligibility status per repo

- name: Run RuboCop (baseline mode)
  run: |
    for repo in repos/*/; do
      BUNDLE_GEMFILE=$PWD/bench/Gemfile \
      BUNDLE_PATH=$PWD/bench/vendor/bundle \
      bundle exec rubocop \
        --config bench/baseline_rubocop.yml \
        --format json \
        --force-exclusion \
        --cache false \
        "$repo" > "results/rubocop/baseline/$(basename $repo).json" 2>&1 || true
    done

- name: Run RuboCop (repo-config mode, eligible only)
  run: |
    # Similar, but use repo's .rubocop.yml for eligible repos

- name: Run turbocop (both modes)
  run: |
    for repo in repos/*/; do
      ./turbocop --format json "$repo" > "results/turbocop/baseline/$(basename $repo).json" 2>&1 || true
    done

- name: Upload per-repo results
  uses: actions/upload-artifact@v4
  with:
    name: corpus-results-${{ matrix.batch }}
    path: results/
```

### Collect job

The final `collect-results` job:

1. Downloads all per-repo result artifacts.
2. Runs the diff engine (Rust binary or script) to produce:
   * `corpus_results.json` — detailed per-cop, per-repo FP/FN/match data
   * `results.md` — human-readable summary with:
     * Overall match rate
     * Per-cop divergence table (top 30 diverging cops)
     * Per-repo summary (match rate, timing, status)
     * Noise bucket breakdown
     * Autocorrect parity summary (if autocorrect lane ran)
3. Uploads both as workflow artifacts (visible in GitHub Actions UI).
4. (Optional) Opens a PR or commits to a `corpus-results` branch for easy diffing between runs.

---

## Oracle: running RuboCop at turbocop's baseline

### Baseline bundle

`bench/Gemfile` pins baseline versions:

```ruby
source "https://rubygems.org"

gem "rubocop", "1.xx.x"
gem "rubocop-rails", "2.yy.y"
gem "rubocop-rspec", "3.zz.z"
gem "rubocop-performance", "1.aa.a"
# add any baseline plugins turbocop vendors
```

All oracle runs use `BUNDLE_GEMFILE=bench/Gemfile` + `BUNDLE_PATH=bench/vendor/bundle`. Never the repo's Gemfile.

### Ruby version

Pin in the workflow: `ruby/setup-ruby@v1` with a specific version (e.g., `3.3`). Record in results metadata.

---

## Repo-config mode: allowlist or skip

### Pre-scan `.rubocop.yml`

Lightweight YAML scan extracts: `require:`, `inherit_gem:`, `inherit_from:`, `AllCops: TargetRubyVersion`.

### Allowlist policy

`allowed_rubocop_plugins` = exactly the gems in `bench/Gemfile`.

Repo-config mode is **eligible** only if:

* every `require:` is either absent or a gem name in the allowlist (no paths)
* every `inherit_gem:` references allowlisted gems

Ineligible repos run **baseline mode only** and are bucketed as `config_deps_missing_or_unsafe`.

---

## Two passes

### Pass 1: Baseline mode (clean semantics signal)

* Ignore repo `.rubocop.yml`.
* Use `bench/baseline_rubocop.yml` (controlled config).
* Exclude standard junk directories.

### Pass 2: Repo-config mode (config compatibility signal)

* Only for eligible repos (per allowlist scan).
* Use repo's `.rubocop.yml`.
* Same baseline bundle (never repo's Gemfile).

---

## Diffing & noise buckets

### Normalized diagnostic key

Compare offenses by: `file + line + column + cop_name`

### Buckets

* `syntax_or_parse` — Prism vs Parser recovery differences
* `gem_version_mismatch` — cop behavior differs due to version gap (already detected by bench harness)
* `outside_baseline` — cop not in turbocop's baseline
* `unimplemented` — cop in baseline but not implemented
* `config_deps_missing_or_unsafe` — repo-config mode skipped
* `true_behavior_diff` — genuine implementation divergence
* `tool_crash_or_timeout`

Only `true_behavior_diff` counts against Stable promotion.

---

## Output artifacts

### `results.md` (human-readable summary)

```markdown
# Corpus Oracle Results — 2026-02-21

**Corpus**: 100 repos (50 frozen + 50 rotating)
**Baseline**: rubocop 1.xx.x, rubocop-rails 2.yy.y, ...
**turbocop**: v0.x.x (commit abc1234)

## Overall
| Metric | Count |
|--------|------:|
| Total repos | 100 |
| Repos with 100% match | 72 |
| Total offenses compared | 45,231 |
| Matches | 44,890 |
| True FP | 142 |
| True FN | 199 |
| Overall match rate | 99.2% |

## Top diverging cops
| Cop | Matches | FP | FN | Match % |
|-----|--------:|---:|---:|--------:|
| ... | ... | ... | ... | ... |

## Per-repo summary
| Repo | Match rate | True FP | True FN | Status |
|------|----------:|--------:|--------:|--------|
| ... | ... | ... | ... | ... |

## Noise buckets
| Bucket | Count |
|--------|------:|
| syntax_or_parse | ... |
| gem_version_mismatch | ... |
| ... | ... |
```

### `corpus_results.json` (machine-readable)

```json
{
  "schema": 1,
  "run_date": "2026-02-21T...",
  "baseline": {"rubocop": "1.xx.x", "...": "..."},
  "turbocop_version": "0.x.x",
  "turbocop_commit": "abc1234",
  "summary": {
    "total_repos": 100,
    "repos_100pct_match": 72,
    "total_offenses": 45231,
    "matches": 44890,
    "true_fp": 142,
    "true_fn": 199
  },
  "by_cop": [
    {"cop": "Style/Foo", "matches": 500, "fp": 2, "fn": 0, "noise": {"syntax": 1}},
    "..."
  ],
  "by_repo": [
    {"repo": "rails__rails__abc1234", "status": "ok", "match_rate": 0.99, "fp": 3, "fn": 1},
    "..."
  ],
  "noise_buckets": {
    "syntax_or_parse": 45,
    "gem_version_mismatch": 102,
    "outside_baseline": 0,
    "true_behavior_diff": 341,
    "tool_crash_or_timeout": 0
  }
}
```

### Autocorrect results (when autocorrect lane runs)

Appended to both `results.md` and `corpus_results.json`:

* Per-cop autocorrect parity (match/mismatch/gate failures)
* Safety gate pass rates
* Autocorrect allowlist candidates (cops with 0 mismatches)

---

## Autocorrect oracle harness (separate CI lane)

Autocorrect is higher risk — a wrong rewrite silently breaks code. Runs as a separate CI job or workflow.

**Existing infrastructure**: `bench_turbocop autocorrect-conform` already copies bench repos, runs both tools with `-A`, and diffs `.rb` files. The CI lane extends this to the full corpus with per-cop granularity and safety gates.

### CI workflow addition

Add an `autocorrect-oracle` job (or separate workflow) that:

1. For each repo, creates a temp copy of the checkout.
2. Runs RuboCop `--autocorrect` (safe only) with the bench bundle, restricted to allowlisted cops.
3. Captures post-state file hashes.
4. Resets to pre-state, runs turbocop `-a` with the same cop set.
5. Diffs post-state file hashes between the two tools.
6. Runs safety gates on turbocop output: parse, idempotence, non-overlap.
7. Uploads per-repo autocorrect results as artifacts.

### Safety gates

* **Parse gate**: every changed file parses successfully with Prism.
* **Idempotence gate**: running autocorrect twice yields no further edits.
* **Non-overlap gate**: edits don't overlap and have valid byte ranges.
* **No-op gate**: if tool reports edits, at least one file hash must change.

Gate failures are bucketed as `autocorrect_invalid_output` (higher severity than mismatches).

### Noise buckets (autocorrect-specific)

* `autocorrect_mismatch` — outputs differ
* `autocorrect_invalid_output` — safety gate failed
* `autocorrect_oracle_failed` — RuboCop crashed
* `autocorrect_tool_failed` — turbocop crashed

### Allowlist promotion

Cops enter `autocorrect_safe_allowlist.json` only after 0 mismatches + 0 gate failures across the corpus. Any bug report removes the cop until fixed.

---

## Implementation checklist

### Phase 0: Foundations

* [x] **Define baseline versions** in one place

  * `bench/corpus/Gemfile` pins RuboCop + plugins to turbocop baseline
  * **Done when**: `bench/corpus/Gemfile` versions match vendor submodule tags.

* [x] **Create `bench/corpus/manifest.jsonl`** with initial ~20 repos

  * 14 bench repos (known-good) + 8 new repos for diversity (22 total).
  * All marked as `set: "frozen"` with pinned SHAs.
  * **Done when**: manifest is checked in and parseable.

* [x] **Implement path exclusions** shared by all runs

  * `bench/corpus/baseline_rubocop.yml` excludes: `vendor/`, `node_modules/`, `tmp/`, `coverage/`, `dist/`, `build/`, `.git/`, `db/schema.rb`, `bin/`
  * **Done when**: both tools use equivalent exclusions.

---

### Phase 1: CI workflow MVP (~20 repos)

* [x] **`build-turbocop` job**

  * Build release binary on `ubuntu-24.04`.
  * Upload as artifact.
  * **Done when**: binary builds and is downloadable by downstream jobs.

* [x] **`setup-bench` or inline step**

  * Install Ruby + corpus bundle per batch job.
  * Cache with `actions/cache` keyed on `bench/corpus/Gemfile` hash.
  * **Done when**: `bundle exec rubocop --version` prints baseline version.

* [x] **`corpus-matrix` job (batch of ~5 repos per job)**

  * Read manifest, compute matrix batches.
  * Per batch: shallow clone repos, run both tools, upload results.
  * **Done when**: per-repo JSON results are uploaded for all repos.

* [x] **`collect-results` job**

  * Download all artifacts, run diff engine, generate `corpus-results.md` + `corpus-results.json`.
  * Upload as workflow artifacts.
  * **Done when**: results are visible in the Actions tab after a successful run.

* [x] **Diff engine** (`bench/corpus/diff_results.py`)

  * Accept per-repo JSON files as input.
  * Produce normalized per-cop + per-repo diffs.
  * Output `corpus-results.md` and `corpus-results.json`.
  * Filters to covered cops only via `--cop-list`.
  * **Done when**: diff engine runs in the collect job and output matches expected format.

---

### Phase 2: Scale to ~100 repos + noise bucketing

* [ ] **Expand manifest to ~100 repos**

  * Use the 3 feed sources (GitHub stars, RubyGems, `.rubocop.yml` search).
  * Mark ~50 as `frozen`, ~50 as `rotating`.
  * **Done when**: manifest has ~100 entries with diverse sources.

* [ ] **Config pre-scan + eligibility**

  * Scan `.rubocop.yml` for unsafe `require:` / `inherit_gem:`.
  * Skip repo-config mode for ineligible repos, bucket appropriately.
  * **Done when**: repo-config mode only runs on safe repos.

* [ ] **Noise bucket classification**

  * Assign diffs into: `syntax_or_parse`, `gem_version_mismatch`, `outside_baseline`, `unimplemented`, `config_deps_missing_or_unsafe`, `true_behavior_diff`, `tool_crash_or_timeout`.
  * **Done when**: `results.md` has a noise bucket breakdown and only `true_behavior_diff` counts against tiers.

* [ ] **Tier generator** (`gen-tiers`)

  * Input: `corpus_results.json`.
  * Output: `resources/tiers.json` (Stable/Preview overrides).
  * **Done when**: deterministic and reviewable in git.

---

### Phase 3: Autocorrect oracle lane

* [ ] **Add `autocorrect-oracle` job to CI**

  * Temp-copy repos, run both tools' autocorrect, compare post-state, run safety gates.
  * Upload autocorrect results as artifacts.
  * **Done when**: autocorrect parity is reported in `results.md`.

* [ ] **Autocorrect allowlist generator**

  * Input: autocorrect results from CI.
  * Output: `resources/autocorrect_safe_allowlist.json`.
  * **Done when**: allowlist is generated from corpus data.

---

### Phase 4: Regression fixtures

* [ ] **Fixture capture**

  * For any `true_behavior_diff`, extract: offending file, minimal config, expected vs observed.
  * Store in `testdata/corpus_regressions/` (checked in).
  * **Done when**: diffs can be reproduced locally without re-cloning.

* [ ] **(Optional) Minimization**

  * Later; MVP is storing whole files.

---

### Phase 5: Scale to ~300 repos (only if needed)

* [ ] **Expand manifest** (only if Phase 2 still producing novel diffs)
* [ ] **Optimize CI budget**: skip repos that have been 100% match for N consecutive runs
* [ ] **Consider splitting into multiple workflows** if budget is tight

---

## Acceptance criteria for "Corpus MVP complete"

On a corpus of at least ~100 repos:

* CI workflow runs end-to-end on schedule + manual trigger.
* `results.md` is generated as a workflow artifact with overall match rate, top diverging cops, per-repo summary, and noise bucket breakdown.
* `corpus_results.json` is generated with full machine-readable data.
* Tier generator can produce a reviewable `tiers.json` from corpus data.
* Autocorrect oracle lane runs and reports per-cop parity.
* `autocorrect_safe_allowlist.json` is generated from corpus results.
* Entire pipeline requires zero local setup — contributors can trigger from the Actions tab.
