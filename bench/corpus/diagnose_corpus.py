#!/usr/bin/env python3
"""Pre-compute per-cop diagnosis and embed results in corpus-results.json.

Runs nitrocop on the source context snippets already embedded in
corpus-results.json (from extract_context.py) to classify each FP/FN
as a code bug or config issue.  The result is written back into each
by_cop entry as a "diagnosis" field:

    {"code_bugs": int, "config_issues": int}

This allows cop-issue-sync to read cached diagnosis instead of
re-running the snippet diagnostic at issue-sync time.

Usage (in collect-results job, after diff + merge steps):
    python3 bench/corpus/diagnose_corpus.py \
        --input corpus-results.json \
        --output corpus-results.json \
        --binary bin/nitrocop
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

CORPUS_DIR = Path(__file__).resolve().parent
BASELINE_CONFIG = CORPUS_DIR / "baseline_rubocop.yml"


def extract_diagnostic_lines(src: list[str]) -> tuple[list[str], str | None]:
    """Extract source lines and the offense line from context-annotated source."""
    lines, offense = [], None
    for source_line in src:
        is_offense = source_line.strip().startswith(">>>")
        cleaned = re.sub(r"^(>>>\s*)?\s*\d+:\s?", "", source_line)
        lines.append(cleaned)
        if is_offense:
            offense = cleaned
    return lines, offense


def parse_example_loc(loc: str) -> tuple[str, str, int] | None:
    """Parse 'repo_id: path/to/file.rb:line' into components."""
    if ": " not in loc:
        return None
    repo_id, rest = loc.split(": ", 1)
    last_colon = rest.rfind(":")
    if last_colon < 0:
        return None
    filepath = rest[:last_colon]
    try:
        line = int(rest[last_colon + 1 :])
    except ValueError:
        return None
    return repo_id, filepath, line


def run_nitrocop(
    binary: str, cwd: str, cop: str, filename: str = "test.rb",
) -> list[dict]:
    """Run nitrocop on a file in the given directory, return offenses list."""
    cmd = [binary, "--preview", "--no-cache", "--format", "json"]
    if BASELINE_CONFIG.exists():
        cmd.extend(["--config", str(BASELINE_CONFIG)])
    else:
        cmd.append("--force-default-config")
    cmd.extend(["--only", cop, filename])
    try:
        proc = subprocess.run(
            cmd, capture_output=True, text=True, timeout=30, cwd=cwd,
        )
    except subprocess.TimeoutExpired:
        return []
    if proc.stdout.strip():
        try:
            return json.loads(proc.stdout).get("offenses", [])
        except json.JSONDecodeError:
            pass
    return []


def diagnose_examples(
    binary: str, cop: str, examples: list, kind: str,
) -> tuple[int, int]:
    """Classify examples as code bugs vs config issues.

    Returns (code_bugs, config_issues).
    """
    bugs, config_issues = 0, 0
    for example in examples[:15]:
        if not isinstance(example, dict) or not example.get("src"):
            continue
        lines, offense = extract_diagnostic_lines(example["src"])
        if not lines:
            continue
        loc = example.get("loc", "")
        parsed = parse_example_loc(loc)
        # Preserve the original repo-relative path so that cops with
        # Include patterns (e.g., Rails cops matching **/test/**/*.rb or
        # spec/**/*.rb) can match the file during snippet testing.
        rel_path = parsed[1] if parsed else "test.rb"
        tmp = tempfile.mkdtemp(prefix="nitrocop_diag_")
        filepath = os.path.join(tmp, rel_path)
        os.makedirs(os.path.dirname(filepath), exist_ok=True)
        try:
            with open(filepath, "w") as f:
                f.write("\n".join(lines) + "\n")
            offenses = run_nitrocop(binary, tmp, cop, rel_path)
            if not offenses and offense:
                with open(filepath, "w") as f:
                    f.write(offense + "\n")
                offenses = run_nitrocop(binary, tmp, cop, rel_path)
            detected = len(offenses) > 0
            if (kind == "fn" and not detected) or (kind == "fp" and detected):
                bugs += 1
            else:
                config_issues += 1
        except Exception:
            pass
        finally:
            shutil.rmtree(tmp, ignore_errors=True)
    return bugs, config_issues


def diagnose_cop(binary: str, entry: dict) -> tuple[str, int, int]:
    """Diagnose a single cop entry. Returns (cop_name, code_bugs, config_issues)."""
    cop = entry["cop"]
    fn_bugs, fn_cfg = diagnose_examples(
        binary, cop, entry.get("fn_examples", []), "fn",
    )
    fp_bugs, fp_cfg = diagnose_examples(
        binary, cop, entry.get("fp_examples", []), "fp",
    )
    return cop, fn_bugs + fp_bugs, fn_cfg + fp_cfg


def main():
    parser = argparse.ArgumentParser(
        description="Pre-compute per-cop diagnosis for corpus results",
    )
    parser.add_argument("--input", type=Path, required=True,
                        help="Path to corpus-results.json")
    parser.add_argument("--output", type=Path, required=True,
                        help="Output path (can be same as input)")
    parser.add_argument("--binary", type=str, required=True,
                        help="Path to nitrocop binary")
    parser.add_argument("--workers", type=int, default=8,
                        help="Parallel workers (default: 8)")
    args = parser.parse_args()

    binary = str(Path(args.binary).resolve())
    if not os.path.isfile(binary):
        print(f"Error: binary not found: {args.binary}", file=sys.stderr)
        sys.exit(1)

    data = json.loads(args.input.read_text())
    by_cop = data.get("by_cop", [])

    diverging = [e for e in by_cop if e.get("fp", 0) + e.get("fn", 0) > 0]
    print(f"Diagnosing {len(diverging)} diverging cops...", file=sys.stderr)

    diagnosis: dict[str, tuple[int, int]] = {}
    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        futures = {
            pool.submit(diagnose_cop, binary, entry): entry["cop"]
            for entry in diverging
        }
        for future in as_completed(futures):
            cop_name, code_bugs, cfg_issues = future.result()
            diagnosis[cop_name] = (code_bugs, cfg_issues)

    # Write diagnosis back into by_cop entries
    enriched = 0
    for entry in by_cop:
        cop = entry["cop"]
        if cop in diagnosis:
            code_bugs, cfg_issues = diagnosis[cop]
            entry["diagnosis"] = {
                "code_bugs": code_bugs,
                "config_issues": cfg_issues,
            }
            enriched += 1

    config_only = sum(
        1 for _, (cb, ci) in diagnosis.items() if cb == 0 and ci > 0
    )
    code_bug_cops = sum(1 for _, (cb, _) in diagnosis.items() if cb > 0)
    print(
        f"  {enriched} cops diagnosed: {code_bug_cops} with code bugs, "
        f"{config_only} config-only",
        file=sys.stderr,
    )

    args.output.write_text(json.dumps(data, indent=None) + "\n")
    print(f"Wrote {args.output}", file=sys.stderr)


if __name__ == "__main__":
    main()
