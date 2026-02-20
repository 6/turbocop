# CLI Conformance with RuboCop

RuboCop CLI flags that turbocop should accept for drop-in compatibility.

## Implemented

| Flag | turbocop | Notes |
|------|----------|-------|
| `[PATHS]...` | `paths` | Same behavior |
| `-c` / `--config FILE` | `--config` | Same |
| `-f` / `--format FORMAT` | `--format` | `text`, `json` (RuboCop also has `emacs`, `html`, `github`, etc.) |
| `--only COP1,COP2` | `--only` | Same |
| `--except COP1,COP2` | `--except` | Same |
| `--no-color` | `--no-color` | Same |
| `-s` / `--stdin FILE` | `--stdin` | Same |
| `--fail-level SEVERITY` | `--fail-level` | Same (convention/C, warning/W, error/E, fatal/F) |
| `-F` / `--fail-fast` | `--fail-fast` | Stop after first file with offenses |
| `--force-exclusion` | `--force-exclusion` | Apply AllCops.Exclude to explicitly-passed files (default: explicit files bypass excludes) |

## Not Yet Implemented

| Flag | Impact | Difficulty | Notes |
|------|--------|------------|-------|
| `-L` / `--list-target-files` | Low-Medium | Easy | Print files that would be linted, then exit. File discovery already exists. |
| `-D` / `--display-cop-names` | Cosmetic | Trivial | Already shown by default. Accept flag silently to avoid "unknown flag" errors. |
| `-P` / `--parallel` | Cosmetic | Trivial | Already always parallel. Accept and ignore. |
| `-r` / `--require` | Medium | Easy | Accept and ignore (plugins handled via config). Avoids errors from `.rubocop` options files. |
| `--force-default-config` | Low | Easy | Ignore all config files, use built-in defaults only. |
| `--ignore-disable-comments` | Low | Easy | Ignore `# rubocop:disable` directives. |
| `-a` / `--autocorrect` | High | Hard | Safe autocorrect only. Requires fixer infrastructure (M7). |
| `-A` / `--autocorrect-all` | High | Hard | All autocorrects including unsafe. Same dependency on M7. |
| `--auto-gen-config` | Medium | Hard | Generate `.rubocop_todo.yml`. Needs offense counting + YAML generation. |

## Format Values

RuboCop accepts: `progress` (default), `simple`, `clang`, `json`, `html`, `emacs`,
`github`, `quiet`, `pacman`, `offenses`, `worst`, `tap`, `files`, `markdown`.

turbocop accepts: `text` (maps to RuboCop's `progress`), `json`.

Adding `emacs` and `github` would cover the most common CI/editor integrations.
