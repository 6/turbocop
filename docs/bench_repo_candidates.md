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

## Not Recommended

### 8. opf/openproject -- No

- **Category**: Massive Rails monorepo (33MB Ruby)
- **Why**: Custom cop gem (`rubocop-openproject`), monorepo structure, `inherit_from` glob patterns, requires loading Ruby initializer. Too many config edge cases for clean conformance.

### 9. reactjs/react-rails -- No

- **Category**: Tiny JS+Ruby gem (144KB Ruby)
- **Why**: Too small. Uses ERB in .rubocop.yml. Unsupported `rubocop-minitest` plugin.

### 10. carrierwaveuploader/carrierwave -- No

- **Category**: File upload gem (core rubocop only)
- **Why**: No plugins, mostly disabled cops, targets Ruby 2.5. Core-only already covered by rails/rubocop bench entries.

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
