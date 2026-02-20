# CI & Release Workflow Plan

Key design points: git submodules (vendor/rubocop*), cross-platform rubygem publishing, musl target for Alpine.

## 1. Checks Workflow (`.github/workflows/checks.yml`)

Runs on push to main and PRs. Concurrency-limited per-branch.

```yaml
name: Checks
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

on:
  push:
    branches: [main]
  pull_request:

jobs:
  build-and-test:
    timeout-minutes: 15
    strategy:
      matrix:
        os: [ubuntu-24.04, macos-26]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
        with:
          submodules: true   # ← needed for vendor/rubocop*

      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - uses: Swatinem/rust-cache@v2

      - name: Format
        if: matrix.os == 'ubuntu-24.04'
        run: cargo fmt --check

      - name: Clippy
        if: matrix.os == 'ubuntu-24.04'
        run: cargo clippy --release -- -D warnings

      - name: Build
        run: cargo build --release

      - name: Test
        run: cargo test --release

      - name: Config audit
        if: matrix.os == 'ubuntu-24.04'
        run: cargo test --release config_audit -- --nocapture

      - name: Prism pitfalls
        if: matrix.os == 'ubuntu-24.04'
        run: cargo test --release prism_pitfalls -- --nocapture
```

**Notes:**
- `submodules: true` on checkout — needed for vendor/rubocop*
- `config_audit` and `prism_pitfalls` are the zero-tolerance integration tests (see CLAUDE.md)

## 2. Release Workflow (`.github/workflows/release.yml`)

Manual dispatch. Bumps version, tags, builds all platforms, publishes to GitHub + crates.io + rubygems.

```yaml
name: Release

on:
  workflow_dispatch:
    inputs:
      version:
        description: "Version to release (e.g. 0.1.0)"
        required: true
        type: string

permissions:
  contents: write

env:
  CARGO_TERM_COLOR: always
  BINARY_NAME: nitrocop

jobs:
  prepare:
    runs-on: ubuntu-24.04
    timeout-minutes: 5
    steps:
      - uses: actions/checkout@v4

      - name: Validate version
        run: |
          if ! echo "${{ inputs.version }}" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
            echo "::error::Invalid version format. Use semver like 0.1.0"
            exit 1
          fi
          if git rev-parse "v${{ inputs.version }}" >/dev/null 2>&1; then
            echo "::error::Tag v${{ inputs.version }} already exists"
            exit 1
          fi

      - name: Bump version and tag
        run: |
          sed -i 's/^version = ".*"/version = "${{ inputs.version }}"/' Cargo.toml
          cargo update -p nitrocop
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add Cargo.toml Cargo.lock
          git commit -m "Release v${{ inputs.version }}"
          git tag "v${{ inputs.version }}"
          git push origin main --tags

  build:
    needs: prepare
    strategy:
      matrix:
        include:
          - target: x86_64-apple-darwin
            os: macos-26
            platform: x86_64-darwin
            cross: false
          - target: aarch64-apple-darwin
            os: macos-26
            platform: arm64-darwin
            cross: false
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-24.04
            platform: x86_64-linux
            cross: false
          - target: x86_64-unknown-linux-musl
            os: ubuntu-24.04
            platform: x86_64-linux-musl
            cross: true
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-24.04
            platform: aarch64-linux
            cross: true
    runs-on: ${{ matrix.os }}
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v4
        with:
          ref: v${{ inputs.version }}
          submodules: true

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.target }}

      - name: Install cross
        if: matrix.cross
        run: cargo install cross --git https://github.com/cross-rs/cross

      - name: Build with cargo
        if: "!matrix.cross"
        run: cargo build --release --target ${{ matrix.target }}

      - name: Build with cross
        if: matrix.cross
        run: cross build --release --target ${{ matrix.target }}

      # GitHub release tarball
      - name: Package tarball
        run: |
          cd target/${{ matrix.target }}/release
          tar czf ../../../${BINARY_NAME}-${{ matrix.target }}.tar.gz ${BINARY_NAME}

      # Platform-specific rubygem
      - name: Package platform gem
        run: ruby script/build_platform_gem.rb ${{ inputs.version }} ${{ matrix.platform }} target/${{ matrix.target }}/release/${BINARY_NAME}

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: build-${{ matrix.target }}
          path: |
            ${BINARY_NAME}-${{ matrix.target }}.tar.gz
            *.gem

  release:
    needs: build
    runs-on: ubuntu-24.04
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@v4
        with:
          ref: v${{ inputs.version }}

      - uses: actions/download-artifact@v4
        with:
          pattern: build-*
          merge-multiple: true

      - name: Create GitHub release
        run: gh release create "v${{ inputs.version }}" ${BINARY_NAME}-*.tar.gz --generate-notes
        env:
          GH_TOKEN: ${{ github.token }}

  publish-crates:
    needs: build
    runs-on: ubuntu-24.04
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@v4
        with:
          ref: v${{ inputs.version }}
          submodules: true

      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Publish to crates.io
        run: cargo publish
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}

  publish-gems:
    needs: build
    runs-on: ubuntu-24.04
    timeout-minutes: 10
    steps:
      - uses: actions/download-artifact@v4
        with:
          pattern: build-*
          merge-multiple: true

      - name: Build base gem
        run: |
          ruby script/build_base_gem.rb ${{ inputs.version }}

      - name: Publish all gems
        env:
          GEM_HOST_API_KEY: ${{ secrets.RUBYGEMS_API_KEY }}
        run: |
          # Base gem first (no platform suffix)
          gem push nitrocop-${{ inputs.version }}.gem
          # Then platform-specific gems
          for f in nitrocop-${{ inputs.version }}-*.gem; do
            gem push "$f"
          done
```

## 3. Required Secrets

- `CARGO_REGISTRY_TOKEN` — from https://crates.io/settings/tokens
- `RUBYGEMS_API_KEY` — from https://rubygems.org/profile/api_keys

## 4. Required Scripts (to be created)

- `script/build_platform_gem.rb` — takes `(version, platform, binary_path)`, outputs `nitrocop-{version}-{platform}.gem`
- `script/build_base_gem.rb` — takes `(version)`, outputs `nitrocop-{version}.gem` (no binary, just the wrapper)

## 5. Prerequisite

The rename from `turbocop` → `nitrocop` must happen first. See [RUBYGEM_PLAN.md](RUBYGEM_PLAN.md).
