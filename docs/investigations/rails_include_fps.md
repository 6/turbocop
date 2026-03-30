# Rails cop Include pattern regressions

## Problem

~1,300 regressions across multiple Rails cops with per-cop `Include` patterns from
`rubocop-rails`'s `config/default.yml`. Nitrocop can't match Include-specified paths
because none of the existing relativization strategies produce the right repo-relative
path when `base_dir` differs from the scan target (repo root).

Affected cops (from corpus oracle run 23718439425):

| Cop | Regressions | Matches | Include pattern |
|-----|---:|--------:|-----------------|
| Rails/I18nLocaleAssignment | 560 | 722 | `spec/**/*.rb`, `test/**/*.rb` |
| Rails/ActionControllerTestCase | 387 | 1,306 | `**/test/**/*.rb` |
| Rails/EnumSyntax | 194 | 256 | (default) |
| Rails/TimeZoneAssignment | 191 | 277 | `spec/**/*.rb`, `test/**/*.rb` |

**Direction**: If Include patterns are loaded (Some) but the glob can't match any
file path, `is_included()` returns false → cop doesn't run → FN (missed offenses).
If Include patterns fail to load (None → match all), cops fire on all files → FP
(extra offenses). The BUNDLE_GEMFILE env var in `run_nitrocop.py:68` suggests gem
resolution should succeed, making the likely behavior FN. The regression column above
may contain a mix of both depending on the oracle run's gem resolution state.

**Verification**: Check corpus oracle stderr for `warning: require 'rubocop-rails'`
lines — their presence would confirm gem loading failure (FP case); absence confirms
patterns are loaded but can't match (FN case).

## Root cause

When nitrocop uses a non-dotfile config (e.g., `baseline_rubocop.yml`), `base_dir`
is set to `CWD` per RuboCop's `base_dir_for_path_parameters` convention.

In `is_cop_match` (`src/config/mod.rs`), Include patterns are matched
against file paths relativized to `config_dir`, `base_dir`, or stripped of `./`:

```rust
let included = filter.is_included(path)
    || rel_path.is_some_and(|rel| filter.is_included(rel))
    || rel_to_base.is_some_and(|rel| filter.is_included(rel))
    || stripped.is_some_and(|s| filter.is_included(s));
```

In the corpus oracle:
- CWD = `/tmp` (via `run_nitrocop.py`, to avoid .gitignore interference)
- `base_dir` = `/tmp` (non-dotfile config → CWD)
- Files are at `/home/runner/.../repos/REPO_ID/spec/foo.rb`
- `strip_prefix(/tmp)` fails (different path tree)
- `config_dir` = `/tmp/nitrocop_corpus_configs/` (overlay parent)
- `strip_prefix(config_dir)` also fails
- Pattern `spec/**/*.rb` doesn't match any relativized form
- Result: `is_included()` returns false → cop doesn't run → FN

Note: `run_nitrocop.py:68` sets `BUNDLE_GEMFILE` and `BUNDLE_PATH` env vars
pointing to `bench/corpus/`, so `bundle info --path rubocop-rails` should succeed
regardless of CWD. Include patterns from the gem config are likely loaded
as `Some(GlobSet)`.

## What RuboCop does differently

RuboCop's `file_name_matches_any?` (in `cop/base.rb:491-497`):

```ruby
def file_name_matches_any?(file, parameter, default_result)
  patterns = cop_config[parameter]
  return default_result unless patterns
  patterns = FilePatterns.from(patterns)
  patterns.match?(config.path_relative_to_config(file)) || patterns.match?(file)
end
```

RuboCop is typically invoked from the repo root with a `.rubocop.yml` in the repo,
so `processed_source.file_path` is repo-relative (e.g., `spec/foo.rb`), making
Include patterns like `spec/**/*.rb` match naturally.

## Attempted fix: CWD change (reverted)

Changed `run_nitrocop.py` to use `cwd=repo_dir` instead of `cwd=/tmp`. This made
`base_dir` = repo root, so `strip_prefix(base_dir)` correctly produced
`spec/foo.rb` from `/abs/repo/spec/foo.rb`.

**Reverted** (commit `44c00c43`) because it introduced +280 new regressions — changing
`base_dir` globally affected ALL pattern resolution (Exclude, AllCops.Exclude, etc.),
not just per-cop Include.

## Fix: scan_root relativization for Include only

Added `scan_roots` (CLI target directories) to `CopFilterSet`. In `is_cop_match`,
file paths are also relativized against each scan root, but ONLY for Include
matching — NOT for Exclude (see `mod.rs:206-213` for why Exclude must NOT use
scan roots).

```rust
let rel_to_scan_root: Option<&Path> = self.scan_roots.iter()
    .find_map(|sr| path.strip_prefix(sr).ok());

let included = filter.is_included(path)
    || rel_path.is_some_and(|rel| filter.is_included(rel))
    || rel_to_base.is_some_and(|rel| filter.is_included(rel))
    || rel_to_scan_root.is_some_and(|rel| filter.is_included(rel))
    || stripped.is_some_and(|s| filter.is_included(s));
```

## Related

- `docs/investigations/investigation-target-dir-relativization.md` — prior analysis
  on scan root relativization
- `src/config/mod.rs:206-213` — comment explaining why scan_roots was removed
  for AllCops.Exclude (smoke test regressions)
