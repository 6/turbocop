#!/usr/bin/env python3
"""Exhaustive per-cop style-variant validation against the corpus.

For a given cop, discovers ALL SupportedStyles from vendor config and runs
`check_cop.py --style <param>=<value>` for each. Reports per-style conformance.

Usage:
    # Check all styles for one cop
    python3 scripts/check_cop_styles.py Style/TrailingCommaInHashLiteral --sample 50

    # Check all cops in a department
    python3 scripts/check_cop_styles.py --department Layout --sample 30

    # Check every configurable cop (slow)
    python3 scripts/check_cop_styles.py --all --sample 30
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from corpus_stress import VENDOR_CONFIGS, parse_enforced_styles  # noqa: E402

PROJECT_ROOT = Path(__file__).resolve().parent.parent


def get_all_styles() -> dict[str, list[dict]]:
    """Parse all vendor configs and return {cop: [{key, default, alternatives}]}.

    Only includes cops that have at least one alternative style value.
    """
    all_styles: list[dict] = []
    for config_path, _plugin in VENDOR_CONFIGS:
        full_path = str(PROJECT_ROOT / config_path)
        all_styles.extend(parse_enforced_styles(full_path))

    by_cop: dict[str, list[dict]] = {}
    for s in all_styles:
        by_cop.setdefault(s["cop"], []).append(s)
    return by_cop


def run_check_cop_style(
    cop: str, param: str, value: str, *, sample: int | None = None,
    clone: bool = False, rerun: bool = True,
) -> dict:
    """Run check_cop.py --style for a single (cop, param, value).

    Returns dict with keys: param, value, passed (bool), output (str).
    """
    cmd = [
        sys.executable, str(PROJECT_ROOT / "scripts" / "check_cop.py"),
        cop, "--style", f"{param}={value}", "--rerun",
    ]
    if sample is not None:
        cmd += ["--sample", str(sample)]
    if clone:
        cmd.append("--clone")

    try:
        result = subprocess.run(
            cmd, capture_output=True, text=True, timeout=600,
            cwd=str(PROJECT_ROOT),
        )
        passed = result.returncode == 0
        output = result.stdout + result.stderr
    except subprocess.TimeoutExpired:
        passed = False
        output = "TIMEOUT after 600s"

    return {
        "param": param,
        "value": value,
        "passed": passed,
        "output": output,
    }


def check_cop_all_styles(
    cop: str,
    styles: list[dict],
    *,
    sample: int | None = None,
    clone: bool = False,
) -> list[dict]:
    """Check all style variants for a cop. Returns list of per-style results."""
    results = []
    for style in styles:
        param = style["key"]
        all_values = [style["default"]] + style["alternatives"]
        for value in all_values:
            print(f"  {param}={value}...", end=" ", flush=True, file=sys.stderr)
            r = run_check_cop_style(cop, param, value, sample=sample, clone=clone)
            status = "PASS" if r["passed"] else "FAIL"
            print(status, file=sys.stderr)
            r["is_default"] = (value == style["default"])
            results.append(r)
    return results


def main():
    parser = argparse.ArgumentParser(
        description="Exhaustive per-cop style-variant validation")
    parser.add_argument("cop", nargs="?", help="Cop name (e.g., Style/TrailingCommaInHashLiteral)")
    parser.add_argument("--department", help="Check all cops in a department (e.g., Layout)")
    parser.add_argument("--all", action="store_true", help="Check every configurable cop")
    parser.add_argument("--sample", type=int, default=30,
                        help="Cap repos per style check (default: 30)")
    parser.add_argument("--clone", action="store_true",
                        help="Auto-clone repos (passes --clone to check_cop.py)")
    parser.add_argument("--json", action="store_true", help="Output JSON results")
    args = parser.parse_args()

    if not args.cop and not args.department and not args.all:
        parser.error("Specify a cop name, --department, or --all")

    all_styles = get_all_styles()

    # Determine which cops to check
    cops_to_check: list[str] = []
    if args.cop:
        if args.cop not in all_styles:
            print(f"Cop '{args.cop}' has no configurable styles in vendor config.",
                  file=sys.stderr)
            sys.exit(1)
        cops_to_check = [args.cop]
    elif args.department:
        cops_to_check = sorted(
            c for c in all_styles if c.startswith(args.department + "/")
        )
        if not cops_to_check:
            print(f"No configurable cops found in department '{args.department}'",
                  file=sys.stderr)
            sys.exit(1)
    elif args.all:
        cops_to_check = sorted(all_styles.keys())

    print(f"Checking {len(cops_to_check)} cops, sample={args.sample}", file=sys.stderr)

    all_results: dict[str, list[dict]] = {}
    total_pass = 0
    total_fail = 0

    for cop in cops_to_check:
        styles = all_styles[cop]
        total_variants = sum(1 + len(s["alternatives"]) for s in styles)
        print(f"\n{cop} ({total_variants} styles):", file=sys.stderr)

        results = check_cop_all_styles(
            cop, styles, sample=args.sample, clone=args.clone,
        )
        all_results[cop] = results
        passes = sum(1 for r in results if r["passed"])
        fails = sum(1 for r in results if not r["passed"])
        total_pass += passes
        total_fail += fails

    if args.json:
        print(json.dumps(all_results, indent=2))
    else:
        # Print summary table
        print(f"\n{'='*70}")
        print(f"Results: {total_pass} pass, {total_fail} fail")
        print(f"{'='*70}\n")

        for cop in cops_to_check:
            results = all_results[cop]
            cop_fails = [r for r in results if not r["passed"]]
            if cop_fails:
                print(f"{cop}: {len(cop_fails)} FAILURES")
                for r in cop_fails:
                    default_tag = " (default)" if r.get("is_default") else ""
                    print(f"  FAIL: {r['param']}={r['value']}{default_tag}")
            else:
                print(f"{cop}: all {len(results)} styles pass")

    if total_fail > 0:
        sys.exit(1)


if __name__ == "__main__":
    main()
