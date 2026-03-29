#!/usr/bin/env python3
"""Validate a Codex auth secret without leaking sensitive token values.

This is intentionally permissive. It validates the fields the current Codex
workflow depends on while allowing the serialized auth.json shape to evolve
across CLI versions.
"""

import argparse
import json
import os
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path


def _fail(msg: str) -> int:
    print(f"ERROR: {msg}", file=sys.stderr)
    return 1


def _warn(msg: str) -> None:
    print(f"WARNING: {msg}", file=sys.stderr)


def _nonempty_string(value) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _parse_timestamp(raw: str) -> datetime:
    if not isinstance(raw, str) or not raw.strip():
        raise ValueError("last_refresh is missing or empty")

    normalized = raw.strip()
    if normalized.endswith("Z"):
        normalized = normalized[:-1] + "+00:00"

    try:
        parsed = datetime.fromisoformat(normalized)
    except ValueError as exc:
        raise ValueError(f"last_refresh is not a valid ISO-8601 timestamp: {raw}") from exc

    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return parsed.astimezone(timezone.utc)


def _load_env(var_name: str):
    raw = os.environ.get(var_name, "")
    if not raw.strip():
        raise ValueError(f"{var_name} is missing or empty")
    try:
        return json.loads(raw)
    except json.JSONDecodeError as exc:
        raise ValueError(f"{var_name} is not valid JSON: {exc}") from exc


def _load_file(path: str):
    file_path = Path(path)
    try:
        raw = file_path.read_text()
    except OSError as exc:
        raise ValueError(f"{path} is unreadable: {exc.strerror}") from exc

    if not raw.strip():
        raise ValueError(f"{path} is missing or empty")

    try:
        return json.loads(raw)
    except json.JSONDecodeError as exc:
        raise ValueError(f"{path} is not valid JSON: {exc}") from exc


def validate_auth(data: dict, max_age_days: int) -> str:
    if not isinstance(data, dict):
        raise ValueError("auth payload must be a JSON object")

    api_key = data.get("OPENAI_API_KEY")
    tokens = data.get("tokens")
    last_refresh = data.get("last_refresh")

    if _nonempty_string(api_key):
        if last_refresh is not None and not isinstance(last_refresh, str):
            _warn("last_refresh is present but not a string")
        return "api_key"

    if not isinstance(tokens, dict):
        raise ValueError("expected tokens object for managed ChatGPT auth")

    access_token = tokens.get("access_token")
    refresh_token = tokens.get("refresh_token")
    account_id = tokens.get("account_id")

    if not _nonempty_string(access_token):
        raise ValueError("tokens.access_token is missing or empty")
    if not _nonempty_string(refresh_token):
        raise ValueError("tokens.refresh_token is missing or empty")
    if not _nonempty_string(account_id):
        _warn("tokens.account_id is missing or empty")

    refreshed_at = _parse_timestamp(last_refresh)
    now = datetime.now(timezone.utc)
    age = now - refreshed_at
    max_age = timedelta(days=max_age_days)
    if age > max_age:
        age_days = age.total_seconds() / 86400
        raise ValueError(
            f"last_refresh is stale ({age_days:.1f} days old; limit is {max_age_days} days)"
        )

    return "chatgpt"


def validate_newer_last_refresh(current: dict, previous: dict) -> None:
    current_refreshed_at = _parse_timestamp(current.get("last_refresh"))
    previous_refreshed_at = _parse_timestamp(previous.get("last_refresh"))
    if current_refreshed_at <= previous_refreshed_at:
        raise ValueError(
            "last_refresh did not advance "
            f"(before={previous.get('last_refresh')}, after={current.get('last_refresh')})"
        )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--from-env",
        default=None,
        help="Environment variable holding the auth JSON (default: CODEX_AUTH_JSON)",
    )
    parser.add_argument(
        "--from-file",
        help="Path to an auth JSON file to validate",
    )
    parser.add_argument(
        "--newer-than-file",
        help="Require last_refresh to be newer than the auth JSON in this file",
    )
    parser.add_argument(
        "--max-age-days",
        type=int,
        default=7,
        help="Maximum allowed age for managed-auth last_refresh (default: 7 days)",
    )
    args = parser.parse_args()

    if args.from_env is not None and args.from_file:
        parser.error("--from-env and --from-file are mutually exclusive")

    from_env = args.from_env or "CODEX_AUTH_JSON"

    try:
        if args.from_file:
            data = _load_file(args.from_file)
        else:
            data = _load_env(from_env)
        mode = validate_auth(data, args.max_age_days)
        if args.newer_than_file:
            previous = _load_file(args.newer_than_file)
            validate_newer_last_refresh(data, previous)
    except ValueError as exc:
        return _fail(str(exc))

    if mode == "api_key":
        print("Codex auth secret validated: API key auth payload")
    else:
        account_id = data.get("tokens", {}).get("account_id", "")
        last_refresh = data.get("last_refresh", "(missing)")
        account_status = "account_id present" if _nonempty_string(account_id) else "account_id missing"
        print(
            "Codex auth secret validated: managed auth payload "
            f"({account_status}, last_refresh={last_refresh})"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
