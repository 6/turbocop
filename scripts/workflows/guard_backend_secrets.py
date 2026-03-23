#!/usr/bin/env python3
"""Register masks and scan files for backend secret leakage."""

import argparse
import glob
import json
import os
import sys
from pathlib import Path


def _nonempty_string(value) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _load_secret(var_name: str):
    raw = os.environ.get(var_name, "")
    if not raw.strip():
        raise ValueError(f"{var_name} is missing or empty")

    parsed = None
    try:
        parsed = json.loads(raw)
    except json.JSONDecodeError:
        parsed = None
    return raw, parsed


def _collect_values(
    var_name: str, raw_secret: str, parsed, *, include_raw: bool
) -> list[tuple[str, str]]:
    values = []
    if include_raw or not isinstance(parsed, dict):
        values.append((f"{var_name}:raw", raw_secret))

    if isinstance(parsed, dict):
        api_key = parsed.get("OPENAI_API_KEY")
        if _nonempty_string(api_key):
            values.append((f"{var_name}:openai_api_key", api_key))

        tokens = parsed.get("tokens")
        if isinstance(tokens, dict):
            for key in ("access_token", "refresh_token", "id_token", "account_id"):
                value = tokens.get(key)
                if _nonempty_string(value):
                    values.append((f"{var_name}:{key}", value))

    deduped = []
    seen = set()
    for label, value in values:
        if value in seen:
            continue
        seen.add(value)
        deduped.append((label, value))
    return deduped


def _expand_patterns(patterns: list[str]) -> list[str]:
    expanded = []
    seen = set()
    for pattern in patterns:
        matches = glob.glob(os.path.expanduser(pattern), recursive=True)
        if matches:
            candidates = matches
        else:
            candidates = [os.path.expanduser(pattern)]
        for candidate in candidates:
            if candidate in seen:
                continue
            seen.add(candidate)
            expanded.append(candidate)
    return expanded


def _read_patterns_file(path: str) -> list[str]:
    lines = []
    for raw in Path(path).read_text().splitlines():
        pattern = raw.strip()
        if not pattern or pattern.startswith("#"):
            continue
        lines.append(pattern)
    return lines


def _load_all_secrets(
    var_names: list[str], ignore_missing: bool, *, include_raw: bool
) -> list[tuple[str, str]]:
    values = []
    for var_name in var_names:
        try:
            raw, parsed = _load_secret(var_name)
        except ValueError:
            if ignore_missing:
                continue
            raise
        values.extend(
            _collect_values(var_name, raw, parsed, include_raw=include_raw)
        )
    return values


def emit_masks(var_names: list[str], ignore_missing: bool) -> int:
    secret_values = _load_all_secrets(var_names, ignore_missing, include_raw=False)
    if not secret_values:
        print("No backend secrets found to mask.")
        return 0
    for _, value in secret_values:
        print(f"::add-mask::{value}")
    return 0


def scan_files(var_names: list[str], ignore_missing: bool, patterns: list[str]) -> int:
    secret_values = _load_all_secrets(var_names, ignore_missing, include_raw=True)
    if not secret_values:
        print("No backend secrets found to scan for.")
        return 0

    leaked = []
    for path in _expand_patterns(patterns):
        if not os.path.isfile(path):
            continue
        try:
            with open(path, "r", errors="ignore") as f:
                content = f.read()
        except OSError:
            continue

        for label, value in secret_values:
            if value and value in content:
                leaked.append((path, label))
                break

    if leaked:
        print("ERROR: potential backend secret leakage detected in generated files:", file=sys.stderr)
        for path, label in leaked:
            print(f"  - {path} ({label})", file=sys.stderr)
        return 1

    print("No backend secret leakage detected in generated files.")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--from-env",
        action="append",
        required=True,
        dest="env_vars",
        help="Environment variable holding a secret value; repeat for multiple secrets",
    )
    parser.add_argument(
        "--ignore-missing",
        action="store_true",
        help="Skip missing or empty env vars instead of failing",
    )

    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("emit-masks")

    scan = subparsers.add_parser("scan-files")
    scan.add_argument("patterns", nargs="+", help="File paths or glob patterns to scan")

    scan_manifest = subparsers.add_parser("scan-manifest")
    scan_manifest.add_argument("manifest", help="Path to newline-separated file/glob manifest")

    args = parser.parse_args()

    try:
        if args.command == "emit-masks":
            return emit_masks(args.env_vars, args.ignore_missing)
        if args.command == "scan-files":
            return scan_files(args.env_vars, args.ignore_missing, args.patterns)
        if args.command == "scan-manifest":
            return scan_files(
                args.env_vars,
                args.ignore_missing,
                _read_patterns_file(args.manifest),
            )
    except ValueError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1

    return 1


if __name__ == "__main__":
    raise SystemExit(main())
