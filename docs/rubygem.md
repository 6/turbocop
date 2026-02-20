# RubyGem Distribution

turbocop is distributed as a RubyGem with precompiled platform-specific binaries, following the same pattern as [sorbet-static](https://rubygems.org/gems/sorbet-static) and [Nokogiri](https://rubygems.org/gems/nokogiri).

## Installation

```ruby
# Gemfile
gem "turbocop", group: :development
```

Or install directly:

```bash
gem install turbocop
```

RubyGems automatically resolves the correct platform variant. Users with Rust installed can also `cargo install turbocop`.

## How it works

A single gem name (`turbocop`) has platform-specific variants. RubyGems picks the right one at install time:

```
turbocop-X.Y.Z.gem                    ← base/fallback (no binary)
turbocop-X.Y.Z-arm64-darwin.gem       ← macOS Apple Silicon
turbocop-X.Y.Z-x86_64-darwin.gem      ← macOS Intel
turbocop-X.Y.Z-x86_64-linux.gem       ← Linux x86_64 (GNU)
turbocop-X.Y.Z-x86_64-linux-musl.gem  ← Linux x86_64 (Alpine/musl)
turbocop-X.Y.Z-aarch64-linux.gem      ← Linux ARM64
```

**Platform gems** contain the native binary at `libexec/turbocop`. The Ruby binstub at `exe/turbocop` finds and execs it.

**Base gem** (fallback) has no binary — the binstub prints a helpful error with install-from-source instructions.

## Directory layout

```
gems/turbocop/
├── turbocop.gemspec      # Gemspec (reads version from lib/turbocop.rb)
├── lib/
│   └── turbocop.rb       # VERSION constant + Turbocop.executable finder
├── exe/
│   └── turbocop          # Ruby binstub (finds libexec binary or shows error)
└── libexec/
    └── turbocop          # Native binary (only in platform gems, not checked in)
```

## Build script

`script/build_gem.rb` builds both base and platform gems. Called by `.github/workflows/release.yml`.

```
ruby script/build_gem.rb VERSION                                         # base gem (no binary)
ruby script/build_gem.rb VERSION --platform PLATFORM --binary PATH       # platform gem
```

The shared logic lives in `script/lib/gem_builder.rb` (`GemBuilder` class), tested by `script/test/gem_builder_test.rb`.

### Local testing

```bash
cargo build --release
ruby script/build_gem.rb 0.1.0.dev --platform arm64-darwin --binary target/release/turbocop
ruby script/build_gem.rb 0.1.0.dev
gem install turbocop-0.1.0.dev-arm64-darwin.gem
turbocop --help
gem uninstall turbocop
```

## Release workflow

Triggered via `workflow_dispatch` with a version input (e.g. `0.1.0`):

1. **prepare** — bumps version in `Cargo.toml` and `gems/turbocop/lib/turbocop.rb`, commits, tags
2. **build** — cross-compiles for 5 platforms, packages tarballs + platform gems
3. **release** — creates GitHub Release with tarballs
4. **publish-crates** — `cargo publish` to crates.io
5. **publish-gems** — builds base gem, then pushes base + all platform gems to RubyGems

### Target platforms

| Platform | Rust target | CI runner |
|----------|-------------|-----------|
| macOS ARM64 (M1+) | `aarch64-apple-darwin` | `macos-15` |
| macOS Intel | `x86_64-apple-darwin` | `macos-13` |
| Linux x86_64 GNU | `x86_64-unknown-linux-gnu` | `ubuntu-24.04` |
| Linux x86_64 musl | `x86_64-unknown-linux-musl` | `ubuntu-24.04` + cross |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | `ubuntu-24.04` + cross |

### Required secrets

- `CARGO_REGISTRY_TOKEN` — for crates.io
- `RUBYGEMS_API_KEY` — for rubygems.org
