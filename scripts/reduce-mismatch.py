#!/usr/bin/env python3
"""Delta reducer for corpus mismatches.

Takes a cop + corpus file with a known FP/FN and automatically shrinks it to
a minimal reproduction using delta debugging (block deletion + line deletion).

Usage:
    python3 scripts/reduce-mismatch.py Style/SymbolProc repo_id app/models/user.rb:42
    python3 scripts/reduce-mismatch.py Style/SymbolProc repo_id app/models/user.rb:42 --type fn
    python3 scripts/reduce-mismatch.py Style/SymbolProc repo_id app/models/user.rb:42 --verbose
"""

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
CORPUS_DIR = PROJECT_ROOT / "vendor" / "corpus"
NITROCOP_BIN = PROJECT_ROOT / "target" / "release" / "nitrocop"
BASELINE_CONFIG = PROJECT_ROOT / "bench" / "corpus" / "baseline_rubocop.yml"
OUTPUT_DIR = Path("/tmp/nitrocop-reduce")

# Counters for stats
_predicate_calls = 0


def corpus_env() -> dict[str, str]:
    """Environment variables for corpus runs, matching CI exactly."""
    env = os.environ.copy()
    env["BUNDLE_GEMFILE"] = str(PROJECT_ROOT / "bench" / "corpus" / "Gemfile")
    env["BUNDLE_PATH"] = str(PROJECT_ROOT / "bench" / "corpus" / "vendor" / "bundle")
    return env


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


def run_rubocop(cop: str, filepath: str) -> set[int]:
    """Run rubocop --only on a single file, return offense line numbers."""
    cmd = [
        "bundle", "exec", "rubocop",
        "--only", cop,
        "--format", "json",
        "--config", str(BASELINE_CONFIG),
        "--force-exclusion",
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
    for f in data.get("files", []):
        for o in f.get("offenses", []):
            if o.get("cop_name", "") == cop:
                lines.add(o.get("location", {}).get("line", 0))
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
    skip_rubocop: bool = False,
    verbose: bool = False,
) -> bool:
    """Check if the file at tmp_path still exhibits the mismatch.

    For FP: nitrocop fires AND rubocop doesn't.
    For FN: rubocop fires AND nitrocop doesn't.

    skip_rubocop: optimization for FP — if original had 0 rubocop offenses,
    any subset will too, so we only need to check nitrocop.
    """
    global _predicate_calls
    _predicate_calls += 1

    if not is_parseable(tmp_path):
        if verbose:
            print("    [skip] not parseable", file=sys.stderr)
        return False

    nc = run_nitrocop(cop, tmp_path)

    if mismatch_type == "fp":
        if not nc:
            if verbose:
                print("    [skip] nitrocop didn't fire", file=sys.stderr)
            return False
        if skip_rubocop:
            return True
        rc = run_rubocop(cop, tmp_path)
        interesting = len(rc) == 0
        if verbose and not interesting:
            print("    [skip] rubocop also fires", file=sys.stderr)
        return interesting
    else:  # fn
        if nc:
            if verbose:
                print("    [skip] nitrocop fires (need it silent for FN)", file=sys.stderr)
            return False
        rc = run_rubocop(cop, tmp_path)
        interesting = len(rc) > 0
        if verbose and not interesting:
            print("    [skip] rubocop also silent", file=sys.stderr)
        return interesting


def write_candidate(lines: list[str], tmp_path: str):
    """Write candidate lines to the temp file."""
    Path(tmp_path).write_text("\n".join(lines) + "\n" if lines else "")


def reduce_blocks(
    lines: list[str],
    cop: str,
    tmp_path: str,
    mismatch_type: str,
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

            write_candidate(candidate, tmp_path)
            if is_interesting(cop, tmp_path, mismatch_type, skip_rubocop, verbose):
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

        write_candidate(candidate, tmp_path)
        if is_interesting(cop, tmp_path, mismatch_type, skip_rubocop, verbose):
            if verbose:
                print(f"  Phase 2: accepted! removed line {i+1}",
                      file=sys.stderr)
            lines = candidate
        i -= 1

    return lines


def main():
    parser = argparse.ArgumentParser(
        description="Delta reducer for corpus mismatches — shrinks files to minimal reproductions")
    parser.add_argument("cop", help="Cop name (e.g., Style/SymbolProc)")
    parser.add_argument("repo_id", help="Corpus repo ID (e.g., mastodon__mastodon__c1f398a)")
    parser.add_argument("location", help="filepath:line (e.g., app/models/user.rb:42)")
    parser.add_argument("--type", choices=["fp", "fn"], default="fp",
                        help="Mismatch type to preserve (default: fp)")
    parser.add_argument("--verbose", action="store_true",
                        help="Print each reduction step")
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

    # Write original and verify the mismatch exists
    write_candidate(lines, tmp_path)
    print("Verifying initial mismatch...", file=sys.stderr)

    nc_lines = run_nitrocop(args.cop, tmp_path)
    rc_lines = run_rubocop(args.cop, tmp_path)

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

    # Phase 1: block deletion
    print("Phase 1: block deletion...", file=sys.stderr)
    lines = reduce_blocks(lines, args.cop, tmp_path, args.type, skip_rubocop, args.verbose)
    print(f"Phase 1 done: {original_count} → {len(lines)} lines", file=sys.stderr)

    # Phase 2: line deletion
    print("Phase 2: line deletion...", file=sys.stderr)
    lines = reduce_lines(lines, args.cop, tmp_path, args.type, skip_rubocop, args.verbose)
    print(f"Phase 2 done: {len(lines)} lines", file=sys.stderr)

    elapsed = time.time() - start_time

    # Write final result
    cop_safe = args.cop.replace("/", "_")
    output_path = OUTPUT_DIR / f"{cop_safe}_reduced.rb"
    write_candidate(lines, str(output_path))

    # Also write final to tmp_path for verification
    write_candidate(lines, tmp_path)

    print()
    print(f"Reduced {original_count} lines → {len(lines)} lines "
          f"({_predicate_calls} checks, {elapsed:.1f}s)")
    print(f"Wrote: {output_path}")
    print()
    print("--- Reduced file ---")
    for i, line in enumerate(lines, 1):
        print(f"  {i:>4}: {line}")


if __name__ == "__main__":
    main()
