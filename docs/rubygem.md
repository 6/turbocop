# RubyGem Distribution

nitrocop is distributed as a RubyGem with precompiled platform-specific binaries, following the same pattern as [sorbet-static](https://rubygems.org/gems/sorbet-static) and [Nokogiri](https://rubygems.org/gems/nokogiri).

## Installation

```ruby
# Gemfile
gem "nitrocop", group: :development
```

Or install directly:

```bash
gem install nitrocop
```

RubyGems automatically resolves the correct platform variant. Users with Rust installed can also `cargo install nitrocop`.

## How it works

A single gem name (`nitrocop`) has platform-specific variants. RubyGems picks the right one at install time:

```
nitrocop-X.Y.Z.gem                    ← base/fallback (no binary)
nitrocop-X.Y.Z-arm64-darwin.gem       ← macOS Apple Silicon
nitrocop-X.Y.Z-x86_64-linux.gem       ← Linux x86_64 (GNU)
nitrocop-X.Y.Z-x86_64-linux-musl.gem  ← Linux x86_64 (Alpine/musl)
nitrocop-X.Y.Z-aarch64-linux.gem      ← Linux ARM64
```

**Platform gems** contain the native binary at `libexec/nitrocop`. The Ruby binstub at `exe/nitrocop` finds and execs it.

**Base gem** (fallback) has no binary — the binstub prints a helpful error with install-from-source instructions.

## Directory layout

```
gem/
├── nitrocop.gemspec      # Gemspec (reads version from lib/nitrocop.rb)
├── build_gem.rb          # Build script (base + platform gems)
├── lib/
│   ├── nitrocop.rb       # VERSION constant + Nitrocop.executable finder
│   └── gem_builder.rb    # GemBuilder class (shared build logic)
├── exe/
│   └── nitrocop          # Ruby binstub (finds libexec binary or shows error)
├── test/
│   └── gem_builder_test.rb  # GemBuilder tests
└── libexec/
    └── nitrocop          # Native binary (only in platform gems, not checked in)
```

## Build script

`gem/build_gem.rb` builds both base and platform gems. Called by `.github/workflows/release.yml`.

```
ruby gem/build_gem.rb VERSION                                         # base gem (no binary)
ruby gem/build_gem.rb VERSION --platform PLATFORM --binary PATH       # platform gem
```

The shared logic lives in `gem/lib/gem_builder.rb` (`GemBuilder` class), tested by `gem/test/gem_builder_test.rb`.

### Local testing

```bash
cargo build --release
ruby gem/build_gem.rb 0.1.0.dev --platform arm64-darwin --binary target/release/nitrocop
ruby gem/build_gem.rb 0.1.0.dev
gem install nitrocop-0.1.0.dev-arm64-darwin.gem
nitrocop --help
gem uninstall nitrocop
```

## Release workflow

Triggered via `workflow_dispatch` with a version input (e.g. `0.1.0`):

1. **prepare** — bumps version in `Cargo.toml` and `gem/lib/nitrocop.rb`, commits, tags
2. **build** — cross-compiles for 4 platforms, packages tarballs + platform gems
3. **release** — creates GitHub Release with tarballs
4. **publish-crates** — `cargo publish` to crates.io
5. **publish-gems** — builds base gem, then pushes base + all platform gems to RubyGems

### Target platforms

| Platform | Rust target | CI runner |
|----------|-------------|-----------|
| macOS ARM64 (M1+) | `aarch64-apple-darwin` | `macos-latest` |
| Linux x86_64 GNU | `x86_64-unknown-linux-gnu` | `ubuntu-24.04` |
| Linux x86_64 musl | `x86_64-unknown-linux-musl` | `ubuntu-24.04` + cross |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | `ubuntu-24.04` + cross |

### Required secrets

- `CARGO_REGISTRY_TOKEN` — for crates.io
- `RUBYGEMS_API_KEY` — for rubygems.org
