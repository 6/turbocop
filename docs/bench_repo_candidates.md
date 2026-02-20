# Bench Repo Candidates

Evaluated 2026-02-16. Criteria: RuboCop usage, plugin diversity, codebase size, config complexity, and what it adds beyond our current bench set.

## Current Bench Set

| Repo | Category | Plugins | Conformance |
|------|----------|---------|-------------|
| mastodon | Large Rails app | rails, rspec, performance | 100% |
| discourse | Large Rails app | core only | 100% |
| rails | Framework | core only | 100% |
| rubocop | Ruby tool (RSpec-heavy) | core only | WIP |
| chatwoot | Large Rails app | rails, rspec | WIP |
| errbit | Small Rails app | core only | 81.9% |

## Recommended Additions

### 1. rubygems/rubygems.org -- YES

- **Category**: Medium Rails app (community infrastructure)
- **Ruby size**: ~2.4MB (~800-1200 files)
- **Plugins**: rubocop-performance, rubocop-rails, **rubocop-minitest**, rubocop-capybara, rubocop-factory_bot
- **Config**: `inherit_from: .rubocop_todo.yml`, `ParserEngine: parser_prism`, `!ruby/regexp` in Exclude, `NewCops: enable`
- **Why**: First **minitest**-based repo (vs all-RSpec current set). Well-known Ruby community project. Exercises `inherit_from` with todo. The `!ruby/regexp` YAML edge case is worth testing.

### 2. activeadmin/activeadmin -- YES

- **Category**: Ruby gem/Rails engine (admin framework)
- **Ruby size**: ~676KB (~200-400 files)
- **Plugins**: rubocop-capybara, rubocop-packaging, rubocop-performance, rubocop-rails, rubocop-rspec
- **Config**: `DisabledByDefault: true` with explicit per-cop enablement. Uses `plugins:` key (new RuboCop 1.72+ style). `inherit_mode: merge: [Include]`.
- **Why**: First **gem/engine** (non-application). Tests `DisabledByDefault: true` config mode. Tests the modern `plugins:` config key. Introduces `rubocop-packaging`.

### 3. bensheldon/good_job -- YES

- **Category**: Ruby gem (Active Job backend)
- **Ruby size**: ~641KB (~150-250 files)
- **Plugins**: rubocop-capybara, rubocop-performance, rubocop-rails, rubocop-rspec, rubocop-rspec_rails
- **Config**: Uses `plugins:` syntax. `inherit_mode: merge:`. `NewCops: enable`. Disables entire `Metrics` department.
- **Why**: Tests `plugins:` config path (modern RuboCop). Exercises 5 plugins including split gems (capybara, rspec_rails). Clean codebase (tiny todo).

### 4. docusealco/docuseal -- YES

- **Category**: Large modern Rails app (document signing)
- **Ruby size**: ~925KB (~300-500 files)
- **Plugins**: rubocop-performance, rubocop-rails, rubocop-rspec
- **Config**: `TargetRubyVersion: '4.0'`, `NewCops: enable`. High metric thresholds. Clean, minimal config.
- **Why**: Popular, actively maintained modern Rails app. Standard plugin set exercises our existing cops broadly — serves as a validation target to confirm conformance at scale. Ruby 4.0 target tests bleeding-edge version handling.

### 5. doorkeeper-gem/doorkeeper -- YES

- **Category**: OAuth provider gem
- **Ruby size**: ~715KB (small-medium)
- **Plugins**: rubocop-performance, rubocop-rails, rubocop-rspec
- **Config**: `inherit_from: .rubocop_todo.yml`. Standard config with double_quotes, trailing commas.
- **Why**: Standalone Ruby gem that uses all three standard plugins (rails, rspec, performance) — tests plugin version detection on a gem that depends on rubocop-rails despite not being a Rails app itself. Clean, well-maintained config.

## Maybe / Lower Priority

### 6. fatfreecrm/fat_free_crm -- MAYBE

- **Category**: Medium Rails CRM app
- **Ruby size**: ~1.2MB (~300-500 files)
- **Plugins**: rubocop-capybara, rubocop-factory_bot, rubocop-rails, rubocop-rspec, rubocop-rspec_rails
- **Config**: `plugins:` syntax. **Massive rubocop_todo** (1443 lines, 47KB).
- **Why**: Stress-tests todo processing. Introduces `rubocop-factory_bot`. But overlaps with existing Rails apps.

### 7. connorshea/vglist -- MAYBE (lean No)

- **Category**: Medium Rails app (video game list)
- **Plugins**: 6 plugins including rubocop-rspec_rails, rubocop-factory_bot
- **Why**: Another Rails app (already have 4). Verbose config but low diversity value.

### 8. jfelchner/ruby-progressbar -- MAYBE

- **Category**: Tiny Ruby gem (progress bar library)
- **Ruby size**: ~151KB (~40-60 files)
- **Plugins**: rubocop-rspec, rubocop-performance, rubocop-capybara, rubocop-factory_bot, **rubocop-thread_safety**
- **Config**: 7 config files (`.rubocop_core.yml`, `_rspec.yml`, `_performance.yml`, etc.) via `rspectacular` gem. Exhaustive per-cop configuration (~79KB core config). Explicit enable/disable for nearly every cop.
- **Why**: Introduces **rubocop-thread_safety** (unique plugin not in any other candidate). Most exhaustive config of any candidate — tests per-cop config handling at scale. But: tiny codebase, config comes from external `rspectacular` gem which may complicate loading.

### 9. sferik/multi_json -- MAYBE

- **Category**: Small Ruby gem (JSON backend abstraction)
- **Ruby size**: ~117 files (~736KB repo)
- **Plugins**: rubocop-minitest, rubocop-performance, **rubocop-rake**, standard-performance
- **Config**: Hybrid `require: standard` inside `.rubocop.yml` with RuboCop overrides on top. Also has `.standard.yml`. `NewCops: enable`. TargetRubyVersion 3.2. Many Layout/Style overrides (double quotes, fixed indentation, 140-char lines).
- **Why**: Introduces **rubocop-rake** (unique plugin). Tests the hybrid standardrb + RuboCop override pattern — exercises turbocop's standard-family gem config resolution with custom overrides layered on top. But: rubocop-rake has very few cops.

### 10. bkeepers/dotenv -- MAYBE

- **Category**: Small Ruby gem (env variable loader)
- **Ruby size**: 29 files
- **Plugins**: None (standardrb defaults only)
- **Config**: Uses **standardrb** exclusively (`.standard.yml` only, no `.rubocop.yml`). `ruby_version: 3.0`, one cop ignore (`Lint/InheritException`).
- **Why**: Tests turbocop's standardrb support in isolation — pure `.standard.yml` with no `.rubocop.yml` overlay. But: only 29 files, minimal config diversity.

## Not Recommended

### 11. opf/openproject -- No

- **Category**: Massive Rails monorepo (33MB Ruby)
- **Why**: Custom cop gem (`rubocop-openproject`), monorepo structure, `inherit_from` glob patterns, requires loading Ruby initializer. Too many config edge cases for clean conformance.

### 12. reactjs/react-rails -- No

- **Category**: Tiny JS+Ruby gem (144KB Ruby)
- **Why**: Too small. Uses ERB in .rubocop.yml. Unsupported `rubocop-minitest` plugin.

### 13. carrierwaveuploader/carrierwave -- No

- **Category**: File upload gem (core rubocop only)
- **Why**: No plugins, mostly disabled cops, targets Ruby 2.5. Core-only already covered by rails/rubocop bench entries.

### 14. rspec/rspec-rails -- No

- **Category**: Ruby gem (RSpec integration for Rails)
- **Ruby size**: ~475KB (~150-250 files)
- **Plugins**: None (core RuboCop only)
- **Config**: Inherits from shared `rspec-dev` base config (`.rubocop_rspec_base.yml`) that disables dozens of Style, Layout, and Naming cops. Pinned to RuboCop 1.28.2. Has stale `.rubocop_todo.yml` (from 2022). TargetRubyVersion 3.1.
- **Why**: Core-only (no plugins) — already well-covered by rails, discourse, rubocop in current bench set. Pinned to old RuboCop version. The rspec-dev base config disables so many cops that relatively few actually run.

### 15. weppos/publicsuffix-ruby -- No

- **Category**: Small Ruby gem (domain name parser)
- **Ruby size**: ~26 files
- **Plugins**: rubocop-minitest, rubocop-rake
- **Config**: `plugins:` key. Layered `inherit_from` (`.rubocop_todo.yml` + `.rubocop_opinionated.yml`). Disables all Metrics cops. TargetRubyVersion 3.0.
- **Why**: Only 26 Ruby files — too small to justify. `plugins:` syntax already covered by activeadmin/good_job. Both plugins (minitest, rake) already introduced by multi_json above.

### 16. jwt/ruby-jwt -- No

- **Category**: Small Ruby gem (JWT implementation)
- **Ruby size**: ~258KB (~60-100 files)
- **Plugins**: None (core RuboCop only)
- **Config**: Minimal (~20 lines). TargetRubyVersion 2.5. `NewCops: enable`. Disables `Layout/LineLength`.
- **Why**: Core-only, minimal config, old Ruby target. No plugin diversity. Core-only already covered.

### 17. ruby-concurrency/concurrent-ruby -- No

- **Category**: Large Ruby gem (concurrency primitives)
- **Ruby size**: ~345 files (11MB repo)
- **Plugins**: N/A
- **Config**: **No linting configuration at all** — no `.rubocop.yml`, no `.standard.yml`, no linting gems.
- **Why**: Cannot benchmark conformance without a RuboCop baseline to compare against.

## What the Top 5 Add

| Gap in Current Set | Filled By |
|--------------------|-----------|
| No minitest-based repo | rubygems.org |
| No gem/engine repos | activeadmin, good_job, doorkeeper |
| No `plugins:` config syntax | activeadmin, good_job |
| No `DisabledByDefault: true` | activeadmin |
| No `rubocop-packaging` plugin | activeadmin |
| No `rubocop-minitest` plugin | rubygems.org |
| No `inherit_mode: merge` testing | good_job |
| No `ParserEngine: parser_prism` | rubygems.org |
| No Ruby 4.0 target | docuseal |
| No gem-with-rails-plugin validation | doorkeeper |
| Broader standard-cop validation | docuseal |
