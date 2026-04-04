#!/usr/bin/env python3
"""Audit which style variants have test coverage (fixture or inline).

Scans all cop implementations for `config.get_str("Enforced*", ...)` calls,
looks up each cop's SupportedStyles in vendor config, and checks whether
non-default style values have corresponding fixture files or inline tests.

Usage:
    python3 scripts/audit_style_coverage.py
    python3 scripts/audit_style_coverage.py --department Style
    python3 scripts/audit_style_coverage.py --json
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

# Reuse the vendor config parser from corpus_stress.py
sys.path.insert(0, str(Path(__file__).resolve().parent))
from corpus_stress import VENDOR_CONFIGS, parse_enforced_styles  # noqa: E402

PROJECT_ROOT = Path(__file__).resolve().parent.parent
SRC_DIR = PROJECT_ROOT / "src" / "cop"
FIXTURE_DIR = PROJECT_ROOT / "tests" / "fixtures" / "cops"


def find_enforced_keys_in_source() -> dict[str, set[str]]:
    """Scan cop source files for config.get_str("Enforced*", ...) calls.

    Returns {cop_name: {enforced_key, ...}} for cops that read Enforced* keys.
    """
    result: dict[str, set[str]] = {}
    # Match: get_str("EnforcedFoo", or get_str("EnforcedFoo"
    pattern = re.compile(r'get_str\(\s*"(Enforced\w+)"')

    for rs_file in SRC_DIR.rglob("*.rs"):
        text = rs_file.read_text()
        # Try to find cop name from name() -> &'static str
        name_match = re.search(r'fn\s+name\(\s*&self\s*\)\s*->\s*&\'static\s+str\s*\{\s*"([^"]+)"', text)
        if not name_match:
            continue
        cop_name = name_match.group(1)

        keys = set()
        for m in pattern.finditer(text):
            keys.add(m.group(1))

        if keys:
            result[cop_name] = keys

    return result


def find_fixture_variants(cop_name: str) -> set[str]:
    """Find style variant fixture files for a cop.

    Looks for offense.<variant>.rb / no_offense.<variant>.rb files.
    Returns set of variant names found.
    """
    dept, name = cop_name.split("/", 1)
    # Convert CamelCase to snake_case
    snake_name = re.sub(r"(?<=[a-z0-9])([A-Z])", r"_\1", name).lower()
    fixture_dir = FIXTURE_DIR / dept.lower() / snake_name

    variants: set[str] = set()
    if not fixture_dir.exists():
        return variants

    for f in fixture_dir.iterdir():
        # Match offense.<variant>.rb or no_offense.<variant>.rb
        m = re.match(r"(?:offense|no_offense)\.(\w+)\.rb$", f.name)
        if m:
            variants.add(m.group(1))

    return variants


def find_inline_test_styles(cop_name: str) -> set[str]:
    """Check if a cop's test section has with_config tests for specific styles.

    Searches for string literals that match known style values in test code.
    """
    dept, name = cop_name.split("/", 1)
    snake_name = re.sub(r"(?<=[a-z0-9])([A-Z])", r"_\1", name).lower()

    # Look in the cop source file's #[cfg(test)] section
    for rs_file in SRC_DIR.rglob("*.rs"):
        if rs_file.stem != snake_name:
            continue
        if dept.lower() not in str(rs_file).lower():
            continue
        text = rs_file.read_text()
        # Find the test section
        test_idx = text.find("#[cfg(test)]")
        if test_idx < 0:
            continue
        test_section = text[test_idx:]
        # Find style values mentioned in with_config tests
        styles = set()
        for m in re.finditer(r'"(Enforced\w+)"[^"]*"([^"]+)"', test_section):
            styles.add(m.group(2))
        return styles

    return set()


def audit() -> list[dict]:
    """Run the full audit and return per-cop results."""
    # Parse all vendor configs to get SupportedStyles
    all_vendor_styles: list[dict] = []
    for config_path, _plugin in VENDOR_CONFIGS:
        full_path = str(PROJECT_ROOT / config_path)
        all_vendor_styles.extend(parse_enforced_styles(full_path))

    # Build lookup: {cop_name: {enforced_key: {default, alternatives}}}
    vendor_lookup: dict[str, dict[str, dict]] = {}
    for s in all_vendor_styles:
        vendor_lookup.setdefault(s["cop"], {})[s["key"]] = {
            "default": s["default"],
            "alternatives": s["alternatives"],
        }

    # Find which cops actually read Enforced* keys in their source
    source_keys = find_enforced_keys_in_source()

    results = []
    for cop_name in sorted(set(vendor_lookup.keys()) | set(source_keys.keys())):
        vendor_keys = vendor_lookup.get(cop_name, {})
        src_keys = source_keys.get(cop_name, set())

        fixture_variants = find_fixture_variants(cop_name)
        inline_styles = find_inline_test_styles(cop_name)

        for key_name in sorted(set(vendor_keys.keys()) | src_keys):
            info = vendor_keys.get(key_name, {})
            default_val = info.get("default", "?")
            alternatives = info.get("alternatives", [])

            for alt in alternatives:
                has_fixture = alt in fixture_variants
                has_inline = alt in inline_styles
                tested = has_fixture or has_inline
                results.append({
                    "cop": cop_name,
                    "key": key_name,
                    "default": default_val,
                    "style": alt,
                    "tested": tested,
                    "fixture": has_fixture,
                    "inline": has_inline,
                })

    return results


def main():
    parser = argparse.ArgumentParser(description="Audit style variant test coverage")
    parser.add_argument("--department", help="Filter to a single department (e.g., Style)")
    parser.add_argument("--json", action="store_true", help="Output JSON instead of text")
    parser.add_argument("--untested-only", action="store_true", help="Only show untested variants")
    args = parser.parse_args()

    results = audit()

    if args.department:
        results = [r for r in results if r["cop"].startswith(args.department + "/")]

    if args.untested_only:
        results = [r for r in results if not r["tested"]]

    if args.json:
        print(json.dumps(results, indent=2))
        return

    # Group by cop for readable output
    by_cop: dict[str, list[dict]] = {}
    for r in results:
        by_cop.setdefault(r["cop"], []).append(r)

    total = len(results)
    tested = sum(1 for r in results if r["tested"])
    untested = total - tested

    print(f"Style variant coverage: {tested}/{total} tested, {untested} untested")
    print()

    for cop_name in sorted(by_cop.keys()):
        entries = by_cop[cop_name]
        if args.untested_only and all(e["tested"] for e in entries):
            continue
        print(f"{cop_name}:")
        # Show default as always tested (it's the baseline)
        default_val = entries[0]["default"]
        default_key = entries[0]["key"]
        print(f"  {default_key}: {default_val} (default) \u2713 baseline")
        for e in entries:
            status = "\u2713" if e["tested"] else "\u2717"
            coverage = []
            if e["fixture"]:
                coverage.append("fixture")
            if e["inline"]:
                coverage.append("inline")
            label = " ".join(coverage) if coverage else "NO TEST"
            print(f"  {e['key']}: {e['style']} {status} {label}")
        print()

    # Summary
    print(f"Total: {tested} tested, {untested} untested out of {total} non-default variants")


if __name__ == "__main__":
    main()
