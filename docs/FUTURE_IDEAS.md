# Future Ideas

Extracted from retired PLAN.md and PLAN2.md. None of these are committed work.

## Autocorrect (M7)

`--fix` / `-a` for safe-only corrections: trailing whitespace, frozen string literal,
string quotes, redundant return, etc. Start with cops where the fix is unambiguous
and non-overlapping. RuboCop's chained autocorrect conflicts are a known hazard.

## LSP Server

Serve diagnostics via LSP for editor integration. Single-file lint is already <50ms,
so the speed is there. Main work is the protocol layer and incremental re-lint on save.

## Strict Mode

`--mode=strict` — exit non-zero if any enabled cops are external (not implemented in
turbocop). For teams that want CI to enforce full coverage. Currently turbocop silently
skips unimplemented cops.

## External Cop Reporting

Print a summary line: "312 enabled, 287 supported, 25 external". Optionally list the
external cops with `--show-external`. Helps users understand coverage gaps.

## Doctor Command

`turbocop doctor` — diagnose common problems: stale lockfile, missing gems,
unsupported config keys, external cops breakdown by source (plugin vs custom require).

## Hybrid Wrapper

A `bin/lint` recipe that runs turbocop for covered cops, then `bundle exec rubocop --only <remaining>`
for the rest. Documents the incremental adoption path for teams that can't drop RuboCop yet.
