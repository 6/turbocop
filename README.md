# rblint

A fast Ruby linter written in Rust, targeting drop-in [RuboCop](https://rubocop.org/) compatibility.

- **364 cops** across 8 departments (Layout, Lint, Metrics, Naming, Performance, Rails, RSpec, Style)
- **2-3x faster** than RuboCop on real-world codebases
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

| Repo | .rb files | rblint | RuboCop | Speedup |
|------|----------:|-------:|--------:|--------:|
| [Mastodon](https://github.com/mastodon/mastodon) | 2,526 | **1.04s** | 2.52s | **2.4x** |
| [Discourse](https://github.com/discourse/discourse) | 5,831 | **3.69s** | 4.27s | **1.2x** |

Run `cargo run --release --bin bench_rblint` to reproduce. See [bench/results.md](bench/results.md) for full details.

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

364 cops organized by department:

| Department | Cops | Examples |
|------------|-----:|---------|
| Layout | 31 | LineLength, IndentationWidth, TrailingWhitespace |
| Lint | 17 | Debugger, DeprecatedClassMethods, SuppressedException |
| Metrics | 8 | MethodLength, AbcSize, CyclomaticComplexity |
| Naming | 8 | MethodName, VariableName, FileName |
| Performance | 39 | FlatMap, Detect, Sum, StartWith |
| Rails | 98 | TimeZone, FindBy, HttpStatus, Validation |
| RSpec | 113 | Focus, DescribedClass, ExampleLength, PredicateMatcher |
| Style | 50 | FrozenStringLiteralComment, HashSyntax, StringLiterals |

Every cop reads its RuboCop YAML config options and has fixture-based test coverage.

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
  -h, --help                Print help
```

## Development

```bash
cargo check          # fast compile check
cargo test           # run all tests (1,300+)
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
  cop/           Cop trait + 364 cop implementations
    layout/        31 cops (whitespace, indentation, alignment)
    lint/          17 cops (potential bugs, deprecated methods)
    metrics/        8 cops (complexity, length)
    naming/         8 cops (naming conventions)
    performance/   39 cops (Ruby performance anti-patterns)
    rails/         98 cops (Rails best practices)
    rspec/        113 cops (RSpec best practices)
    style/         50 cops (code style)
    util.rs        Shared helpers (constant resolution, method chains, RSpec DSL)
    registry.rs    CopRegistry (all 364 cops)
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
4. **Cop execution** — Three check phases per file: `check_lines` (raw text), `check_source` (bytes + CodeMap), `check_node` (AST walk)
5. **Output** — RuboCop-compatible text format or JSON

## License

MIT
