# Excluded Cops in Conformance

Cops excluded from per-repo conformance comparison via `per_repo_excluded_cops()` in `bench/bench.rs`. Each exclusion represents a known divergence between turbocop and RuboCop that isn't a turbocop bug.

## fat_free_crm

**Cops:** `Style/RedundantRegexpEscape`, `Layout/FirstArrayElementIndentation`, `Layout/MultilineMethodCallIndentation`, `Style/TrailingCommaInHashLiteral`

**Reason:** RuboCop reports 0 offenses on these cops even when run with `--only`, but the source code contains patterns that match the cop specifications. turbocop correctly flags them. These are RuboCop quirks (likely autocorrect artifacts in RuboCop's cache or parser differences), not turbocop bugs.

## multi_json

**Cop:** `Style/EmptyClassDefinition`

**Reason:** Config interaction between `require: standard` and `NewCops: enable`.

- Standard gem (v1.54.0) `config/base.yml` sets `Style/EmptyClassDefinition: Enabled: false`
- RuboCop's `config/default.yml` sets `Enabled: pending` (cop added in v1.84)
- multi_json's `.rubocop.yml` sets `NewCops: enable`
- RuboCop's `--show-cops` shows `Enabled: pending` (not `false`), meaning standard's explicit disable is being ignored
- RuboCop fires the cop (1 offense); turbocop does not

This appears to be a RuboCop quirk where `require:` extensions inject config at a different stage than YAML inheritance. The `NewCops: enable` setting sees the default `pending` state rather than standard's `false`. turbocop correctly applies the YAML config chain (standard's `false` wins) and disables the cop.

**Resolution options:**
1. Match RuboCop's behavior by treating `NewCops: enable` as promoting `pending` even when an inherited config explicitly sets `Enabled: false` — risky, may cause FPs in other repos
2. Accept as a known RuboCop quirk and keep the exclusion
3. Investigate RuboCop's `require:` config injection order to understand the exact semantics

## Lint/Syntax (all repos with TargetRubyVersion < 3.0)

**Repos:** rubocop (2.7), activeadmin (2.6)

**Reason:** Prism always parses modern Ruby syntax. It cannot detect parser-version-specific syntax errors (e.g., `...` forwarding under Ruby 2.6) that the legacy parser gem would flag. This is a fundamental limitation of using Prism as the parser.
