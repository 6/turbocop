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
| `-L` / `--list-target-files` | `--list-target-files` | Print files that would be linted (respecting excludes), then exit |
| `-D` / `--display-cop-names` | `--display-cop-names` | Accepted silently (cop names always shown) |
| `-P` / `--parallel` | `--parallel` | Accepted silently (always parallel) |
| `-r` / `--require LIB` | `--require` | Accepted with warning (plugins handled via `require:` in config) |
| `--ignore-disable-comments` | `--ignore-disable-comments` | Ignore all `# rubocop:disable` inline directives |
| `--force-default-config` | `--force-default-config` | Ignore all config files, use built-in defaults only |

## Not Yet Implemented

| Flag | Impact | Difficulty | Notes |
|------|--------|------------|-------|
| `-a` / `--autocorrect` | High | Hard | Safe autocorrect only. Requires fixer infrastructure (M7). |
| `-A` / `--autocorrect-all` | High | Hard | All autocorrects including unsafe. Same dependency on M7. |
| `--auto-gen-config` | Medium | Hard | Generate `.rubocop_todo.yml`. Needs offense counting + YAML generation. |

## Format Values

RuboCop accepts: `progress` (default), `simple`, `clang`, `json`, `html`, `emacs`,
`github`, `quiet`, `pacman`, `offenses`, `worst`, `tap`, `files`, `markdown`.

turbocop accepts: `text` (maps to RuboCop's `progress`), `json`.

Adding `emacs` and `github` would cover the most common CI/editor integrations.
