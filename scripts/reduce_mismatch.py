#!/usr/bin/env python3
from __future__ import annotations

"""Delta reducer for corpus mismatches.

Takes a cop + corpus file with a known FP/FN and automatically shrinks it to
a minimal reproduction using delta debugging (block deletion + line deletion).

Usage:
    python3 scripts/reduce_mismatch.py Style/SymbolProc repo_id app/models/user.rb:42
    python3 scripts/reduce_mismatch.py Style/SymbolProc repo_id app/models/user.rb:42 --type fn
    python3 scripts/reduce_mismatch.py Style/SymbolProc repo_id app/models/user.rb:42 --verbose
"""

import argparse
import atexit
import hashlib
import json
import os
import signal
import subprocess
import sys
import time
from pathlib import Path

# Allow importing from scripts/shared
sys.path.insert(0, str(Path(__file__).resolve().parent))
from shared.rubocop_cache import cached_rubocop_run

PROJECT_ROOT = Path(__file__).resolve().parent.parent
CORPUS_DIR = PROJECT_ROOT / "vendor" / "corpus"
NITROCOP_BIN = Path(os.environ["NITROCOP_BIN"]) if "NITROCOP_BIN" in os.environ else PROJECT_ROOT / os.environ.get("CARGO_TARGET_DIR", "target") / "release" / "nitrocop"
BASELINE_CONFIG = PROJECT_ROOT / "bench" / "corpus" / "baseline_rubocop.yml"
OUTPUT_DIR = Path("/tmp/nitrocop-reduce")

# Counters for stats
_predicate_calls = 0
_predicate_cache: dict[tuple[str, str, str, bool, str], bool] = {}
_deadline: float = float("inf")


class TimeoutError(Exception):
    """Raised when the total reduction timeout is exceeded."""
    pass


def _check_deadline():
    """Raise TimeoutError if the deadline has passed."""
    if time.time() > _deadline:
        raise TimeoutError("Total reduction timeout exceeded")


def corpus_env() -> dict[str, str]:
    """Environment variables for corpus runs, matching CI exactly."""
    env = os.environ.copy()
    env["BUNDLE_GEMFILE"] = str(PROJECT_ROOT / "bench" / "corpus" / "Gemfile")
    env["BUNDLE_PATH"] = str(PROJECT_ROOT / "bench" / "corpus" / "vendor" / "bundle")
    return env


class RubocopRunner:
    """Run RuboCop without server mode to avoid contention with other processes.

    The reducer runs many RuboCop invocations during bisection. Using --server
    mode caused hangs when multiple agents/worktrees shared the same RuboCop
    server — requests would block waiting for the server lock, causing the
    reducer to appear stalled for 50+ minutes. Using --no-server avoids this
    at the cost of ~1s extra startup per invocation (mitigated by the result
    cache in shared/rubocop_cache.py).
    """

    def __init__(self):
        self.env = corpus_env()

    def _base_cmd(self, cop: str, filepath: str) -> list[str]:
        return [
            "bundle", "exec", "rubocop",
            "--no-server",
            "--only", cop,
            "--format", "json",
            "--config", str(BASELINE_CONFIG),
            "--force-exclusion",
            filepath,
        ]

    def run(self, cop: str, filepath: str) -> set[int]:
        """Run RuboCop on a single file, returning offense line numbers."""
        cmd = self._base_cmd(cop, filepath)

        # Read file content for cache key
        try:
            file_content = Path(filepath).read_text()
        except OSError:
            file_content = ""

        data = cached_rubocop_run(
            cmd=cmd,
            file_content=file_content,
            cop_name=cop,
            config_path=str(BASELINE_CONFIG),
            env=self.env,
            timeout=60,
        )

        if data is None:
            return set()

        lines = set()
        for f in data.get("files", []):
            for o in f.get("offenses", []):
                if o.get("cop_name", "") == cop:
                    lines.add(o.get("location", {}).get("line", 0))
        return lines


def run_nitrocop(cop: str, filepath: str) -> set[int]:
    """Run nitrocop --only on a single file, return offense line numbers."""
    cmd = [
        str(NITROCOP_BIN), "--only", cop, "--preview",
        "--format", "json", "--no-cache",
        "--config", str(BASELINE_CONFIG),
        filepath,
    ]
    try:
        result = subprocess.run(
            cmd, capture_output=True, text=True, timeout=30, env=corpus_env(),
        )
    except subprocess.TimeoutExpired:
        return set()

    if result.returncode not in (0, 1):
        return set()

    try:
        data = json.loads(result.stdout)
    except json.JSONDecodeError:
        return set()

    lines = set()
    for o in data.get("offenses", []):
        if o.get("cop_name", "") == cop:
            lines.add(o.get("line", 0))
    return lines


def is_parseable(filepath: str) -> bool:
    """Check if a Ruby file parses without errors using Prism."""
    cmd = [
        "ruby", "-e",
        "require 'prism'; exit(Prism.parse(File.read(ARGV[0])).errors.empty? ? 0 : 1)",
        filepath,
    ]
    try:
        result = subprocess.run(cmd, capture_output=True, timeout=10)
        return result.returncode == 0
    except subprocess.TimeoutExpired:
        return False


def is_interesting(
    cop: str,
    tmp_path: str,
    mismatch_type: str,
    rubocop_runner: RubocopRunner,
    skip_rubocop: bool = False,
    verbose: bool = False,
    candidate_text: str | None = None,
) -> bool:
    """Check if the file at tmp_path still exhibits the mismatch.

    For FP: nitrocop fires AND rubocop doesn't.
    For FN: rubocop fires AND nitrocop doesn't.

    skip_rubocop: optimization for FP — if original had 0 rubocop offenses,
    any subset will too, so we only need to check nitrocop.
    """
    _check_deadline()

    cache_key = None
    if candidate_text is not None:
        digest = hashlib.blake2b(candidate_text.encode(), digest_size=16).hexdigest()
        cache_key = (cop, tmp_path, mismatch_type, skip_rubocop, digest)
        cached = _predicate_cache.get(cache_key)
        if cached is not None:
            return cached

    global _predicate_calls
    _predicate_calls += 1

    nc = run_nitrocop(cop, tmp_path)

    if mismatch_type == "fp":
        reject_reason = None
        if not nc:
            interesting = False
            reject_reason = "nitrocop didn't fire"
        elif not is_parseable(tmp_path):
            interesting = False
            reject_reason = "not parseable"
        elif skip_rubocop:
            interesting = True
        else:
            rc = rubocop_runner.run(cop, tmp_path)
            interesting = len(rc) == 0
            if not interesting:
                reject_reason = "rubocop also fires"
        if verbose and not interesting:
            print(f"    [skip] {reject_reason}", file=sys.stderr)
    else:  # fn
        if nc:
            if verbose:
                print("    [skip] nitrocop fires (need it silent for FN)", file=sys.stderr)
            interesting = False
        else:
            rc = rubocop_runner.run(cop, tmp_path)
            if not rc:
                if verbose:
                    print("    [skip] rubocop also silent", file=sys.stderr)
                interesting = False
            elif not is_parseable(tmp_path):
                if verbose:
                    print("    [skip] not parseable", file=sys.stderr)
                interesting = False
            else:
                interesting = True

    if cache_key is not None:
        _predicate_cache[cache_key] = interesting
    return interesting


def render_candidate(lines: list[str]) -> str:
    """Render candidate lines back to file contents."""
    return "\n".join(lines) + "\n" if lines else ""


def write_candidate(lines: list[str], tmp_path: str) -> str:
    """Write candidate lines to the temp file."""
    text = render_candidate(lines)
    Path(tmp_path).write_text(text)
    return text


def find_ruby_spans(lines: list[str]) -> list[tuple[int, int, str]]:
    """Find top-level structural spans in Ruby source using indent tracking.

    Returns a list of (start_line_idx, end_line_idx_exclusive, kind) tuples.
    Kinds: 'def', 'block' (do..end at class level), 'class_method'.

    Uses leading whitespace to match `def`/`end` pairs at the same indent.
    Only finds spans at the SECOND indent level (inside a class/module body),
    not top-level or deeply nested ones.
    """
    spans = []
    i = 0
    while i < len(lines):
        stripped = lines[i].lstrip()
        indent = len(lines[i]) - len(stripped)

        # Match `def ` at any indent level
        if stripped.startswith("def "):
            end_idx = _find_matching_end(lines, i, indent)
            if end_idx is not None:
                spans.append((i, end_idx + 1, "def"))
                i = end_idx + 1
                continue

        # Match class-level DSL blocks: `before_action do`, `validate do`, etc.
        # These are method calls followed by `do` that have a matching `end`.
        if stripped.endswith(" do") or stripped.endswith(" do |"):
            # Check for `do |...| ` pattern too
            if " do" in stripped and not stripped.startswith(("class ", "module ", "if ", "unless ",
                                                              "while ", "until ", "for ", "begin",
                                                              "case ")):
                end_idx = _find_matching_end(lines, i, indent)
                if end_idx is not None:
                    spans.append((i, end_idx + 1, "block"))
                    i = end_idx + 1
                    continue

        i += 1

    return spans


def _find_matching_end(lines: list[str], start: int, target_indent: int) -> int | None:
    """Find the matching `end` for a block starting at `start` with `target_indent`.

    Tracks nesting depth: increments on block-opening keywords (def, class,
    module, if, unless, while, until, for, begin, case, do) and decrements on
    every `end`. Returns the line index of the `end` that brings depth to 0.

    Only considers keywords at the START of a line (after whitespace) to avoid
    matching modifier forms like `return if x` or `x unless y`.
    """
    # Keywords that open a new block scope (need matching `end`)
    OPENERS = ("def ", "class ", "module ", "if ", "unless ", "while ", "until ",
               "for ", "begin", "case ")
    depth = 1
    i = start + 1
    while i < len(lines):
        stripped = lines[i].lstrip()

        # Skip empty lines and comments
        if not stripped or stripped.startswith("#"):
            i += 1
            continue

        # Count openers — only when keyword starts the line (not modifier form)
        for opener in OPENERS:
            if stripped.startswith(opener):
                depth += 1
                break
        else:
            # `do` at end of line also opens a block
            if stripped.endswith(" do") or " do |" in stripped:
                depth += 1

        # Every `end` decrements depth (at any indent level)
        if stripped == "end":
            depth -= 1
            if depth == 0:
                return i

        i += 1

    return None


def reduce_structure(
    lines: list[str],
    cop: str,
    tmp_path: str,
    mismatch_type: str,
    rubocop_runner: RubocopRunner,
    skip_rubocop: bool,
    verbose: bool,
) -> list[str]:
    """Phase 0: remove whole methods and blocks (structure-aware).

    Tries removing each structural span (method, block) one at a time,
    largest first. Much more effective than line-level ddmin for files
    with many methods where only one is relevant to the mismatch.
    """
    changed = True
    while changed:
        changed = False
        spans = find_ruby_spans(lines)
        # Sort largest first for maximum reduction per step
        spans.sort(key=lambda s: s[1] - s[0], reverse=True)

        for start, end, kind in spans:
            candidate = lines[:start] + lines[end:]
            if not candidate:
                continue

            if verbose:
                first_line = lines[start].strip()[:60]
                print(f"  Phase 0: trying remove {kind} at lines {start+1}-{end} "
                      f"({end - start} lines, \"{first_line}...\")",
                      file=sys.stderr)

            candidate_text = write_candidate(candidate, tmp_path)
            if is_interesting(
                cop, tmp_path, mismatch_type, rubocop_runner,
                skip_rubocop, verbose, candidate_text,
            ):
                if verbose:
                    print(f"  Phase 0: accepted! {len(lines)} → {len(candidate)} lines",
                          file=sys.stderr)
                lines = candidate
                changed = True
                break  # Re-scan since line numbers shifted

    return lines


def reduce_blocks(
    lines: list[str],
    cop: str,
    tmp_path: str,
    mismatch_type: str,
    rubocop_runner: RubocopRunner,
    skip_rubocop: bool,
    verbose: bool,
) -> list[str]:
    """Phase 1: coarse block deletion (ddmin algorithm)."""
    n = 2
    while n <= len(lines):
        chunk_size = max(1, len(lines) // n)
        reduced = False

        for i in range(n):
            start = i * chunk_size
            end = start + chunk_size if i < n - 1 else len(lines)
            candidate = lines[:start] + lines[end:]

            if not candidate:
                continue

            if verbose:
                print(f"  Phase 1: trying delete chunk {i+1}/{n} "
                      f"(lines {start+1}-{end}, {len(candidate)} remaining)",
                      file=sys.stderr)

            candidate_text = write_candidate(candidate, tmp_path)
            if is_interesting(
                cop, tmp_path, mismatch_type, rubocop_runner,
                skip_rubocop, verbose, candidate_text,
            ):
                if verbose:
                    print(f"  Phase 1: accepted! {len(lines)} → {len(candidate)} lines",
                          file=sys.stderr)
                lines = candidate
                reduced = True
                # Reset n since the file shrank
                n = max(2, n - 1)
                break

        if not reduced:
            n *= 2

    return lines


def reduce_lines(
    lines: list[str],
    cop: str,
    tmp_path: str,
    mismatch_type: str,
    rubocop_runner: RubocopRunner,
    skip_rubocop: bool,
    verbose: bool,
) -> list[str]:
    """Phase 2: fine-grained line deletion (single pass, bottom to top)."""
    i = len(lines) - 1
    while i >= 0:
        candidate = lines[:i] + lines[i + 1:]

        if not candidate:
            i -= 1
            continue

        if verbose:
            print(f"  Phase 2: trying delete line {i+1}/{len(lines)} "
                  f"({len(candidate)} remaining)",
                  file=sys.stderr)

        candidate_text = write_candidate(candidate, tmp_path)
        if is_interesting(
            cop, tmp_path, mismatch_type, rubocop_runner,
            skip_rubocop, verbose, candidate_text,
        ):
            if verbose:
                print(f"  Phase 2: accepted! removed line {i+1}",
                      file=sys.stderr)
            lines = candidate
        i -= 1

    return lines


def _setup_process_group():
    """Create own process group and register cleanup for child processes.

    Only called from main() — never at import time, since that would kill
    the test runner when the module is imported by pytest.
    """
    try:
        os.setpgrp()
    except OSError:
        pass  # May fail if already a process group leader

    def _cleanup_children():
        try:
            os.killpg(os.getpgrp(), signal.SIGTERM)
        except (OSError, ProcessLookupError):
            pass

    atexit.register(_cleanup_children)


def main():
    _setup_process_group()

    parser = argparse.ArgumentParser(
        description="Delta reducer for corpus mismatches — shrinks files to minimal reproductions")
    parser.add_argument("cop", help="Cop name (e.g., Style/SymbolProc)")
    parser.add_argument("repo_id", help="Corpus repo ID (e.g., mastodon__mastodon__c1f398a)")
    parser.add_argument("location", help="filepath:line (e.g., app/models/user.rb:42)")
    parser.add_argument("--type", choices=["fp", "fn"], default="fp",
                        help="Mismatch type to preserve (default: fp)")
    parser.add_argument("--verbose", action="store_true",
                        help="Print each reduction step")
    parser.add_argument("--timeout", type=int, default=600,
                        help="Total timeout in seconds (default: 600 = 10 min)")
    args = parser.parse_args()

    # Parse filepath:line
    last_colon = args.location.rfind(":")
    if last_colon < 0:
        print(f"Error: location must be filepath:line, got '{args.location}'", file=sys.stderr)
        sys.exit(1)
    filepath = args.location[:last_colon]
    try:
        target_line = int(args.location[last_colon + 1:])
    except ValueError:
        print(f"Error: invalid line number in '{args.location}'", file=sys.stderr)
        sys.exit(1)

    # Check prerequisites
    if not NITROCOP_BIN.exists():
        print("Error: release binary not found. Run: cargo build --release", file=sys.stderr)
        sys.exit(1)

    source_path = CORPUS_DIR / args.repo_id / filepath
    if not source_path.exists():
        print(f"Error: source file not found: {source_path}", file=sys.stderr)
        sys.exit(1)

    # Load source
    source = source_path.read_text(errors="replace")
    lines = source.splitlines()
    original_count = len(lines)

    print(f"Reducing {args.cop} {args.type.upper()} in {args.repo_id}/{filepath}:{target_line}")
    print(f"Original: {original_count} lines")
    print()

    # Set up temp file (preserve original filename for path-sensitive cops)
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    tmp_path = str(OUTPUT_DIR / Path(filepath).name)
    rubocop_runner = RubocopRunner()

    # Write original and verify the mismatch exists
    write_candidate(lines, tmp_path)
    print("Verifying initial mismatch...", file=sys.stderr)

    nc_lines = run_nitrocop(args.cop, tmp_path)
    rc_lines = rubocop_runner.run(args.cop, tmp_path)

    if args.type == "fp":
        if not nc_lines:
            print(f"Error: nitrocop doesn't fire on this file for {args.cop}", file=sys.stderr)
            print("Cannot reduce an FP that doesn't exist.", file=sys.stderr)
            sys.exit(1)
        skip_rubocop = len(rc_lines) == 0
        if not skip_rubocop:
            print(f"Warning: rubocop also fires ({len(rc_lines)} offenses). "
                  "Both tools agree — this may not be a true FP.", file=sys.stderr)
            print("Proceeding anyway (will try to find a subset where only nitrocop fires).",
                  file=sys.stderr)
        else:
            print(f"Confirmed FP: nitrocop={len(nc_lines)} offenses, rubocop=0",
                  file=sys.stderr)
            print("Optimization: skipping rubocop checks during reduction (baseline is 0)",
                  file=sys.stderr)
    else:  # fn
        if not rc_lines:
            print(f"Error: rubocop doesn't fire on this file for {args.cop}", file=sys.stderr)
            print("Cannot reduce an FN that doesn't exist.", file=sys.stderr)
            sys.exit(1)
        skip_rubocop = False
        if nc_lines:
            print(f"Warning: nitrocop also fires ({len(nc_lines)} offenses). "
                  "Both tools agree — this may not be a true FN.", file=sys.stderr)
            print("Proceeding anyway (will try to find a subset where only rubocop fires).",
                  file=sys.stderr)
        else:
            print(f"Confirmed FN: rubocop={len(rc_lines)} offenses, nitrocop=0",
                  file=sys.stderr)

    print(file=sys.stderr)
    start_time = time.time()

    global _deadline
    _deadline = start_time + args.timeout
    timed_out = False

    try:
        # Phase 0: structure-aware reduction (remove whole methods/blocks)
        print("Phase 0: structure-aware reduction...", file=sys.stderr)
        lines = reduce_structure(
            lines, args.cop, tmp_path, args.type, rubocop_runner, skip_rubocop, args.verbose,
        )
        print(f"Phase 0 done: {original_count} → {len(lines)} lines", file=sys.stderr)

        # Phase 1: block deletion
        print("Phase 1: block deletion...", file=sys.stderr)
        lines = reduce_blocks(
            lines, args.cop, tmp_path, args.type, rubocop_runner, skip_rubocop, args.verbose,
        )
        print(f"Phase 1 done: {original_count} → {len(lines)} lines", file=sys.stderr)

        # Phase 2: line deletion
        print("Phase 2: line deletion...", file=sys.stderr)
        lines = reduce_lines(
            lines, args.cop, tmp_path, args.type, rubocop_runner, skip_rubocop, args.verbose,
        )
        print(f"Phase 2 done: {len(lines)} lines", file=sys.stderr)
    except TimeoutError:
        timed_out = True
        print(f"\nTimeout after {args.timeout}s — writing best result so far "
              f"({len(lines)} lines)", file=sys.stderr)

    elapsed = time.time() - start_time

    # Write final result
    cop_safe = args.cop.replace("/", "_")
    output_path = OUTPUT_DIR / f"{cop_safe}_reduced.rb"
    write_candidate(lines, str(output_path))

    # Also write final to tmp_path for verification
    write_candidate(lines, tmp_path)

    print()
    status = " (TIMED OUT — partial result)" if timed_out else ""
    print(f"Reduced {original_count} lines → {len(lines)} lines "
          f"({_predicate_calls} checks, {elapsed:.1f}s){status}")
    print(f"Wrote: {output_path}")
    print()
    print("--- Reduced file ---")
    for i, line in enumerate(lines, 1):
        print(f"  {i:>4}: {line}")


if __name__ == "__main__":
    main()
