# Rails cop Include pattern FPs

## Problem

~1,300 FPs across multiple Rails cops with per-cop `Include` patterns from
`rubocop-rails`'s `config/default.yml`. Nitrocop fires on ALL files instead of
restricting to the Include-specified paths.

Affected cops (from corpus oracle run 23718439425):

| Cop | FP | Matches | Include pattern |
|-----|---:|--------:|-----------------|
| Rails/I18nLocaleAssignment | 560 | 722 | `spec/**/*.rb`, `test/**/*.rb` |
| Rails/ActionControllerTestCase | 387 | 1,306 | `**/test/**/*.rb` |
| Rails/EnumSyntax | 194 | 256 | (default) |
| Rails/TimeZoneAssignment | 191 | 277 | `spec/**/*.rb`, `test/**/*.rb` |

## Root cause

When nitrocop uses a non-dotfile config (e.g., `baseline_rubocop.yml`), `base_dir`
is set to `CWD` per RuboCop's `base_dir_for_path_parameters` convention.

In `is_cop_match` (`src/config/mod.rs:294-343`), Include patterns are matched
against file paths relativized to `config_dir`, `base_dir`, or stripped of `./`:

```rust
let included = filter.is_included(path)
    || rel_path.is_some_and(|rel| filter.is_included(rel))
    || rel_to_base.is_some_and(|rel| filter.is_included(rel))
    || stripped.is_some_and(|s| filter.is_included(s));
```

In the corpus oracle:
- CWD = workspace root (e.g., `/home/runner/work/nitrocop/nitrocop`)
- Files are at `/home/runner/.../repos/REPO_ID/spec/foo.rb`
- `strip_prefix(base_dir)` → `repos/REPO_ID/spec/foo.rb`
- Pattern `spec/**/*.rb` doesn't match `repos/REPO_ID/spec/foo.rb`
- Result: Include filter never matches → cop runs on ALL files (no Include = match all)

Wait — `None` include means match all. The Include patterns ARE loaded from the
gem, so `include_set` is `Some(...)`. But the glob never matches because paths
have extra prefix.

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

Matches against both the config-relative path AND the original file path.
RuboCop's `relevant_file?` is called with `processed_source.file_path` which
may already be in a form that matches (e.g., repo-relative).

**Key question**: What does `processed_source.file_path` look like in the corpus
oracle? If RuboCop internally converts to repo-relative paths, that would explain
why `spec/**/*.rb` matches for RuboCop but not nitrocop.

## Attempted fix: CWD change (reverted)

Changed `run_nitrocop.py` to use `cwd=repo_dir` instead of `cwd=/tmp`. This made
`base_dir` = repo root, so `strip_prefix(base_dir)` correctly produced
`spec/foo.rb` from `/abs/repo/spec/foo.rb`.

**Reverted** because it introduced +280 new FPs — changing `base_dir` globally
affected ALL pattern resolution (Exclude, AllCops.Exclude, etc.), not just
per-cop Include.

## Proper fix direction

Fix `is_cop_match` to also try relativizing against the **scan root** (the
target directory passed on the command line). The scan root is available as
`DiscoveredFiles` context or could be stored in `CopFilterSet`.

For Include matching only (not Exclude — see `mod.rs:206-211` for why Exclude
must NOT use scan root), add:

```rust
let rel_to_scan_root = scan_root.and_then(|sr| path.strip_prefix(sr).ok());
let included = filter.is_included(path)
    || rel_path.is_some_and(|rel| filter.is_included(rel))
    || rel_to_base.is_some_and(|rel| filter.is_included(rel))
    || rel_to_scan_root.is_some_and(|rel| filter.is_included(rel))
    || stripped.is_some_and(|s| filter.is_included(s));
```

This would need:
1. Storing scan roots in `CopFilterSet` (from CLI args or `discover_files`)
2. Using them only for Include matching
3. Testing that Exclude behavior doesn't change

## Related

- `docs/investigations/investigation-target-dir-relativization.md` — may have
  prior analysis on scan root relativization
- `src/config/mod.rs:206-211` — comment explaining why scan_roots was removed
  for AllCops.Exclude (smoke test regressions)
