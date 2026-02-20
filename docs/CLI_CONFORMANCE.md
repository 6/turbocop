# CLI Conformance with RuboCop

RuboCop CLI flags that turbocop should accept for drop-in compatibility.

## Implemented

| Flag | turbocop | Notes |
|------|----------|-------|
| `[PATHS]...` | `paths` | Same behavior |
| `-c` / `--config FILE` | `--config` | Same |
| `-f` / `--format FORMAT` | `--format` | `progress` (default), `text`, `json`, `github`, `pacman`, `quiet`, `files`, `emacs`, `simple` |
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

RuboCop accepts 14 format values. turbocop implements the most useful subset.

### Implemented

| Format | Default? | Description | Example Output |
|--------|----------|-------------|----------------|
| `progress` | **Yes** (turbocop + RuboCop default) | One char per file: `.` = clean, `C`/`W`/`E`/`F` = worst severity. Offense details + summary follow. | `..C.W...` |
| `text` | | Alias for `emacs`/`simple`. Per-offense lines with file path, line, column, severity, cop name, and message. Summary at end. Legacy turbocop default. | `foo.rb:3:5: C: Style/Foo: msg` |
| `emacs` | | Same as `text`. Machine-parsable, one offense per line. `path:line:col: SEVERITY: Cop/Name: message`. | `foo.rb:3:5: C: Style/Foo: msg` |
| `simple` | | Same as `text`/`emacs` in turbocop. (In RuboCop, `simple` omits file path and groups by file; turbocop treats as alias.) | `foo.rb:3:5: C: Style/Foo: msg` |
| `json` | | Structured JSON with `metadata` (file count, offense count) and `offenses` array. | `{"metadata": {...}, "offenses": [...]}` |
| `github` | | GitHub Actions workflow annotations. Convention/Warning → `::warning`, Error/Fatal → `::error`. No summary line. | `::warning file=foo.rb,line=3,col=5::Style/Foo: msg` |
| `pacman` | | Pac-Man visual format. `ᗧ` eats `•` (clean files), `ᗣ` (ghost) for files with offenses. Offense details + summary follow. | `ᗧ••ᗣ•••ᗣ••` |
| `quiet` | | Same as `text` when offenses exist. Completely silent (no output) when all files are clean. Useful for CI. | *(nothing on clean run)* |
| `files` | | Deduplicated, sorted file paths with offenses, one per line. No offense details, no summary. Useful for piping. | `foo.rb` |

### Not Yet Implemented

| Format | RuboCop Description | Notes |
|--------|---------------------|-------|
| `clang` | Like `emacs` but shows the offending source line + caret (`^`) pointing to the column. | Needs source text access in formatter. |
| `offenses` | Cop summary: count per cop sorted by frequency. `89 Layout/LineLength`. | Useful for triage. |
| `worst` | File summary: offense count per file sorted descending. `89 lib/foo.rb`. | Useful for triage. |
| `html` | Full standalone HTML report. | Low priority. |
| `tap` | TAP (Test Anything Protocol) format. | Low demand. |
| `markdown` | Markdown table output. | Low demand. |
