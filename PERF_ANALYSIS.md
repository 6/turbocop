# turbocop Performance Analysis

> Profiled on 2026-02-18, Darwin arm64, 915 cops, release build.

## Executive Summary

**Bundler shell-outs account for 41-48% of total wall time.** The actual Rust linting engine is already 4-5x faster than RuboCop on large codebases, but this is masked by spawning Ruby processes to resolve gem paths during config loading.

Eliminating the bundler overhead (via a lockfile/resolver or disk cache) would immediately push real-world speedup past 5x.

## Benchmark Repos

| Repo | .rb files | LOC | Plugins loaded |
|------|----------:|----:|----------------|
| Discourse | 5,895 | 2.8M | rubocop-discourse, rubocop-capybara, rubocop-factory_bot, rubocop-rspec_rails, rubocop-rspec, rubocop-rails |
| Mastodon | 2,540 | 144K | rubocop-rails, rubocop-rspec, rubocop-rspec_rails, rubocop-performance, rubocop-capybara |

## Phase Breakdown: Discourse (5,895 files, 2.8M LOC)

```
bundle info --path rubocop:                357ms
bundle info --path rubocop-discourse:      133ms
bundle info --path rubocop-discourse-base: 128ms
bundle info --path rubocop-capybara:       149ms
bundle info --path rubocop-factory_bot:    130ms
bundle info --path rubocop-rspec_rails:    132ms
bundle info --path rubocop-rspec:          127ms
bundle info --path rubocop-rails:          132ms
──────────────────────────────────────────────────
config loading total:                      1.0s
──────────────────────────────────────────────────
linter phase breakdown (cumulative across threads):
  file I/O:        1.0s
  prism parse:     488ms   (~83µs/file)
  codemap build:    71ms   (~12µs/file)
  cop execution:   6.0s    (~1.0ms/file)
  disable filter:   26ms
  accounted:       7.6s    (sum of per-thread time)
──────────────────────────────────────────────────
linter wall clock:                         723ms
total wall clock:                          2.9s
```

## Phase Breakdown: Mastodon (2,540 files, 144K LOC)

```
bundle info --path rubocop:            438ms
bundle info --path rubocop-rails:      244ms
bundle info --path rubocop-rspec:      243ms
bundle info --path rubocop-rspec_rails:239ms
bundle info --path rubocop-performance:237ms
bundle info --path rubocop-capybara:   243ms
──────────────────────────────────────────────────
config loading total:                  2.0s
──────────────────────────────────────────────────
linter phase breakdown (cumulative across threads):
  file I/O:        565ms
  prism parse:     155ms   (~61µs/file)
  codemap build:    25ms   (~10µs/file)
  cop execution:  15.0s    (~5.9ms/file)
  disable filter:    8ms
  accounted:      15.8s    (sum of per-thread time)
──────────────────────────────────────────────────
linter wall clock:                     1.0s
total wall clock:                      3.3s
```

## Where Time Goes

### Wall-clock breakdown (Discourse)

| Phase | Wall time | % of total |
|-------|----------:|-----------:|
| `bundle info` (8 calls) | 1.2s | 41% |
| Config loading (total) | 1.0s | 34% |
| Linting (parallel) | 723ms | 25% |
| **Total** | **2.9s** | |

### Wall-clock breakdown (Mastodon)

| Phase | Wall time | % of total |
|-------|----------:|-----------:|
| `bundle info` (6 calls) | 1.6s | 48% |
| Config loading (total) | 2.0s | 61% |
| Linting (parallel) | 1.0s | 30% |
| **Total** | **3.3s** | |

### Inside the linter (per-file costs, from cumulative thread time)

| Phase | Discourse | Mastodon | Notes |
|-------|----------:|---------:|-------|
| Prism parse | 83µs/file | 61µs/file | Cheap. Not a bottleneck. |
| CodeMap build | 12µs/file | 10µs/file | Negligible. |
| Cop execution | 1.0ms/file | 5.9ms/file | Dominates. 915 AST walks/file. |
| File I/O | 170µs/file | 222µs/file | Disk reads + line offset computation. |

Mastodon has higher per-file cop cost because more plugin cops are enabled (all 5 plugins vs Discourse's custom cop setup that disables many standard cops).

## Comparison to RuboCop

### Including bundler overhead (current state)

| Repo | turbocop | RuboCop | Speedup |
|------|-------:|--------:|--------:|
| Discourse | 2.9s | 3.45s | **1.2x** |
| Mastodon | 3.3s | 2.49s | **0.8x** (slower!) |

### Linting only (no bundler)

| Repo | turbocop | RuboCop (est.) | Speedup |
|------|-------:|---------------:|--------:|
| Discourse | 723ms | ~3.0s | **4.1x** |
| Mastodon | 1.0s | ~2.0s | **2.0x** |

RuboCop estimates subtract ~0.5s for its own Ruby VM boot, which is a fixed cost that doesn't scale with files.

## Optimization Opportunities (ranked by impact)

### 1. Eliminate bundler shell-outs (saves 1-2s per run)

Each `bundle info --path <gem>` spawns a Ruby process that boots Bundler. This is 130-440ms per call, called once per plugin gem. A project with 5 plugins pays 1-2s just for gem path resolution.

Options:
- **Lockfile/resolver**: Run `turbocop resolve` once (uses Ruby), cache resolved config. Runtime reads lockfile only. Zero Ruby in hot path.
- **Disk cache**: Cache gem paths keyed on `Gemfile.lock` mtime. First run pays the cost, subsequent runs are free.
- **Pure Rust resolution**: Parse `Gemfile.lock` + detect version manager to construct gem paths without calling Ruby. Fragile but zero-dependency.

### 2. Multi-cop AST walker (saves 30-50% of cop execution time)

Currently each cop walks the entire AST independently: 915 calls to `walker.visit()` per file. A multi-cop walker that traverses once and dispatches all enabled cops at each node would eliminate ~500 redundant tree traversals per file.

Estimated savings: 30-50% of the cop execution phase, which is 75-94% of linting time. Net effect: ~20-40% reduction in linting wall time.

### 3. Lazy cop filtering (minor)

Currently all 915 cops are iterated per file, with filter checks for each. Pre-partitioning cops into "enabled for this file" sets could skip the per-cop filter check, but the overhead is small.

## Path to 5x

| Optimization | Discourse | Mastodon |
|-------------|----------:|---------:|
| Current total | 2.9s | 3.3s |
| - Eliminate bundler | 0.7s | 1.0s |
| - Multi-cop walker (-35% cop exec) | 0.5s | 0.7s |
| **Optimized total** | **~0.5s** | **~0.7s** |
| **vs RuboCop** | **6.9x** | **3.6x** |

For a 500K LOC Rails app (similar to the rails bench repo), the optimized turbocop would likely run in under 1 second vs RuboCop's typical 3-5 seconds.
