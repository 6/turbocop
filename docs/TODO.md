# Remaining Cop Coverage: Rails Schema Cops

2 cops remain to reach 100% coverage across all gems. Both require `db/schema.rb` parsing, which nitrocop doesn't have yet.

**Current conformance impact: none.** Neither cop fires on any bench repo today — our no-op stubs match RuboCop's output perfectly. This is a completeness goal, not a conformance fix.

## Rails/UniqueValidationWithoutIndex

**Difficulty: Medium-Hard.** The cop itself is moderately complex (validates call matching + scope handling + polymorphic associations), but the real work is the schema loader prerequisite.

**Enabled by default** in rubocop-rails. Scoped to `**/app/models/**/*.rb`.

Detects `validates :col, uniqueness: true` without a corresponding unique database index. Without the index, race conditions can still insert duplicates and the validation SELECT is slow on large tables.

**Needs from schema:**
- Table name for the model class (derived from class name or `self.table_name =`)
- Column names being validated (including `scope:` columns)
- Whether a matching unique index exists (including expression indexes like `lower(email)`)

**Edge cases:**
- `uniqueness: false` / `uniqueness: nil` → skip
- Conditional validations (`if:`, `unless:`) → skip
- Polymorphic `belongs_to` → check both `_id` and `_type` columns
- Expression indexes → substring match on column name
- Scope with `.freeze` → unwrap frozen array

**Stub:** `src/cop/rails/unique_validation_without_index.rs` (empty `check_node`)

**Bench repos for testing:** chatwoot has explicit excludes for this cop (6 model files), suggesting it fires there. mastodon, doorkeeper, fat_free_crm, good_job all have `db/schema.rb`.

## Rails/UnusedIgnoredColumns

**Difficulty: Easy** (once the schema loader exists). The cop logic is straightforward — match `ignored_columns` assignments and check column existence.

**Disabled by default** in rubocop-rails. No bench repo enables it, so this will likely never affect conformance. Low priority.

Detects `self.ignored_columns = [:col]` where the column no longer exists in the schema. Stale `ignored_columns` entries should be cleaned up after the migration that removes the column.

**Needs from schema:**
- Table name for the model class
- Whether each referenced column exists in the table definition

**Edge cases:**
- Both `=` and `+=` assignment forms
- Both symbol and string column names
- Non-literal arrays (variable reference) → skip
- Module context (not a class) → skip

**Stub:** `src/cop/rails/unused_ignored_columns.rs` (empty `check_node`)

## Prerequisite: Schema Loader

**Difficulty: Medium.** This is the blocking prerequisite for both cops. Must be built first.

Both cops depend on the same schema infrastructure. The work breaks down into:

### 1. Schema parser

Parse `db/schema.rb` and extract structured data. The file uses a small DSL:

```ruby
ActiveRecord::Schema.define(version: 2024_01_01) do
  create_table "users" do |t|
    t.string "email", null: false
    t.string "name"
    t.index ["email"], unique: true
  end

  add_index "users", ["name"], unique: false
end
```

We need to extract:
- **Tables** — name, columns (name + type + null), inline indexes
- **Indexes** — columns or expression, unique flag
- **add_index calls** — table name + index info

This is a subset of Ruby that Prism can parse. Walk the AST for `create_table` blocks and `add_index` calls.

### 2. Table name resolution

Derive table name from ActiveRecord model class name:
- `User` → `users` (underscore + pluralize)
- `Admin::User` → `users` (last segment)
- Explicit `self.table_name = "custom_table"` overrides

Rails pluralization is complex, but a basic `s` suffix + common irregular forms covers the majority of real-world cases. RuboCop uses the same simplified approach (it doesn't load ActiveSupport's inflector).

### 3. Integration with linter

Schema data is per-project (not per-file), so it should be:
- Loaded once during config resolution
- Passed to cops that need it (new field on cop context or a shared reference)
- Optional — if `db/schema.rb` doesn't exist, these cops become no-ops

### 4. Wire up the cops

- Uncomment registrations in `src/cop/rails/mod.rs`
- Implement `check_node` using the schema data
- Add test fixtures using `# nitrocop-schema:` directive or similar mechanism to provide inline schema for tests

## Suggested order

1. **Schema loader** (prerequisite) — new module `src/schema.rs`, parse with Prism, unit test against real `db/schema.rb` files from bench repos
2. **UniqueValidationWithoutIndex** — higher value since it's enabled by default; test against chatwoot
3. **UnusedIgnoredColumns** — low priority since disabled by default and no conformance impact

---

# Excluded Cops in Conformance

Cops excluded from per-repo conformance comparison via `per_repo_excluded_cops()` in `bench/bench.rs`. Each exclusion represents a known divergence between nitrocop and RuboCop that isn't a nitrocop bug.

## fat_free_crm

**Cops:** `Style/RedundantRegexpEscape`, `Layout/FirstArrayElementIndentation`, `Layout/MultilineMethodCallIndentation`, `Style/TrailingCommaInHashLiteral`

**Reason:** RuboCop reports 0 offenses on these cops even when run with `--only`, but the source code contains patterns that match the cop specifications. nitrocop correctly flags them. These are RuboCop quirks (likely autocorrect artifacts in RuboCop's cache or parser differences), not nitrocop bugs.

## multi_json

**Cop:** `Style/EmptyClassDefinition`

**Reason:** Config interaction between `require: standard` and `NewCops: enable`.

- Standard gem (v1.54.0) `config/base.yml` sets `Style/EmptyClassDefinition: Enabled: false`
- RuboCop's `config/default.yml` sets `Enabled: pending` (cop added in v1.84)
- multi_json's `.rubocop.yml` sets `NewCops: enable`
- RuboCop's `--show-cops` shows `Enabled: pending` (not `false`), meaning standard's explicit disable is being ignored
- RuboCop fires the cop (1 offense); nitrocop does not

This appears to be a RuboCop quirk where `require:` extensions inject config at a different stage than YAML inheritance. The `NewCops: enable` setting sees the default `pending` state rather than standard's `false`. nitrocop correctly applies the YAML config chain (standard's `false` wins) and disables the cop.

**Resolution options:**
1. Match RuboCop's behavior by treating `NewCops: enable` as promoting `pending` even when an inherited config explicitly sets `Enabled: false` — risky, may cause FPs in other repos
2. Accept as a known RuboCop quirk and keep the exclusion
3. Investigate RuboCop's `require:` config injection order to understand the exact semantics

## Lint/Syntax (all repos with TargetRubyVersion < 3.0)

**Repos:** rubocop (2.7), activeadmin (2.6)

**Reason:** Prism always parses modern Ruby syntax. It cannot detect parser-version-specific syntax errors (e.g., `...` forwarding under Ruby 2.6) that the legacy parser gem would flag. This is a fundamental limitation of using Prism as the parser.

---

# Streaming Progress Output

## Problem

RuboCop prints progress characters (`.`, `C`, `W`, etc.) as each file finishes linting.
nitrocop waits for all files to complete, then dumps the entire progress line at once.
On large repos this means several seconds of silence before any output appears.

## Current Architecture

```
run_linter()                          Formatter::print()
  files.par_iter()                      format_to(diagnostics, files, stdout)
    .flat_map(lint_file)                  build progress string in memory
    .collect::<Vec<Diagnostic>>()         writeln!(out, "{progress}")
  sort diagnostics                        writeln!(out, "{details}")
  return LintResult { diagnostics }       writeln!(out, "{summary}")
```

The linter returns a single `LintResult` containing all diagnostics. The formatter
receives the complete dataset and renders it. There is no communication between the
two during execution.

Key files:
- `src/linter.rs:157` — `run_linter()` uses `par_iter().flat_map().collect()`
- `src/linter.rs:122` — `LintResult { diagnostics, file_count, corrected_count }`
- `src/formatter/mod.rs:14` — `Formatter::format_to(&self, diagnostics, files, out)`
- `src/formatter/progress.rs` — builds entire progress `String` then writes once
- `src/formatter/pacman.rs` — same pattern

## Design

### Channel-based streaming

Replace the `par_iter().collect()` with an `mpsc::channel`. Each rayon worker sends
per-file results through the channel. The main thread consumes the channel, printing
progress characters as results arrive.

```
Main thread                         Rayon workers (N threads)
-----------                         -------------------------
create mpsc::channel()
spawn rayon scope {                 for each file:
                                      lint_file(path)
                                      tx.send(FileResult { path, diagnostics })
}
                                    (tx drops when scope ends)

loop rx.recv():
  print_progress_char(result)
  collect diagnostics

sort diagnostics
print details + summary
```

### New types

```rust
/// Result for a single file, sent through the channel.
struct FileResult {
    path: PathBuf,
    diagnostics: Vec<Diagnostic>,
    corrected_count: usize,
}
```

### Formatter trait changes

Add an optional streaming method with a default no-op implementation:

```rust
pub trait Formatter {
    /// Called once per file as results arrive. Default: no-op.
    fn file_finished(&self, path: &Path, diagnostics: &[Diagnostic], out: &mut dyn Write) {}

    /// Called after all files are done. Existing batch method.
    fn format_to(&self, diagnostics: &[Diagnostic], files: &[PathBuf], out: &mut dyn Write);
}
```

Progress and pacman formatters implement `file_finished()` to print one character.
JSON, text, and other formatters leave it as a no-op (they need the full dataset).

### Progress formatter streaming

```rust
fn file_finished(&self, _path: &Path, diagnostics: &[Diagnostic], out: &mut dyn Write) {
    let ch = if diagnostics.is_empty() {
        '.'
    } else {
        diagnostics.iter().map(|d| d.severity).max().unwrap().letter()
    };
    let _ = write!(out, "{ch}");
    let _ = out.flush();  // force immediate display
}
```

The batch `format_to()` would then skip the progress line (already printed) and
only print the diagnostic details and summary.

### Linter changes (`src/linter.rs`)

The main change is in `run_linter()`. Two options:

**Option A: Channel inside rayon scope**

```rust
pub fn run_linter(..., on_file: &dyn Fn(&Path, &[Diagnostic])) -> LintResult {
    let (tx, rx) = std::sync::mpsc::channel();

    rayon::scope(|s| {
        s.spawn(|_| {
            files.par_iter().for_each(|path| {
                let result = lint_file(path, ...);
                let _ = tx.send((path.clone(), result));
            });
        });
        drop(tx); // not needed — tx moves into spawn, drops when done

        // Main thread: consume results
        for (path, diagnostics) in rx {
            on_file(&path, &diagnostics);
            all_diagnostics.extend(diagnostics);
        }
    });
}
```

**Option B: Callback parameter**

Simpler — pass a closure to `run_linter` that's called per file:

```rust
pub fn run_linter(
    ...,
    on_file: Option<&(dyn Fn(&Path, &[Diagnostic]) + Sync)>,
) -> LintResult {
```

Rayon workers call `on_file` after linting each file. The callback needs `Sync`
because it's called from multiple threads. This means the callback must handle
its own synchronization (e.g., `Mutex<StdoutLock>`).

Option A is cleaner because only the main thread writes to stdout — no mutex needed,
no interleaved output risk.

### stdout considerations

- `write!` + `flush()` per file is necessary for streaming — without flush, libc
  buffers stdout and the characters appear in chunks.
- On a 5000-file repo this is 5000 flush syscalls. Each is ~1μs, so ~5ms total.
  Negligible compared to linting time.
- The progress line needs `\r` or newline handling for terminal width wrapping.
  RuboCop wraps at terminal width with a newline. We should do the same.

### Line wrapping

RuboCop wraps the progress line at the terminal width (default 80 columns). Each
line of dots is followed by a newline, so the output looks like:

```
................................................................................
..........C.....W...............................................................
................
```

Use `terminal_size` crate or `libc::ioctl(TIOCGWINSZ)` to detect width. Fall back
to 80 if detection fails (piped output, no tty).

### Cache integration

No changes needed. The cache check happens inside `lint_file()` — a cache hit
produces diagnostics (usually empty vec) the same as a cache miss. The channel
or callback receives both equally.

### --fail-fast interaction

Currently `run_linter` uses an `AtomicBool` to signal early stop. With the channel
approach, the main thread can signal stop by dropping the receiver or setting the
atomic. Workers check the flag before starting each file (already implemented).

### Formats that don't stream

JSON, text, github, quiet, files, emacs — these all need the complete dataset.
They leave `file_finished()` as a no-op. The main thread still collects all
results, sorts them, and calls `format_to()` at the end. No behavior change
for these formats.

## Implementation Order

1. Add `file_finished()` default method to `Formatter` trait
2. Change `run_linter` to use `mpsc::channel` inside a `rayon::scope`
3. Wire the channel consumer to call `formatter.file_finished()`
4. Implement `file_finished()` for `ProgressFormatter` (print severity char + flush)
5. Implement `file_finished()` for `PacmanFormatter` (print dot/ghost + flush)
6. Update `format_to()` for both to skip the progress line (already printed)
7. Add terminal width detection for line wrapping
8. Test: verify progress output matches batch output for all formatters
9. Test: verify `--fail-fast` still works with streaming

## Non-Goals

- Streaming diagnostic details as they arrive (interleaved output from multiple
  files would be unreadable; keep details as a sorted batch at the end)
- Streaming for JSON format (must be a single valid JSON document)
- Progress bar / ETA estimation

## Difficulty Assessment

**Low-medium.** The core idea (channel + consumer loop) is straightforward and
well-understood. The tricky parts are:

1. **Refactoring `run_linter`'s return path.** It currently uses
   `par_iter().flat_map().collect()` — a clean one-liner. Replacing it with a
   `rayon::scope` + channel is more code and changes the function's shape. Every
   caller of `run_linter` needs to adapt. There's only one call site today
   (`src/lib.rs`), but the bench binary also calls linter internals, so check
   for breakage there.

2. **Formatter trait becomes two-phase.** Adding `file_finished()` is easy, but
   `format_to()` now needs to know whether streaming already happened (to avoid
   printing the progress line twice). This means either a flag, splitting into
   separate methods, or having the streaming formatters track state — all slightly
   awkward. The cleanest approach is probably to split `format_to` into
   `format_details()` and `format_summary()` and have the caller orchestrate.

3. **Testing.** The batch formatters are easy to test (in/out). Streaming
   formatters need tests that verify output arrives incrementally, which means
   either inspecting the write buffer between calls or just testing
   `file_finished()` in isolation. Not hard, but more test surface.

4. **Debug timing.** The `--debug` phase timing and `NITROCOP_COP_PROFILE`
   re-run currently live inside `run_linter`. They'll need to coexist with
   the channel approach. The profiler re-runs files single-threaded, so it
   can stay as-is (it doesn't need streaming).

5. **Output ordering.** Rayon doesn't guarantee file order. Progress characters
   will appear in whatever order workers finish, which is fine (RuboCop does
   the same). But it means the progress line won't match the file list order,
   which changes the pacman/progress formatter tests. Not a real problem, just
   something to be aware of.

What's **not** hard: cache integration (no changes), `--fail-fast` (already
uses an atomic flag), non-streaming formatters (no-op default), and the
`flush()` syscall overhead (negligible).

Estimated scope: ~150-200 lines of changes across `linter.rs`, `formatter/mod.rs`,
`formatter/progress.rs`, `formatter/pacman.rs`, and `lib.rs`. A focused session.
