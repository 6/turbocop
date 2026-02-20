# Bench Repo Candidates

Updated 2026-02-20. Criteria: RuboCop usage, plugin diversity, codebase size, config complexity, and what it adds beyond our current bench set.

## Current Bench Set (13 repos, all 100% conformance)

| Repo | Category | Plugins | Notes |
|------|----------|---------|-------|
| mastodon | Large Rails app | rails, rspec, rspec_rails, performance, capybara | 302 offenses matched |
| discourse | Large Rails app | discourse (custom) | 0 offenses (clean) |
| rails | Framework | minitest, packaging, performance, rails, md | 6 offenses matched |
| rubocop | Ruby tool | rspec, performance, rake, internal_affairs | 0 offenses (clean) |
| chatwoot | Large Rails app | rails, rspec, performance, factory_bot | 251 offenses matched |
| errbit | Small Rails app | rspec, rspec_rails, performance, rails, capybara, rake, thread_safety, standard | 1,579 offenses matched |
| rubygems.org | Medium Rails app | minitest, performance, rails, capybara, factory_bot | 0 offenses (clean) |
| activeadmin | Gem/Rails engine | rails, rspec, packaging | `DisabledByDefault: true`, `plugins:` key |
| good_job | Ruby gem | rspec, rspec_rails, performance, rails, capybara | `plugins:` key, `inherit_mode: merge` |
| docuseal | Modern Rails app | rspec, performance, rails | `TargetRubyVersion: '4.0'` |
| doorkeeper | OAuth gem | rspec, performance, rails | Gem using rubocop-rails |
| fat_free_crm | Medium Rails CRM | rspec, rspec_rails, rails, capybara, factory_bot | `plugins:` key, massive todo (47KB) |
| multi_json | Small Ruby gem | minitest, performance, rake, standard | Hybrid standardrb + RuboCop |

## Recommended Additions

### 1. danbooru/danbooru -- YES

- **Category**: Large Rails app (image board, top OSS Rails project)
- **Ruby size**: 1,627 files
- **Plugins**: rubocop-rails
- **Config**: Rich `.rubocop.yml` (130+ lines) with many explicit cop configurations. `NewCops: enable`. Separate `test/.rubocop.yml` overlay.
- **Why**: Major OSS Rails app. 20+ models with `validates uniqueness` — ideal for testing `Rails/UniqueValidationWithoutIndex` once schema analysis is built. Uses `db/structure.sql` instead of `db/schema.rb`, which is an important edge case (many large Rails apps use SQL format). Only rubocop-rails plugin — common real-world pattern.

### 2. zammad/zammad -- YES

- **Category**: Large Rails helpdesk app
- **Ruby size**: ~13 MB (large)
- **Plugins**: rubocop-capybara, rubocop-factory_bot, **rubocop-faker**, **rubocop-graphql**, rubocop-performance, rubocop-rails, **rubocop-rake**, rubocop-rspec, rubocop-rspec_rails, rubocop-inflector
- **Config**: `inherit_from` chain (`.dev/rubocop/default.yml`), `inherit_mode: merge`, custom cops
- **Why**: Most plugin-diverse repo we've found — 10 plugins including graphql, faker, and inflector (none in current bench set). Stress-tests plugin version awareness and unsupported-plugin handling. Large codebase.

### 3. lobsters/lobsters -- YES

- **Category**: Small Rails link aggregator
- **Ruby size**: ~850 KB
- **Plugins**: standard-performance, standard-rails (pure standardrb)
- **Config**: `.standard.yml` only, no `.rubocop.yml`. Uses `extend_config` for custom cops.
- **db/schema.rb**: Yes (32 KB)
- **Why**: First **pure standardrb** Rails app in bench set. Tests the standardrb config path end-to-end without any `.rubocop.yml` overlay. Has schema.rb for future schema cop testing.

## Maybe / Lower Priority

### 4. forem/forem (dev.to) -- MAYBE

- **Category**: Large Rails app (developer community)
- **Ruby size**: ~7 MB
- **Plugins**: rubocop-performance, rubocop-rails, rubocop-rspec, rubocop-capybara
- **db/schema.rb**: Yes (90 KB)
- **Why**: Big, well-known Rails app. Has schema.rb with many models. But plugin combo overlaps heavily with mastodon/chatwoot.

### 5. redmine/redmine -- MAYBE

- **Category**: Classic medium-large Rails app (project management)
- **Ruby size**: ~5.5 MB
- **Plugins**: rubocop-performance, rubocop-rails (uses `plugins:` key)
- **Why**: Venerable Rails project. But plugin combo already well-covered.

### 6. solidusio/solidus -- MAYBE

- **Category**: E-commerce framework (multi-gem monorepo)
- **Ruby size**: ~4.3 MB
- **Plugins**: rubocop-performance, rubocop-rails (+ custom migration cop)
- **Config**: "Relaxed.Ruby.Style" — disables many cops. Multi-gem structure (core, backend, api).
- **Why**: Monorepo structure is unique. But relaxed config means few offenses.

### 7. puma/puma -- MAYBE

- **Category**: Ruby web server gem
- **Ruby size**: ~800 KB
- **Plugins**: rubocop-performance (+ custom cops)
- **Config**: `DisabledByDefault: true` with selective enablement
- **Why**: Well-known gem. But `DisabledByDefault` already tested by activeadmin. Small.

## Not Recommended

### hanami/hanami -- No
- Uses `inherit_from` with a **remote HTTPS URL** (interesting edge case but small codebase, no plugins)

### sinatra/sinatra -- No
- Vanilla rubocop, no plugins, small. Nothing new.

### gitlabhq/gitlabhq -- No
- 90 MB Ruby, custom `gitlab-styles` meta-gem, ERB in `.rubocop.yml`. Impractical.

### heartcombo/devise -- No
- Does not use RuboCop.

### spree/spree -- No
- Does not use RuboCop.

### sidekiq (mperham/sidekiq) -- No
- Uses standardrb but minimal config. lobsters is better for standardrb testing.

### huginn/huginn -- No
- Has rubocop in Gemfile but no config file. Unused/abandoned lint setup.

### opf/openproject -- No
- Custom cop gem, monorepo, `inherit_from` glob patterns, requires Ruby initializer. Too many edge cases.

### reactjs/react-rails -- No
- Too small. ERB in `.rubocop.yml`.

### carrierwaveuploader/carrierwave -- No
- No plugins, mostly disabled cops, old Ruby target.

### rspec/rspec-rails -- No
- Core-only, pinned to old RuboCop, rspec-dev base config disables most cops.

### weppos/publicsuffix-ruby -- No
- Only 26 files. Plugins already covered by multi_json.

### jwt/ruby-jwt -- No
- Core-only, minimal config. Nothing new.

### ruby-concurrency/concurrent-ruby -- No
- No linting configuration at all.

### connorshea/vglist -- No
- Another Rails app with no diversity value beyond what we already have.

### jfelchner/ruby-progressbar -- No
- Tiny codebase (40-60 files). Config comes from external `rspectacular` gem.

### bkeepers/dotenv -- No
- Only 29 files. Lobsters is a better pure-standardrb candidate.

## What the Top 3 Additions Would Add

| Gap | Filled By |
|-----|-----------|
| No pure-standardrb Rails app | lobsters |
| No `db/structure.sql` edge case | danbooru |
| No rubocop-graphql plugin | zammad |
| No rubocop-faker plugin | zammad |
| No 10+ plugin stress test | zammad |
| No large Rails app with rails-only plugin | danbooru |
| Minimal `validates uniqueness` testing | danbooru (20+ models) |
