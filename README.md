# NitroCop

Fast Ruby linter in Rust targeting RuboCop compatibility.

> [!NOTE]
> 🚧 Early-stage: Detection is high-fidelity on most codebases but edge cases remain. Autocorrect is not yet complete. Expect bugs.

Benchmark on the [rubygems.org repo](https://github.com/rubygems/rubygems.org) (1,227 files, Ruby 4.0), Apple Silicon:

| Scenario | nitrocop | RuboCop | Speedup |
|----------|-------:|--------:|--------:|
| Local dev (50 files changed) | **75ms** | 1.30s | **17.3x** |
| CI (no cache) | **279ms** | 14.86s | **53.4x** |

**Features**

- **915 cops** from 7 RuboCop gems (rubocop, rubocop-rails, rubocop-performance, rubocop-rspec, rubocop-rspec_rails, rubocop-factory_bot, rubocop-rake)
- **99.7% conformance** against RuboCop across [**5,590 open-source repos**](docs/corpus.md)
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

<!-- corpus-cops:start -->
nitrocop supports 915 cops from 7 RuboCop gems.

Compared with RuboCop on [**5,590 open-source repos**](docs/corpus.md) (590k Ruby files).

99.7% of compared issue reports matched (28.3M of 28.4M). 847 of 915 cops matched exactly; 68 differed.

**[rubocop](https://github.com/rubocop/rubocop)** `1.84.2` (588 cops)

| Department | Cops | Matched exactly | Differed | Matched exactly % |
|------------|-----:|----------------:|---------:|------------------:|
| Layout | 100 | 89 | 11 | 89.0% |
| Lint | 148 | 143 | 5 | 96.6% |
| Style | 287 | 260 | 27 | 90.5% |
| Metrics | 10 | 10 | 0 | ✓ 100.0% |
| Naming | 19 | 19 | 0 | ✓ 100.0% |
| Security | 6 | 6 | 0 | ✓ 100.0% |
| Bundler | 7 | 7 | 0 | ✓ 100.0% |
| Gemspec | 10 | 10 | 0 | ✓ 100.0% |
| Migration | 1 | 1 | 0 | ✓ 100.0% |
| **Total** | **588** | **545** | **43** | **92.6%** |

**[rubocop-rails](https://github.com/rubocop/rubocop-rails)** `2.34.3` (138 cops)

| Department | Cops | Matched exactly | Differed | Matched exactly % |
|------------|-----:|----------------:|---------:|------------------:|
| Rails | 138 | 118 | 20 | 85.5% |

**[rubocop-performance](https://github.com/rubocop/rubocop-performance)** `1.26.1` (52 cops)

| Department | Cops | Matched exactly | Differed | Matched exactly % |
|------------|-----:|----------------:|---------:|------------------:|
| Performance | 52 | 52 | 0 | ✓ 100.0% |

**[rubocop-rspec](https://github.com/rubocop/rubocop-rspec)** `3.9.0` (113 cops)

| Department | Cops | Matched exactly | Differed | Matched exactly % |
|------------|-----:|----------------:|---------:|------------------:|
| RSpec | 113 | 113 | 0 | ✓ 100.0% |

**[rubocop-rspec_rails](https://github.com/rubocop/rubocop-rspec_rails)** `2.32.0` (8 cops)

| Department | Cops | Matched exactly | Differed | Matched exactly % |
|------------|-----:|----------------:|---------:|------------------:|
| RSpecRails | 8 | 8 | 0 | ✓ 100.0% |

**[rubocop-factory_bot](https://github.com/rubocop/rubocop-factory_bot)** `2.28.0` (11 cops)

| Department | Cops | Matched exactly | Differed | Matched exactly % |
|------------|-----:|----------------:|---------:|------------------:|
| FactoryBot | 11 | 11 | 0 | ✓ 100.0% |

**[rubocop-rake](https://github.com/rubocop/rubocop-rake)** `0.7.1` (5 cops)

| Department | Cops | Matched exactly | Differed | Matched exactly % |
|------------|-----:|----------------:|---------:|------------------:|
| Rake | 5 | 0 | 5 | 0.0% |

"Matched exactly" means nitrocop produced no extra issues and missed no issues for that cop anywhere in the corpus.
See [docs/corpus.md](docs/corpus.md) for the full corpus breakdown.
<!-- corpus-cops:end -->

Every cop reads its RuboCop YAML config options and has fixture-based test coverage.

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

## Limitations

These cops are registered but cannot be exercised under current Ruby versions:

- `Lint/ItWithoutArgumentsInBlock` — `it` is a block parameter in Ruby 3.4+, making this cop obsolete
- `Lint/NonDeterministicRequireOrder` — `Dir` results are sorted since Ruby 3.0
- `Lint/NumberedParameterAssignment` — assigning to `_1` is a syntax error in Ruby 3.4+
- `Lint/UselessElseWithoutRescue` — syntax error in Ruby 3.4+
- `Security/YAMLLoad` — `YAML.load` is safe since Ruby 3.1 (cop has max Ruby 3.0)

These cops are excluded from corpus reporting counts.
