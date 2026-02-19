# rblint

A fast Ruby linter written in Rust, targeting drop-in [RuboCop](https://rubocop.org/) compatibility.

- **915 cops** across 8 departments (Layout, Lint, Metrics, Naming, Performance, Rails, RSpec, Style)
- **1.2-26x faster** than RuboCop on real-world codebases (without result caching)
- Reads your existing `.rubocop.yml` — no migration needed
- Uses [Prism](https://github.com/ruby/prism) (Ruby's official parser) via `ruby-prism` crate
- Parallel file processing with [rayon](https://github.com/rayon-rs/rayon)

## Quick Start

```bash
# Build
cargo build --release

# Lint current directory (reads .rubocop.yml automatically)
cargo run --release -- .

# JSON output
cargo run --release -- --format json .

# Lint via stdin (for editor integration)
echo 'x = 1 ' | cargo run --release -- --stdin test.rb
```

## Installation

Requires Rust 1.85+ (edition 2024).

```bash
git clone https://github.com/peterb/rblint.git
cd rblint
git submodule update --init    # fetch vendor/rubocop for tests
cargo build --release
```

The binary is at `target/release/rblint`.

## Benchmarks

Measured on Apple Silicon, median of 5 runs via [hyperfine](https://github.com/sharkdp/hyperfine).
rblint has no result cache; RuboCop uses its built-in file cache (warm after warmup runs).

| Repo | .rb files | rblint (no cache) | RuboCop (cached) | Speedup |
|------|----------:|------------------:|-----------------:|--------:|
| [mastodon](https://github.com/mastodon/mastodon) | 2,526 | **482ms** | 2.30s | **4.8x** |
| [discourse](https://github.com/discourse/discourse) | 5,831 | **565ms** | 3.46s | **6.1x** |
| [rails](https://github.com/rails/rails) | 3,332 | **377ms** | 5.99s | **15.9x** |
| [rubocop](https://github.com/rubocop/rubocop) | 1,665 | **1.02s** | 1.19s | **1.2x** |
| [chatwoot](https://github.com/chatwoot/chatwoot) | 1,900 | **584ms** | 2.67s | **4.6x** |
| [errbit](https://github.com/errbit/errbit) | 207 | **80ms** | 1.20s | **14.9x** |
| [activeadmin](https://github.com/activeadmin/activeadmin) | 354 | **42ms** | 1.10s | **26.0x** |

Run `cargo run --release --bin bench_rblint` to reproduce. See [bench/results.md](bench/results.md) for full details across 12 repos.

## Configuration

rblint reads `.rubocop.yml` with full support for:

- **`inherit_from`** — local files, recursive
- **`inherit_gem`** — resolves gem paths via `bundle info`
- **`inherit_mode`** — merge/override for arrays
- **Department-level config** — `RSpec:`, `Rails:` Include/Exclude/Enabled
- **`AllCops`** — `NewCops`, `DisabledByDefault`, `Exclude`, `Include`
- **`Enabled: pending`** tri-state
- **Per-cop options** — `EnforcedStyle`, `Max`, `AllowedMethods`, `AllowedPatterns`, etc.

Config auto-discovery walks up from the target directory to find `.rubocop.yml`.

## Cops

915 cops organized by department:

| Department | Cops | Examples |
|------------|-----:|---------|
| Style | 289 | FrozenStringLiteralComment, HashSyntax, StringLiterals |
| Lint | 152 | Debugger, DeprecatedClassMethods, SuppressedException |
| Rails | 136 | TimeZone, FindBy, HttpStatus, Validation |
| RSpec | 113 | Focus, DescribedClass, ExampleLength, PredicateMatcher |
| Layout | 100 | LineLength, IndentationWidth, TrailingWhitespace |
| Performance | 52 | FlatMap, Detect, Sum, StartWith |
| Naming | 19 | MethodName, VariableName, FileName |
| FactoryBot | 11 | CreateList, ConsistentParenthesesStyle |
| Metrics | 10 | MethodLength, AbcSize, CyclomaticComplexity |
| Gemspec | 10 | RequiredRubyVersion, DuplicatedAssignment |
| RSpecRails | 8 | HttpStatus, InferredSpecType |
| Security | 7 | Eval, Open, YAMLLoad |
| Bundler | 7 | OrderedGems, GemComment, DuplicatedGem |
| Migration | 1 | DepartmentName |

Every cop reads its RuboCop YAML config options and has fixture-based test coverage.

## Conformance

Location-level conformance against RuboCop on benchmark repos:

| Repo | Match rate |
|------|----------:|
| rails | 100.0% |
| rubocop | 100.0% |
| chatwoot | 100.0% |
| errbit | 100.0% |
| activeadmin | 100.0% |
| good_job | 100.0% |
| rubygems.org | 100.0% |
| mastodon | 95.4% |
| docuseal | 93.5% |

See [bench/results.md](bench/results.md) for per-cop divergence details.

## Hybrid Mode

Use `--rubocop-only` to run rblint alongside RuboCop for cops it doesn't cover yet:

```bash
#!/usr/bin/env bash
# bin/lint — fast hybrid linter
rblint "$@"

REMAINING=$(rblint --rubocop-only)
if [ -n "$REMAINING" ]; then
  bundle exec rubocop --only "$REMAINING" "$@"
fi
```

## CLI

```
rblint [OPTIONS] [PATHS]...

Arguments:
  [PATHS]...    Files or directories to lint [default: .]

Options:
  -c, --config <PATH>       Path to .rubocop.yml
  -f, --format <FORMAT>     Output format: text, json [default: text]
      --only <COPS>         Run only specified cops (comma-separated)
      --except <COPS>       Skip specified cops (comma-separated)
      --rubocop-only        Print cops NOT covered by rblint
      --stdin <PATH>        Read source from stdin, use PATH for display
      --debug               Print timing and debug info
      --list-cops           List all registered cops
  -h, --help                Print help
```

## Development

```bash
cargo check          # fast compile check
cargo test           # run all tests (2,600+)
cargo run -- .       # lint current directory

# Quality checks (must pass — zero tolerance)
cargo test config_audit     # all YAML config keys implemented
cargo test prism_pitfalls   # no missing node type handling

# Benchmarks
cargo run --release --bin bench_rblint          # full: setup + bench + conform
cargo run --release --bin bench_rblint -- bench # timing only
```

## Architecture

```
src/
  cop/           Cop trait + 915 cop implementations
    style/        289 cops (code style)
    lint/         152 cops (potential bugs, deprecated methods)
    rails/        136 cops (Rails best practices)
    rspec/        113 cops (RSpec best practices)
    layout/       100 cops (whitespace, indentation, alignment)
    performance/   52 cops (Ruby performance anti-patterns)
    naming/        19 cops (naming conventions)
    ...and 54 more across FactoryBot, Metrics, Gemspec, RSpecRails, Security, Bundler, Migration
    util.rs        Shared helpers (constant resolution, method chains, RSpec DSL)
    registry.rs    CopRegistry (all 915 cops)
  config/        .rubocop.yml loading, inheritance, gem resolution
  parse/         Prism parser wrapper, SourceFile, CodeMap
  formatter/     Text (RuboCop-compatible) and JSON output
  linter.rs      Parallel orchestration (rayon)
  cli.rs         Clap CLI
testdata/        Fixture files (offense.rb + no_offense.rb per cop)
tests/           Integration tests (config_audit, prism_pitfalls, conformance)
bench/           Benchmark harness + results
vendor/          RuboCop + plugin submodules (reference configs/specs)
```

## How It Works

1. **Config resolution** — Walks up from target to find `.rubocop.yml`, resolves `inherit_from`/`inherit_gem` chains, merges layers
2. **File discovery** — Uses the `ignore` crate for .gitignore-aware traversal, applies AllCops.Exclude/Include patterns
3. **Parallel linting** — Each rayon worker thread parses files with Prism (`ParseResult` is `!Send`), runs all enabled cops per file
4. **Cop execution** — Three check phases per file: `check_lines` (raw text), `check_source` (bytes + CodeMap), `check_node` (AST walk via batched dispatch table)
5. **Output** — RuboCop-compatible text format or JSON

## License

MIT
