# Turbocop: Rename & RubyGem Distribution Plan

## 1. Renaming turbocop → turbocop

### Why

`turbocop` is already taken on [rubygems.org](https://rubygems.org/gems/turbocop) (dormant since 2017, but squatting the name). We need a clear name for both crates.io and rubygems.org distribution. **`turbocop`** is available on both.

### Checklist

The rename touches ~305 references. A `sed`/`rg` bulk replace handles most of it, but some areas need manual attention.

**Build & metadata:**
- [ ] `Cargo.toml` — crate name (`turbocop` → `turbocop`), binary names (`turbocop` → `turbocop`, `bench_turbocop` → `bench_turbocop`), `default-run`
- [ ] GitHub repo rename (`turbocop` → `turbocop`)

**Source code (manual review needed):**
- [ ] `src/cli.rs` — `#[command(name = "turbocop")]` → `#[command(name = "turbocop")]`
- [ ] `src/main.rs` — `use turbocop::` → `use turbocop::`, `turbocop::run()` → `turbocop::run()`
- [ ] `src/testutil.rs` — directive parsing for `# turbocop-expect:` → `# turbocop-expect:`, `# turbocop-filename:` → `# turbocop-filename:`
- [ ] `src/parse/directives.rs` — regex pattern for `# turbocop:` inline directives, alias handling
- [ ] `src/cop/style/inline_comment.rs` — skip pattern `starts_with("turbocop-")` → `starts_with("turbocop-")`
- [ ] `src/bin/coverage_table.rs` — output text, variable names (~28 refs)
- [ ] `src/config/mod.rs` — ~40 temp directory prefixes in tests (`turbocop_test_*` → `turbocop_test_*`)
- [ ] `src/parse/source.rs` — temp dir `turbocop_test_source`
- [ ] `src/fs/mod.rs` — temp dir pattern `turbocop_test_fs_*`
- [ ] `src/cop/lint/script_permission.rs` — temp path `/tmp/turbocop-test/`

**Test fixtures (bulk replace):**
- [ ] 74 files under `testdata/cops/` containing `# turbocop-expect:` and `# turbocop-filename:` directives
- [ ] `testdata/config/rubocop_only/mixed.yml` — comments mentioning turbocop

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
rg -l 'turbocop' --type rust --type ruby --type md --type yaml | \
  xargs sed -i '' 's/turbocop/turbocop/g; s/TURBOCOP/NITROCOP/g'

# 2. Manual review for mixed-case or context-sensitive replacements
#    (e.g., "turbocop" in prose vs code, bench_turbocop binary name)

# 3. Verify build
cargo check && cargo test

# 4. Rename GitHub repo (Settings → General → Repository name)
```

---

## 2. RubyGem Distribution Strategy

### Architecture: single gem with platform-specific binaries

Use the same pattern as [sorbet-static](https://rubygems.org/gems/sorbet-static) and [Nokogiri](https://rubygems.org/gems/nokogiri): **one gem name with platform-specific variants**. RubyGems natively resolves the correct platform at install time.

```
turbocop-0.1.0.gem                    ← source/fallback (no binary)
turbocop-0.1.0-arm64-darwin.gem       ← macOS Apple Silicon
turbocop-0.1.0-x86_64-darwin.gem      ← macOS Intel
turbocop-0.1.0-x86_64-linux.gem       ← Linux x86_64 (GNU)
turbocop-0.1.0-x86_64-linux-musl.gem  ← Linux x86_64 (Alpine/musl)
turbocop-0.1.0-aarch64-linux.gem      ← Linux ARM64
```

When a user runs `gem install turbocop`, RubyGems picks the matching platform variant automatically.

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

The base gem's `exe/turbocop` is a thin Ruby shim:

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

---

## 3. Claiming the Names

### RubyGems: publish a placeholder

You cannot "reserve" a gem name — you must publish. Create a minimal 0.0.1.pre gem:

```bash
# 1. Create a minimal gemspec
mkdir -p /tmp/turbocop-placeholder/lib
cat > /tmp/turbocop-placeholder/lib/turbocop.rb << 'RUBY'
module Turbocop
  VERSION = "0.0.1.pre"
end
RUBY

cat > /tmp/turbocop-placeholder/turbocop.gemspec << 'RUBY'
Gem::Specification.new do |s|
  s.name     = "turbocop"
  s.version  = "0.0.1.pre"
  s.summary  = "Fast Ruby linter targeting RuboCop compatibility (coming soon)"
  s.authors  = ["Peter"]
  s.license  = "MIT"
  s.homepage = "https://github.com/OWNER/turbocop"
  s.files    = ["lib/turbocop.rb"]
  s.required_ruby_version = ">= 2.7.0"
end
RUBY

# 2. Build and push
cd /tmp/turbocop-placeholder
gem build turbocop.gemspec
gem push turbocop-0.0.1.pre.gem
```

**Note:** You need a rubygems.org account. Sign up at https://rubygems.org/sign_up and run `gem signin` first.

### Crates.io: publish a placeholder

```bash
# In the project root, after renaming Cargo.toml
# Add required fields to Cargo.toml:
#   description = "Fast Ruby linter targeting RuboCop compatibility"
#   license = "MIT"
#   repository = "https://github.com/OWNER/turbocop"

cargo publish --dry-run   # verify it works
cargo publish             # claims the name
```

**Note:** You need a crates.io account linked to your GitHub. Visit https://crates.io and log in with GitHub, then run `cargo login`.

### Do this now

Claim both names before someone else does. The placeholder gems can be yanked later if needed, but having the name registered is what matters.
