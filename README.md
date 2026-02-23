# turbocop

Experimental RuboCop rewrite in Rust. 900+ cops.

> [!NOTE]
> 🚧 Early-stage: Detection is high-fidelity on most codebases but edge cases remain. Autocorrect is not yet complete. Expect bugs.

Benchmark on the [rubygems.org repo](https://github.com/rubygems/rubygems.org) (1,222 files), Apple Silicon:

| Scenario | turbocop | RuboCop | Speedup |
|----------|-------:|--------:|--------:|
| Local dev (50 files changed) | **64ms** | 1.39s | **21.7x** |
| CI (no cache) | **207ms** | 18.21s | **87.8x** |

**Features**

- **915 cops** from 6 RuboCop gems (rubocop, rubocop-rails, rubocop-performance, rubocop-rspec, rubocop-rspec_rails, rubocop-factory_bot)
- **High conformance** against RuboCop — 14 of 14 benchmark repos at 100%
- **Autocorrect** (`-a`/`-A`) is partial — work in progress
- Reads your existing `.rubocop.yml` — no migration needed
- Uses [Prism](https://github.com/ruby/prism) (Ruby's official parser) via `ruby-prism` crate
- Parallel file processing with [rayon](https://github.com/rayon-rs/rayon)

## Quick Start (Work in progress 🚧)

Requires Rust 1.85+ (edition 2024).

```bash
cargo install turbocop   # not yet published — build from source for now
```

Then run it in your Ruby project:

```bash
turbocop
```

## Configuration

turbocop reads `.rubocop.yml` with full support for:

- **`inherit_from`** — local files, recursive
- **`inherit_gem`** — resolves gem paths via `bundle info`
- **`inherit_mode`** — merge/override for arrays
- **Department-level config** — `RSpec:`, `Rails:` Include/Exclude/Enabled
- **`AllCops`** — `NewCops`, `DisabledByDefault`, `Exclude`, `Include`
- **`Enabled: pending`** tri-state
- **Per-cop options** — `EnforcedStyle`, `Max`, `AllowedMethods`, `AllowedPatterns`, etc.

Config auto-discovery walks up from the target directory to find `.rubocop.yml`.

## Cops

turbocop supports 915 cops from 6 RuboCop gems:

| Gem | Version | Cops | Coverage | Departments |
|-----|---------|-----:|---------:|-------------|
| [rubocop](https://github.com/rubocop/rubocop) | 1.84.2 | 593 | 100% | Layout, Lint, Style, Metrics, Naming, Security, Bundler, Gemspec, Migration |
| [rubocop-rails](https://github.com/rubocop/rubocop-rails) | 2.34.3 | 138 | 100% | Rails |
| [rubocop-performance](https://github.com/rubocop/rubocop-performance) | 1.26.1 | 52 | 100% | Performance |
| [rubocop-rspec](https://github.com/rubocop/rubocop-rspec) | 3.9.0 | 113 | 100% | RSpec |
| [rubocop-rspec_rails](https://github.com/rubocop/rubocop-rspec_rails) | 2.32.0 | 8 | 100% | RSpecRails |
| [rubocop-factory_bot](https://github.com/rubocop/rubocop-factory_bot) | 2.28.0 | 11 | 100% | FactoryBot |

Every cop reads its RuboCop YAML config options and has fixture-based test coverage.

## Conformance

We run both turbocop and RuboCop on 14 popular open source repos and compare every offense by file, line, and cop name. Match rate is the percentage of offenses that both tools agree on:

| Repo | .rb files | Offenses | Match | turbocop | RuboCop |
|------|----------:|-------:|------:|---------:|--------:|
| [mastodon](https://github.com/mastodon/mastodon) | 2,526 | 302 | 100% | **152ms** | 2,390ms |
| [discourse](https://github.com/discourse/discourse) | 5,831 | 0 | 100% | **499ms** | 3,500ms |
| [rails](https://github.com/rails/rails) | 3,332 | 6 | 100% | **157ms** | 6,040ms |
| [rubocop](https://github.com/rubocop/rubocop) | 1,665 | 0 | 100% | **316ms** | 1,190ms |
| [chatwoot](https://github.com/chatwoot/chatwoot) | 1,900 | 251 | 100% | **218ms** | 2,550ms |
| [errbit](https://github.com/errbit/errbit) | 207 | 1,579 | 100% | **51ms** | 1,190ms |
| [activeadmin](https://github.com/activeadmin/activeadmin) | 354 | 3 | 100% | **35ms** | 1,090ms |
| [good_job](https://github.com/bensheldon/good_job) | 242 | 37 | 100% | **51ms** | 1,170ms |
| [docuseal](https://github.com/docusealco/docuseal) | 406 | 60 | 100% | **50ms** | 1,190ms |
| [rubygems.org](https://github.com/rubygems/rubygems.org) | 1,222 | 3 | 100% | **56ms** | 1,390ms |
| [doorkeeper](https://github.com/doorkeeper-gem/doorkeeper) | 246 | 623 | 100% | **48ms** | 1,090ms |
| [fat_free_crm](https://github.com/fatfreecrm/fat_free_crm) | 464 | 32 | 100% | **55ms** | 1,220ms |
| [multi_json](https://github.com/sferik/multi_json) | 116 | 2 | 100% | **21ms** | 867ms |
| [lobsters](https://github.com/lobsters/lobsters) | 450 | 6 | 100% | **36ms** | 1,150ms |

Times are with warm cache, 10% of files invalidated (capped at 50), Apple Silicon. See [bench/results.md](bench/results.md) for full details.

## Hybrid Mode

Use `--rubocop-only` to run turbocop alongside RuboCop for cops it doesn't cover yet:

```bash
#!/usr/bin/env bash
# bin/lint — fast hybrid linter
turbocop "$@"

REMAINING=$(turbocop --rubocop-only)
if [ -n "$REMAINING" ]; then
  bundle exec rubocop --only "$REMAINING" "$@"
fi
```

## CLI

```
turbocop [OPTIONS] [PATHS]...

Arguments:
  [PATHS]...    Files or directories to lint [default: .]

Options:
  -a, --autocorrect         Autocorrect offenses (safe cops only)
  -A, --autocorrect-all     Autocorrect offenses (all cops, including unsafe)
  -c, --config <PATH>       Path to .rubocop.yml
  -f, --format <FORMAT>     Output format: text, json [default: text]
      --only <COPS>         Run only specified cops (comma-separated)
      --except <COPS>       Skip specified cops (comma-separated)
      --rubocop-only        Print cops NOT covered by turbocop
      --stdin <PATH>        Read source from stdin, use PATH for display
      --debug               Print timing and debug info
      --list-cops           List all registered cops
      --ignore-disable-comments  Ignore all # rubocop:disable inline comments
      --cache <true|false>  Enable/disable file-level result caching [default: true]
      --cache-clear         Clear the result cache and exit
      --init                Resolve gem paths and write lockfile to cache directory, then exit
      --fail-level <SEV>    Minimum severity for non-zero exit (convention/warning/error/fatal)
  -F, --fail-fast           Stop after first file with offenses
      --force-exclusion     Apply AllCops.Exclude to explicitly-passed files
  -L, --list-target-files   Print files that would be linted, then exit
      --force-default-config  Ignore all config files, use built-in defaults
  -h, --help                Print help
```

## Local Development

```bash
cargo check          # fast compile check
cargo test           # run all tests (2,700+)
cargo run -- .       # lint current directory

# Quality checks (must pass — zero tolerance)
cargo test config_audit     # all YAML config keys implemented
cargo test prism_pitfalls   # no missing node type handling

# Benchmarks
cargo run --release --bin bench_turbocop          # full: setup + bench + conform
cargo run --release --bin bench_turbocop -- bench # timing only
```

## How It Works

1. **Config resolution** — Walks up from target to find `.rubocop.yml`, resolves `inherit_from`/`inherit_gem` chains, merges layers
2. **File discovery** — Uses the `ignore` crate for .gitignore-aware traversal, applies AllCops.Exclude/Include patterns
3. **Parallel linting** — Each rayon worker thread parses files with Prism (`ParseResult` is `!Send`), runs all enabled cops per file
4. **Cop execution** — Three check phases per file: `check_lines` (raw text), `check_source` (bytes + CodeMap), `check_node` (AST walk via batched dispatch table)
5. **Output** — RuboCop-compatible text format or JSON
