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
- **93.6% conformance** against RuboCop across **500 open-source repos** (all cops enabled)
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

We run a [corpus oracle](https://github.com/6/turbocop/actions/workflows/corpus-oracle.yml) that diffs turbocop against RuboCop on **500 open-source repos** (163k Ruby files) with all cops enabled. Every offense is compared by file, line, and cop name.

**Overall: 93.6% match rate** across 4.9M offenses compared.

Top 15 repos by GitHub stars:

| Repo | Stars | Offenses | Match |
|------|------:|---------:|------:|
| [rails](https://github.com/rails/rails) | 58k | 144,728 | 91.0% |
| [jekyll](https://github.com/jekyll/jekyll) | 51k | 7,697 | 92.5% |
| [mastodon](https://github.com/mastodon/mastodon) | 50k | 22,390 | 95.6% |
| [huginn](https://github.com/huginn/huginn) | 49k | 18,734 | 96.6% |
| [discourse](https://github.com/discourse/discourse) | 46k | 333,429 | 97.3% |
| [fastlane](https://github.com/fastlane/fastlane) | 41k | 62,115 | 95.5% |
| [chatwoot](https://github.com/chatwoot/chatwoot) | 27k | 18,231 | 98.6% |
| [vagrant](https://github.com/hashicorp/vagrant) | 27k | 50,531 | 94.8% |
| [forem](https://github.com/forem/forem) | 23k | 60,564 | 97.7% |
| [openproject](https://github.com/opf/openproject) | 14k | 194,365 | 97.5% |
| [sidekiq](https://github.com/sidekiq/sidekiq) | 14k | 6,104 | 95.7% |
| [rubocop](https://github.com/rubocop/rubocop) | 13k | 17,473 | 96.8% |
| [sinatra](https://github.com/sinatra/sinatra) | 12k | 4,732 | 91.3% |
| [faker](https://github.com/faker-ruby/faker) | 12k | 1,423 | 97.8% |
| [activeadmin](https://github.com/activeadmin/activeadmin) | 10k | 8,077 | 96.5% |

Remaining gaps are mostly in complex layout cops (indentation, alignment) and a few style cops. See [docs/coverage.md](docs/coverage.md) for the full breakdown.

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
