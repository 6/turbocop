# Synthetic Corpus

Handcrafted Ruby files that exercise the 55 cops with zero activity in the 1,000-repo corpus oracle. These cops target niche patterns (Rails migrations, specific API usage, edge-case lint rules) that don't appear in real-world repos.

## Usage

```bash
cargo build --release
cd bench/synthetic && bundle install
python3 bench/synthetic/run_synthetic.py           # summary
python3 bench/synthetic/run_synthetic.py --verbose  # per-cop breakdown
```

## How It Works

1. Runs nitrocop and RuboCop on `project/` with the same config
2. Filters offenses to only the 55 target cops
3. Compares offense tuples `(file, line, cop_name)` — same approach as the corpus oracle
4. Reports matches / FP / FN per cop

## Structure

```
bench/synthetic/
  Gemfile              # rubocop + plugins + railties (for gem_requirements checks)
  rubocop.yml          # inherits baseline, overrides TargetRailsVersion to 8.0
  run_synthetic.py     # comparison script → synthetic-results.json
  project/
    .rubocop.yml       # inherits parent (ensures Include patterns match)
    app/               # Rails app files (models, controllers, mailers, jobs)
    db/migrate/        # migration cops
    db/schema.rb       # required by UniqueValidationWithoutIndex, UnusedIgnoredColumns
    lib/               # Lint, Style, Security cops
    spec/              # RSpec cops
    test/              # ActionController test cops
```

## Cops Not Covered (7)

These cops cannot be triggered under Ruby 3.4 / our gem versions:

- `Lint/ItWithoutArgumentsInBlock` — `it` is a valid block param in Ruby 3.4
- `Lint/NonDeterministicRequireOrder` — Ruby 3.0+ sorts Dir results
- `Lint/NumberedParameterAssignment` — `_1 = x` is a syntax error in Ruby 3.4
- `Lint/UselessElseWithoutRescue` — syntax error in Ruby 3.4
- `Security/YAMLLoad` — max Ruby 3.0 (YAML.load is safe in 3.1+)
- `Style/ReverseFind` — requires Ruby 4.0
- `Rails/StrongParametersExpect` — requires railties >= 8.0

## Research Findings

### Rails Cops and `railties` Gem Requirement

RuboCop-Rails 2.34.3 uses a `requires_gem` API for version-gated cops. When a cop declares `minimum_target_rails_version 6.1`, this translates to `requires_gem('railties', '>= 6.1')`. RuboCop checks the **project's `Gemfile.lock`** for the `railties` gem version — not the `TargetRailsVersion` config key.

Without `railties` in the Gemfile.lock, 18+ Rails cops silently disable themselves:
- `Rails/CompactBlank` (>= 6.1), `Rails/IndexWith` (>= 6.0), `Rails/Pick` (>= 6.0)
- `Rails/ApplicationRecord`, `Rails/ApplicationMailer`, `Rails/ApplicationJob` (>= 5.0)
- `Rails/WhereRange` (>= 6.0), `Rails/WhereMissing` (>= 6.1)
- `Rails/EnvLocal` (>= 7.1), `Rails/FreezeTime` (>= 5.2), and many more

The `TargetRailsVersion` config is used by a different, older mechanism. Newer cops use `requires_gem` exclusively.

### Include Pattern Path Matching

Many cops restrict which files they check via `Include` patterns in `config/default.yml`:
- Migration cops: `Include: ['db/**/*.rb']`
- Test-only cops: `Include: ['spec/**/*.rb', 'test/**/*.rb']`
- Controller cops: `Include: ['**/app/controllers/**/*.rb']`

These patterns are resolved **relative to the `.rubocop.yml` location**. Running `rubocop project/db/migrate/foo.rb` from the parent directory won't match `db/**/*.rb`. The fix: place `.rubocop.yml` inside the project directory so relative paths match.

### Ruby Version Gates

Several cops use `minimum_target_ruby_version` or `maximum_target_ruby_version`:
- `Security/YAMLLoad`: `maximum_target_ruby_version 3.0` — YAML.load became safe in Ruby 3.1
- `Style/ReverseFind`: `minimum_target_ruby_version 4.0` — `rfind` doesn't exist until Ruby 4.0
- `Lint/NonDeterministicRequireOrder`: only fires on Ruby 2.7 and below (Dir results sorted since 3.0)

Some patterns are Ruby 3.4 **syntax errors**, not just version-gated:
- `_1 = x` — numbered parameters are reserved words, can't be assigned
- `begin ... else ... end` without rescue — invalid syntax
- `it` in blocks — it's now a valid implicit block parameter (Lint/ItWithoutArgumentsInBlock becomes moot)

### Conflicting Cop Pairs

`Style/RedundantConstantBase` and `Lint/ConstantResolution` are mutually exclusive by design. RedundantConstantBase says "don't use `::Foo` at top level" while ConstantResolution says "always fully qualify constants." The RedundantConstantBase spec explicitly disables ConstantResolution in its test setup.

### Schema-Dependent Cops

`Rails/UniqueValidationWithoutIndex` and `Rails/UnusedIgnoredColumns` require `db/schema.rb` to exist. Without it, they can't check database state and silently skip all files.

### Style/Copyright Config Requirement

`Style/Copyright` requires a `Notice` config key with a regex pattern (e.g., `Notice: 'Copyright (\(c\) )?20\d{2}'`). Without it, the cop emits a warning and exits without checking any files. `AutocorrectNotice` must also be set for autocorrect to work.

### Rails/BulkChangeTable Database Requirement

`Rails/BulkChangeTable` needs to know the database type to determine which operations can be combined. Set `Database: postgresql` or `Database: mysql` in the cop config, or provide a `config/database.yml` file.

## Adding New Cops

When a cop has zero corpus activity, add triggering patterns to the appropriate file in `project/` and add the cop name to `TARGET_COPS` in `run_synthetic.py`. Check the vendor spec (`vendor/rubocop*/spec/`) for the exact patterns that trigger the cop.

Key gotchas:
- Rails cops with `minimum_target_rails_version` also check `railties` in `Gemfile.lock` via `requires_gem`
- Cops with `Include` patterns (e.g., `db/**/*.rb`) need files at the right paths relative to `.rubocop.yml`
- Some cop pairs conflict (e.g., `Style/RedundantConstantBase` vs `Lint/ConstantResolution`)
- Some cops need external state files (`db/schema.rb`, `config/database.yml`)
- Check `minimum_target_ruby_version` / `maximum_target_ruby_version` — some cops are gated
