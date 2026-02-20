# Turbocop: RubyGem Distribution Plan

## Done

- **Rename** — completed (commit 2815343)
- **RubyGems placeholder** — `turbocop-0.0.1.pre` published (`gems/turbocop/`)

---

## TODO: RubyGem Distribution

### Architecture: single gem with platform-specific binaries

Same pattern as [sorbet-static](https://rubygems.org/gems/sorbet-static) and [Nokogiri](https://rubygems.org/gems/nokogiri): one gem name with platform-specific variants. RubyGems resolves the correct platform at install time.

```
turbocop-0.1.0.gem                    ← source/fallback (no binary)
turbocop-0.1.0-arm64-darwin.gem       ← macOS Apple Silicon
turbocop-0.1.0-x86_64-darwin.gem      ← macOS Intel
turbocop-0.1.0-x86_64-linux.gem       ← Linux x86_64 (GNU)
turbocop-0.1.0-x86_64-linux-musl.gem  ← Linux x86_64 (Alpine/musl)
turbocop-0.1.0-aarch64-linux.gem      ← Linux ARM64
```

### Target platforms

| Platform | Rust target | CI runner |
|----------|-------------|-----------|
| macOS ARM64 (M1+) | `aarch64-apple-darwin` | `macos-latest` |
| macOS Intel | `x86_64-apple-darwin` | `macos-13` |
| Linux x86_64 GNU | `x86_64-unknown-linux-gnu` | `ubuntu-latest` |
| Linux x86_64 musl | `x86_64-unknown-linux-musl` | `ubuntu-latest` + cross |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | `ubuntu-latest` + cross |

### Directory layout

```
gems/
└── turbocop/
    ├── turbocop.gemspec          # Main gemspec
    ├── lib/
    │   └── turbocop.rb           # Version constant + binary locator
    ├── exe/
    │   └── turbocop              # Wrapper script (fallback to source build)
    └── Rakefile                  # gem build tasks
```

### Gemspec

```ruby
# gems/turbocop/turbocop.gemspec
Gem::Specification.new do |spec|
  spec.name     = "turbocop"
  spec.version  = "0.1.0"
  spec.authors  = ["Peter"]
  spec.summary  = "Fast Ruby linter targeting RuboCop compatibility"
  spec.homepage = "https://github.com/OWNER/turbocop"
  spec.license  = "MIT"

  spec.required_ruby_version = ">= 2.7.0"

  spec.files       = Dir["lib/**/*", "exe/**/*", "*.md"]
  spec.bindir      = "exe"
  spec.executables = ["turbocop"]
end
```

For platform-specific variants, the gemspec adds:

```ruby
spec.platform = "arm64-darwin"   # (or x86_64-linux, etc.)
spec.files    = ["exe/turbocop"] # Just the precompiled binary
```

### Wrapper script (`exe/turbocop`)

```ruby
#!/usr/bin/env ruby
# frozen_string_literal: true

require "rubygems"

# Find the precompiled binary from the platform-specific gem
gem_dir = File.expand_path("..", __dir__)
binary  = File.join(gem_dir, "exe", "turbocop")

if File.executable?(binary) && !File.read(binary, 2).start_with?("#!")
  # We ARE the platform gem — exec the real binary
  exec(binary, *ARGV)
else
  # Fallback: tell the user to install from source
  warn "turbocop: no precompiled binary for #{RUBY_PLATFORM}"
  warn "Install Rust and run: cargo install turbocop"
  exit 1
end
```

### CI release workflow (GitHub Actions)

```yaml
# .github/workflows/release.yml
name: Release
on:
  push:
    tags: ["v*"]

jobs:
  build:
    strategy:
      matrix:
        include:
          - { os: macos-latest,  target: aarch64-apple-darwin,        platform: arm64-darwin }
          - { os: macos-13,      target: x86_64-apple-darwin,         platform: x86_64-darwin }
          - { os: ubuntu-latest, target: x86_64-unknown-linux-gnu,    platform: x86_64-linux }
          - { os: ubuntu-latest, target: x86_64-unknown-linux-musl,   platform: x86_64-linux-musl }
          - { os: ubuntu-latest, target: aarch64-unknown-linux-gnu,   platform: aarch64-linux }
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: "${{ matrix.target }}" }

      - name: Install cross (for cross-compilation)
        if: contains(matrix.target, 'aarch64-unknown-linux') || contains(matrix.target, 'musl')
        run: cargo install cross

      - name: Build
        run: |
          if command -v cross &>/dev/null && [ "${{ matrix.target }}" != "$(rustc -vV | grep host | cut -d' ' -f2)" ]; then
            cross build --release --target ${{ matrix.target }}
          else
            cargo build --release --target ${{ matrix.target }}
          fi

      - name: Package platform gem
        run: ruby script/build_platform_gem.rb ${{ matrix.platform }} target/${{ matrix.target }}/release/turbocop

      - uses: actions/upload-artifact@v4
        with:
          name: gem-${{ matrix.platform }}
          path: "*.gem"

  publish:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        with: { path: gems/, merge-multiple: true }
      - name: Publish all gems
        env:
          GEM_HOST_API_KEY: ${{ secrets.RUBYGEMS_API_KEY }}
        run: |
          # Publish base gem first, then platform variants
          gem push gems/turbocop-*.gem --force 2>/dev/null || true
          for f in gems/turbocop-*-*.gem; do gem push "$f"; done
```

### Also publish on crates.io

```bash
cargo publish   # publishes to crates.io as "turbocop"
```

Users who have Rust installed can also `cargo install turbocop`.
