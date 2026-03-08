# NitroCop

Fast Ruby linter in Rust targeting RuboCop compatibility.

> [!NOTE]
> 🚧 Early-stage: Detection is high-fidelity on most codebases but edge cases remain. Autocorrect is not yet complete. Expect bugs.

Benchmark on the [rubygems.org repo](https://github.com/rubygems/rubygems.org) (1,222 files), Apple Silicon:

| Scenario | nitrocop | RuboCop | Speedup |
|----------|-------:|--------:|--------:|
| Local dev (50 files changed) | **64ms** | 1.39s | **21.7x** |
| CI (no cache) | **207ms** | 18.21s | **87.8x** |

**Features**

- **915 cops** from 6 RuboCop gems (rubocop, rubocop-rails, rubocop-performance, rubocop-rspec, rubocop-rspec_rails, rubocop-factory_bot)
- **95.2% conformance** against RuboCop across [**1,000 open-source repos**](#conformance)
- **Autocorrect** (`-a`/`-A`) is partial — work in progress
- Reads your existing `.rubocop.yml` — no migration needed
- Uses [Prism](https://github.com/ruby/prism) (Ruby's official parser) via `ruby-prism` crate
- Parallel file processing with [rayon](https://github.com/rayon-rs/rayon)

## Quick Start (Work in progress 🚧)

Requires Rust 1.85+ (edition 2024).

```bash
cargo install nitrocop   # not yet published — build from source for now
```

Then run it in your Ruby project:

```bash
nitrocop
```

## Configuration

nitrocop reads `.rubocop.yml` with full support for:

- **`inherit_from`** — local files, recursive
- **`inherit_gem`** — resolves gem paths via `bundle info`
- **`inherit_mode`** — merge/override for arrays
- **Department-level config** — `RSpec:`, `Rails:` Include/Exclude/Enabled
- **`AllCops`** — `NewCops`, `DisabledByDefault`, `Exclude`, `Include`
- **`Enabled: pending`** tri-state
- **Per-cop options** — `EnforcedStyle`, `Max`, `AllowedMethods`, `AllowedPatterns`, etc.

Config auto-discovery walks up from the target directory to find `.rubocop.yml`.

## Cops

nitrocop supports 915 cops from 6 RuboCop gems:

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

We diff nitrocop against RuboCop on [**1,000 open-source repos**](docs/corpus.md) (232k Ruby files) with all cops enabled. Every offense is compared by file, line, and cop name.

|                        |    Count |  Rate |
|:-----------------------|--------: |------:|
| Agreed                 |    11.4M | 95.3% |
| nitrocop extra (FP)    |    64.1K | 0.5% |
| nitrocop missed (FN)   |   506.1K | 4.2% |

Per-repo results (top 15 by GitHub stars):

| Repo | .rb files | RuboCop offenses | nitrocop extra (FP) | nitrocop missed (FN) | Agreement |
|------|----------:|-----------------:|--------------------:|---------------------:|----------:|
| [rails](https://github.com/rails/rails) | 3,498 | 314,852 | 938 | 17,139 | 94.2% |
| [jekyll](https://github.com/jekyll/jekyll) | 190 | 13,052 | 75 | 737 | 93.8% |
| [mastodon](https://github.com/mastodon/mastodon) | 3,123 | 76,406 | 108 | 2,349 | 96.7% |
| [huginn](https://github.com/huginn/huginn) | 451 | 34,402 | 153 | 961 | 96.7% |
| [discourse](https://github.com/discourse/discourse) | 9,182 | 621,127 | 2,181 | 13,378 | 97.5% |
| [fastlane](https://github.com/fastlane/fastlane) | 1,302 | 118,730 | 198 | 2,994 | 97.3% |
| [devdocs](https://github.com/freeCodeCamp/devdocs) | 833 | 19,903 | 118 | 1,283 | 93.0% |
| [chatwoot](https://github.com/chatwoot/chatwoot) | 2,262 | 64,941 | 59 | 1,479 | 97.6% |
| [vagrant](https://github.com/hashicorp/vagrant) | 1,460 | 86,064 | 215 | 2,946 | 96.3% |
| [devise](https://github.com/heartcombo/devise) | 206 | 5,800 | 20 | 411 | 92.5% |
| [forem](https://github.com/forem/forem) | 3,390 | 128,530 | 278 | 4,297 | 96.4% |
| [postal](https://github.com/postalserver/postal) | 294 | 13,948 | 24 | 709 | 94.7% |
| [CocoaPods](https://github.com/CocoaPods/CocoaPods) | 438 | 28,422 | 360 | 1,945 | 91.9% |
| [openproject](https://github.com/opf/openproject) | 9,286 | 388,980 | 1,205 | 10,287 | 97.0% |
| [gollum](https://github.com/gollum/gollum) | 55 | 3,790 | 21 | 295 | 91.7% |

Remaining gaps are mostly in complex layout cops (indentation, alignment) and a few style cops. See [docs/corpus.md](docs/corpus.md) for the full corpus breakdown.

## Hybrid Mode

Use `--rubocop-only` to run nitrocop alongside RuboCop for cops it doesn't cover yet:

```bash
#!/usr/bin/env bash
# bin/lint — fast hybrid linter
nitrocop "$@"

REMAINING=$(nitrocop --rubocop-only)
if [ -n "$REMAINING" ]; then
  bundle exec rubocop --only "$REMAINING" "$@"
fi
```

## CLI

```
nitrocop [OPTIONS] [PATHS]...

Arguments:
  [PATHS]...    Files or directories to lint [default: .]

Options:
  -a, --autocorrect         Autocorrect offenses (safe cops only)
  -A, --autocorrect-all     Autocorrect offenses (all cops, including unsafe)
  -c, --config <PATH>       Path to .rubocop.yml
  -f, --format <FORMAT>     Output format: text, json [default: text]
      --only <COPS>         Run only specified cops (comma-separated)
      --except <COPS>       Skip specified cops (comma-separated)
      --rubocop-only        Print cops NOT covered by nitrocop
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
cargo run --release --bin bench_nitrocop          # full: setup + bench + conform
cargo run --release --bin bench_nitrocop -- bench # timing only
```

## How It Works

1. **Config resolution** — Walks up from target to find `.rubocop.yml`, resolves `inherit_from`/`inherit_gem` chains, merges layers
2. **File discovery** — Uses the `ignore` crate for .gitignore-aware traversal, applies AllCops.Exclude/Include patterns
3. **Parallel linting** — Each rayon worker thread parses files with Prism (`ParseResult` is `!Send`), runs all enabled cops per file
4. **Cop execution** — Three check phases per file: `check_lines` (raw text), `check_source` (bytes + CodeMap), `check_node` (AST walk via batched dispatch table)
5. **Output** — RuboCop-compatible text format or JSON
