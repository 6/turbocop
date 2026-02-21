# Streaming Progress Output

## Problem

RuboCop prints progress characters (`.`, `C`, `W`, etc.) as each file finishes linting.
turbocop waits for all files to complete, then dumps the entire progress line at once.
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

4. **Debug timing.** The `--debug` phase timing and `TURBOCOP_COP_PROFILE`
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
