# turbocop

A fast Ruby linter written in Rust, targeting drop-in [RuboCop](https://rubocop.org/) compatibility.

Quick benchmark on the codebase of [rubygems.org](https://github.com/rubygems/rubygems.org) (1,222 Ruby files), Apple Silicon:

| Mode | turbocop | RuboCop | Speedup |
|------|-------:|--------:|--------:|
| Cached (warm) | **81ms** | 1.47s | **18x** |
| No cache | **203ms** | 17.53s | **86x** |

**Features**

- **915 cops** across 14 departments (Layout, Lint, Style, Metrics, Naming, Security, Bundler, Gemspec, Migration, Rails, Performance, RSpec, RSpecRails, FactoryBot)
- **18x faster** than RuboCop (cached), **84x faster** uncached on [rubygems.org](https://github.com/rubygems/rubygems.org) (1,222 files)
- **100% conformance** against RuboCop on 13 benchmark repos
- Reads your existing `.rubocop.yml` — no migration needed
- Uses [Prism](https://github.com/ruby/prism) (Ruby's official parser) via `ruby-prism` crate
- Parallel file processing with [rayon](https://github.com/rayon-rs/rayon)

## Quick Start

Requires Rust 1.85+ (edition 2024).

```bash
cargo install turbocop
```

Then run it in your ruby repo:

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

Location-level conformance against RuboCop on 13 benchmark repos:

| Repo | Match rate |
|------|----------:|
| mastodon | 100.0% |
| discourse | 100.0% |
| rails | 100.0% |
| rubocop | 100.0% |
| chatwoot | 100.0% |
| errbit | 100.0% |
| activeadmin | 100.0% |
| good_job | 100.0% |
| docuseal | 100.0% |
| rubygems.org | 100.0% |
| doorkeeper | 100.0% |
| fat_free_crm | 100.0% |
| multi_json | 100.0% |

See [bench/results.md](bench/results.md) for full details.

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
      --init                Generate .turbocop.cache with gem paths and exit
      --fail-level <SEV>    Minimum severity for non-zero exit (convention/warning/error/fatal)
  -F, --fail-fast           Stop after first file with offenses
      --force-exclusion     Apply AllCops.Exclude to explicitly-passed files
  -L, --list-target-files   Print files that would be linted, then exit
      --force-default-config  Ignore all config files, use built-in defaults
  -h, --help                Print help
```

## Development

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
