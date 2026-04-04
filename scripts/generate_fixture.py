#!/usr/bin/env python3
"""Generate annotated offense fixture files using RuboCop as the source of truth.

Takes raw Ruby code (without annotations) and a cop name, runs RuboCop,
and outputs the code with `^` annotations matching RuboCop's offenses.

Usage:
    # Generate offense fixture from raw Ruby code:
    python3 scripts/generate_fixture.py Style/AndOr input.rb

    # With a non-default style:
    python3 scripts/generate_fixture.py Style/AndOr input.rb \
        --style EnforcedStyle=always

    # Write directly to fixture path:
    python3 scripts/generate_fixture.py Style/AndOr input.rb \
        --style EnforcedStyle=always \
        --output tests/fixtures/cops/style/and_or/offense.always.rb

    # Verify a fixture: check that nitrocop agrees with the annotations:
    python3 scripts/generate_fixture.py Style/AndOr \
        tests/fixtures/cops/style/and_or/offense.rb --verify
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
CORPUS_DIR = PROJECT_ROOT / "bench" / "corpus"


def build_rubocop_config(cop: str, style: dict[str, str] | None = None) -> str:
    """Create a temporary RuboCop config enabling only the given cop.

    Returns path to the temp config file.
    """
    lines = [
        "AllCops:",
        "  DisabledByDefault: true",
        "  TargetRubyVersion: 3.4",
        f"{cop}:",
        "  Enabled: true",
    ]
    if style:
        for key, value in style.items():
            lines.append(f"  {key}: {value}")

    fd, path = tempfile.mkstemp(suffix=".yml", prefix="fixture_gen_")
    with os.fdopen(fd, "w") as f:
        f.write("\n".join(lines) + "\n")
    return path


def run_rubocop(ruby_file: str, cop: str, config_path: str) -> list[dict]:
    """Run RuboCop on a file and return offense locations.

    Returns list of {line, column, length, message} dicts.
    """
    env = os.environ.copy()
    env["BUNDLE_GEMFILE"] = str(CORPUS_DIR / "Gemfile")
    env["BUNDLE_PATH"] = str(CORPUS_DIR / "vendor" / "bundle")

    cmd = [
        "bundle", "exec", "rubocop",
        "--only", cop,
        "--config", config_path,
        "--format", "json",
        "--cache", "false",
        ruby_file,
    ]
    result = subprocess.run(
        cmd, capture_output=True, text=True, timeout=30, env=env,
        cwd=str(CORPUS_DIR),
    )
    try:
        data = json.loads(result.stdout)
    except json.JSONDecodeError:
        print(f"RuboCop failed:\n{result.stderr}", file=sys.stderr)
        sys.exit(1)

    offenses = []
    for f in data.get("files", []):
        for o in f.get("offenses", []):
            loc = o["location"]
            offenses.append({
                "line": loc["line"],
                "column": loc["column"] - 1,  # RuboCop is 1-indexed, we use 0-indexed
                "length": loc.get("length", 1),
                "message": o["message"],
                "cop_name": o["cop_name"],
            })
    return offenses


def annotate_source(source_lines: list[str], offenses: list[dict]) -> list[str]:
    """Insert ^ annotation lines after each source line with offenses."""
    # Group offenses by line
    by_line: dict[int, list[dict]] = {}
    for o in offenses:
        by_line.setdefault(o["line"], []).append(o)

    output = []
    for i, line in enumerate(source_lines):
        output.append(line)
        line_num = i + 1
        if line_num in by_line:
            # Sort by column for consistent output
            for o in sorted(by_line[line_num], key=lambda x: x["column"]):
                carets = "^" * max(1, o["length"])
                padding = " " * o["column"]
                output.append(f"{padding}{carets} {o['cop_name']}: {o['message']}")

    return output


def parse_style_arg(style_str: str) -> dict[str, str]:
    """Parse 'Key=value' into {Key: value}."""
    result = {}
    for part in style_str.split(","):
        if "=" not in part:
            print(f"ERROR: --style must be Key=value, got: {part}", file=sys.stderr)
            sys.exit(1)
        key, value = part.split("=", 1)
        result[key] = value
    return result


def main():
    parser = argparse.ArgumentParser(
        description="Generate annotated fixture files using RuboCop as source of truth")
    parser.add_argument("cop", help="Cop name (e.g., Style/AndOr)")
    parser.add_argument("input", help="Path to raw Ruby file (without annotations)")
    parser.add_argument("--style", type=str, default=None,
                        help="Style override (e.g., EnforcedStyle=always)")
    parser.add_argument("--output", "-o", type=Path, default=None,
                        help="Write output to file instead of stdout")
    parser.add_argument("--verify", action="store_true",
                        help="Verify mode: check that nitrocop agrees with RuboCop")
    args = parser.parse_args()

    input_path = Path(args.input)
    if not input_path.exists():
        print(f"ERROR: {input_path} not found", file=sys.stderr)
        sys.exit(1)

    source = input_path.read_text()
    source_lines = source.splitlines()

    style = parse_style_arg(args.style) if args.style else None
    config_path = build_rubocop_config(args.cop, style)

    try:
        offenses = run_rubocop(str(input_path), args.cop, config_path)
    finally:
        os.unlink(config_path)

    if args.verify:
        print(f"RuboCop found {len(offenses)} offense(s):", file=sys.stderr)
        for o in offenses:
            print(f"  {o['line']}:{o['column']} {o['message']}", file=sys.stderr)
        # TODO: also run nitrocop and compare
        return

    annotated = annotate_source(source_lines, offenses)
    output_text = "\n".join(annotated) + "\n"

    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(output_text)
        print(f"Wrote {len(offenses)} offense(s) to {args.output}", file=sys.stderr)
    else:
        print(output_text, end="")
        print(f"\n# {len(offenses)} offense(s)", file=sys.stderr)


if __name__ == "__main__":
    main()
