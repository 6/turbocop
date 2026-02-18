# Nitrocop: Rename & RubyGem Distribution Plan

## 1. Renaming rblint → nitrocop

### Why

`rblint` is already taken on [rubygems.org](https://rubygems.org/gems/rblint) (dormant since 2017, but squatting the name). We need a clear name for both crates.io and rubygems.org distribution. **`nitrocop`** is available on both.

### Checklist

The rename touches ~305 references. A `sed`/`rg` bulk replace handles most of it, but some areas need manual attention.

**Build & metadata:**
- [ ] `Cargo.toml` — crate name (`rblint` → `nitrocop`), binary names (`rblint` → `nitrocop`, `bench_rblint` → `bench_nitrocop`), `default-run`
- [ ] GitHub repo rename (`rblint` → `nitrocop`)

**Source code (manual review needed):**
- [ ] `src/cli.rs` — `#[command(name = "rblint")]` → `#[command(name = "nitrocop")]`
- [ ] `src/main.rs` — `use rblint::` → `use nitrocop::`, `rblint::run()` → `nitrocop::run()`
- [ ] `src/testutil.rs` — directive parsing for `# rblint-expect:` → `# nitrocop-expect:`, `# rblint-filename:` → `# nitrocop-filename:`
- [ ] `src/parse/directives.rs` — regex pattern for `# rblint:` inline directives, alias handling
- [ ] `src/cop/style/inline_comment.rs` — skip pattern `starts_with("rblint-")` → `starts_with("nitrocop-")`
- [ ] `src/bin/coverage_table.rs` — output text, variable names (~28 refs)
- [ ] `src/config/mod.rs` — ~40 temp directory prefixes in tests (`rblint_test_*` → `nitrocop_test_*`)
- [ ] `src/parse/source.rs` — temp dir `rblint_test_source`
- [ ] `src/fs/mod.rs` — temp dir pattern `rblint_test_fs_*`
- [ ] `src/cop/lint/script_permission.rs` — temp path `/tmp/rblint-test/`

**Test fixtures (bulk replace):**
- [ ] 74 files under `testdata/cops/` containing `# rblint-expect:` and `# rblint-filename:` directives
- [ ] `testdata/config/rubocop_only/mixed.yml` — comments mentioning rblint

**Benchmark & scripts:**
- [ ] `bench/bench.rs` — function names, binary paths, hyperfine commands (~63 refs)
- [ ] `bench/compare.rb` — variable names, JSON keys (~19 refs)
- [ ] `bench/report.rb` — output text, table headers (~12 refs)

**Documentation:**
- [ ] `README.md` — title, examples, URLs, benchmark tables
- [ ] `PROGRESS.md` — title and references
- [ ] `PLAN.md`, `PLAN2.md`, `TODO.md`, `OPTIMIZATION_IDEAS.md`
- [ ] `docs/coverage.md` — title, table headers, cop counts
- [ ] `bench/results.md` — benchmark tables
- [ ] `CLAUDE.md` — project description and all references

### Execution strategy

```bash
# 1. Bulk rename in file contents (covers ~90% of references)
rg -l 'rblint' --type rust --type ruby --type md --type yaml | \
  xargs sed -i '' 's/rblint/nitrocop/g; s/RBLINT/NITROCOP/g'

# 2. Manual review for mixed-case or context-sensitive replacements
#    (e.g., "rblint" in prose vs code, bench_rblint binary name)

# 3. Verify build
cargo check && cargo test

# 4. Rename GitHub repo (Settings → General → Repository name)
```

---

## 2. RubyGem Distribution Strategy

### Architecture: single gem with platform-specific binaries

Use the same pattern as [sorbet-static](https://rubygems.org/gems/sorbet-static) and [Nokogiri](https://rubygems.org/gems/nokogiri): **one gem name with platform-specific variants**. RubyGems natively resolves the correct platform at install time.

```
nitrocop-0.1.0.gem                    ← source/fallback (no binary)
nitrocop-0.1.0-arm64-darwin.gem       ← macOS Apple Silicon
nitrocop-0.1.0-x86_64-darwin.gem      ← macOS Intel
nitrocop-0.1.0-x86_64-linux.gem       ← Linux x86_64 (GNU)
nitrocop-0.1.0-x86_64-linux-musl.gem  ← Linux x86_64 (Alpine/musl)
nitrocop-0.1.0-aarch64-linux.gem      ← Linux ARM64
```

When a user runs `gem install nitrocop`, RubyGems picks the matching platform variant automatically.

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
└── nitrocop/
    ├── nitrocop.gemspec          # Main gemspec
    ├── lib/
    │   └── nitrocop.rb           # Version constant + binary locator
    ├── exe/
    │   └── nitrocop              # Wrapper script (fallback to source build)
    └── Rakefile                  # gem build tasks
```

### Gemspec

```ruby
# gems/nitrocop/nitrocop.gemspec
Gem::Specification.new do |spec|
  spec.name     = "nitrocop"
  spec.version  = "0.1.0"
  spec.authors  = ["Peter"]
  spec.summary  = "Fast Ruby linter targeting RuboCop compatibility"
  spec.homepage = "https://github.com/OWNER/nitrocop"
  spec.license  = "MIT"

  spec.required_ruby_version = ">= 2.7.0"

  spec.files       = Dir["lib/**/*", "exe/**/*", "*.md"]
  spec.bindir      = "exe"
  spec.executables = ["nitrocop"]
end
```

For platform-specific variants, the gemspec adds:

```ruby
spec.platform = "arm64-darwin"   # (or x86_64-linux, etc.)
spec.files    = ["exe/nitrocop"] # Just the precompiled binary
```

### Wrapper script (`exe/nitrocop`)

The base gem's `exe/nitrocop` is a thin Ruby shim:

```ruby
#!/usr/bin/env ruby
# frozen_string_literal: true

require "rubygems"

# Find the precompiled binary from the platform-specific gem
gem_dir = File.expand_path("..", __dir__)
binary  = File.join(gem_dir, "exe", "nitrocop")

if File.executable?(binary) && !File.read(binary, 2).start_with?("#!")
  # We ARE the platform gem — exec the real binary
  exec(binary, *ARGV)
else
  # Fallback: tell the user to install from source
  warn "nitrocop: no precompiled binary for #{RUBY_PLATFORM}"
  warn "Install Rust and run: cargo install nitrocop"
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
        run: ruby script/build_platform_gem.rb ${{ matrix.platform }} target/${{ matrix.target }}/release/nitrocop

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
          gem push gems/nitrocop-*.gem --force 2>/dev/null || true
          for f in gems/nitrocop-*-*.gem; do gem push "$f"; done
```

### Also publish on crates.io

```bash
cargo publish   # publishes to crates.io as "nitrocop"
```

Users who have Rust installed can also `cargo install nitrocop`.

---

## 3. Claiming the Names

### RubyGems: publish a placeholder

You cannot "reserve" a gem name — you must publish. Create a minimal 0.0.1.pre gem:

```bash
# 1. Create a minimal gemspec
mkdir -p /tmp/nitrocop-placeholder/lib
cat > /tmp/nitrocop-placeholder/lib/nitrocop.rb << 'RUBY'
module Nitrocop
  VERSION = "0.0.1.pre"
end
RUBY

cat > /tmp/nitrocop-placeholder/nitrocop.gemspec << 'RUBY'
Gem::Specification.new do |s|
  s.name     = "nitrocop"
  s.version  = "0.0.1.pre"
  s.summary  = "Fast Ruby linter targeting RuboCop compatibility (coming soon)"
  s.authors  = ["Peter"]
  s.license  = "MIT"
  s.homepage = "https://github.com/OWNER/nitrocop"
  s.files    = ["lib/nitrocop.rb"]
  s.required_ruby_version = ">= 2.7.0"
end
RUBY

# 2. Build and push
cd /tmp/nitrocop-placeholder
gem build nitrocop.gemspec
gem push nitrocop-0.0.1.pre.gem
```

**Note:** You need a rubygems.org account. Sign up at https://rubygems.org/sign_up and run `gem signin` first.

### Crates.io: publish a placeholder

```bash
# In the project root, after renaming Cargo.toml
# Add required fields to Cargo.toml:
#   description = "Fast Ruby linter targeting RuboCop compatibility"
#   license = "MIT"
#   repository = "https://github.com/OWNER/nitrocop"

cargo publish --dry-run   # verify it works
cargo publish             # claims the name
```

**Note:** You need a crates.io account linked to your GitHub. Visit https://crates.io and log in with GitHub, then run `cargo login`.

### Do this now

Claim both names before someone else does. The placeholder gems can be yanked later if needed, but having the name registered is what matters.
