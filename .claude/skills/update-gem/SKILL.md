---
name: update-gem
description: Checklist of files to update when adding or updating a rubocop plugin gem
allowed-tools: Read, Grep, Glob
---

When adding a new rubocop plugin gem or updating an existing gem version, the following files must be updated. Use this checklist to ensure nothing is missed.

## Files to update

### 1. `bench/corpus/Gemfile`
Pin the gem version (e.g. `gem "rubocop-rake", "0.7.1"`).

### 2. `bench/corpus/diff_results.py`
Add or update the gem version in the `"baseline"` dictionary inside `write_corpus_results()`.

### 3. `bench/corpus/baseline_rubocop.yml`
Add the gem to the `plugins:` list. If any cops are `Enabled: false` in the gem's vendor default config, add explicit `Enabled: true` overrides (see existing examples for RSpec, Rails, etc.). Cops that default to `Enabled: true` or `Enabled: pending` don't need overrides — `NewCops: enable` handles pending cops.

### 4. `bench/corpus/update_readme.py`
Add an entry to the `GEMS` list with `key`, `url`, and `departments`.

### 5. `scripts/dispatch_cops.py`
Add the department to `DEPT_TO_VENDOR` (maps department name to `vendor/<gem-dir>`). If the department name doesn't snake_case cleanly (e.g. `FactoryBot` -> `factory_bot`, `RSpecRails` -> `rspec_rails`), also add it to `DEPT_TO_DIR` and `DEPT_TO_SRC_DIR`.

### 6. `.github/workflows/batch-dispatch.yml`
Add the department to the `department` choice options list (keep alphabetical order).

### 7. Vendor submodule
Add the vendor submodule pinned to the release tag: `git submodule add <repo-url> vendor/<gem-name>`.

## Reference

See PR #1353 (rubocop-rake support) for an example of adding a new plugin gem, though note it missed several of the items in this checklist (fixed in a follow-up commit).

## Verification

After making changes, run:
```bash
uv run ruff check bench/corpus/diff_results.py scripts/dispatch_cops.py bench/corpus/update_readme.py
```
