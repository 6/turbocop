#!/usr/bin/env python3
"""Tests for validate_codex_auth.py."""
import json
import os
import subprocess
import sys
import tempfile
from datetime import datetime, timedelta, timezone
from pathlib import Path

SCRIPT = Path(__file__).parents[3] / "scripts" / "workflows" / "validate_codex_auth.py"


def iso_days_ago(days: int) -> str:
    return (
        datetime.now(timezone.utc) - timedelta(days=days)
    ).isoformat().replace("+00:00", "Z")


def run(payload=None, max_age_days: int = 7):
    env = os.environ.copy()
    if payload is None:
        env.pop("CODEX_AUTH_JSON", None)
    else:
        env["CODEX_AUTH_JSON"] = json.dumps(payload)
    return subprocess.run(
        [sys.executable, str(SCRIPT), "--max-age-days", str(max_age_days)],
        capture_output=True,
        text=True,
        env=env,
    )


def run_from_file(tmp_path: Path, payload, max_age_days: int = 7):
    auth_path = tmp_path / "auth.json"
    auth_path.write_text(json.dumps(payload))
    return subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            "--from-file",
            str(auth_path),
            "--max-age-days",
            str(max_age_days),
        ],
        capture_output=True,
        text=True,
        env=os.environ.copy(),
    )


def test_accepts_managed_auth():
    result = run({
        "OPENAI_API_KEY": None,
        "tokens": {
            "access_token": "eyJ-access",
            "refresh_token": "rt-refresh",
            "id_token": "eyJ-id",
            "account_id": "e7-account",
        },
        "last_refresh": iso_days_ago(1),
    })
    assert result.returncode == 0
    assert "managed auth payload" in result.stdout
    assert "account_id present" in result.stdout


def test_accepts_api_key_auth():
    result = run({
        "OPENAI_API_KEY": "sk-test",
        "tokens": None,
        "last_refresh": None,
    })
    assert result.returncode == 0
    assert "API key auth payload" in result.stdout


def test_accepts_auth_from_file(tmp_path: Path):
    result = run_from_file(
        tmp_path,
        {
            "OPENAI_API_KEY": None,
            "tokens": {
                "access_token": "eyJ-access",
                "refresh_token": "rt-refresh",
                "id_token": "eyJ-id",
                "account_id": "e7-account",
            },
            "last_refresh": iso_days_ago(1),
        },
    )
    assert result.returncode == 0
    assert "managed auth payload" in result.stdout


def test_accepts_newer_last_refresh_than_previous_file(tmp_path: Path):
    previous = tmp_path / "previous.json"
    current = tmp_path / "current.json"
    previous.write_text(json.dumps({
        "OPENAI_API_KEY": None,
        "tokens": {
            "access_token": "eyJ-access",
            "refresh_token": "rt-refresh",
            "id_token": "eyJ-id",
            "account_id": "e7-account",
        },
        "last_refresh": iso_days_ago(2),
    }))
    current.write_text(json.dumps({
        "OPENAI_API_KEY": None,
        "tokens": {
            "access_token": "eyJ-access-new",
            "refresh_token": "rt-refresh-new",
            "id_token": "eyJ-id-new",
            "account_id": "e7-account",
        },
        "last_refresh": iso_days_ago(1),
    }))
    result = subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            "--from-file",
            str(current),
            "--newer-than-file",
            str(previous),
            "--max-age-days",
            "7",
        ],
        capture_output=True,
        text=True,
        env=os.environ.copy(),
    )
    assert result.returncode == 0


def test_rejects_unchanged_last_refresh_vs_previous_file(tmp_path: Path):
    payload = {
        "OPENAI_API_KEY": None,
        "tokens": {
            "access_token": "eyJ-access",
            "refresh_token": "rt-refresh",
            "id_token": "eyJ-id",
            "account_id": "e7-account",
        },
        "last_refresh": iso_days_ago(1),
    }
    previous = tmp_path / "previous.json"
    current = tmp_path / "current.json"
    previous.write_text(json.dumps(payload))
    current.write_text(json.dumps(payload))
    result = subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            "--from-file",
            str(current),
            "--newer-than-file",
            str(previous),
            "--max-age-days",
            "7",
        ],
        capture_output=True,
        text=True,
        env=os.environ.copy(),
    )
    assert result.returncode != 0
    assert "last_refresh did not advance" in result.stderr


def test_rejects_missing_secret():
    result = run(None)
    assert result.returncode != 0
    assert "missing or empty" in result.stderr


def test_rejects_invalid_shape():
    result = run({
        "OPENAI_API_KEY": None,
        "tokens": {
            "access_token": "",
            "refresh_token": "rt-refresh",
        },
    })
    assert result.returncode != 0
    assert "tokens.access_token is missing or empty" in result.stderr


def test_warns_on_missing_account_id():
    result = run({
        "OPENAI_API_KEY": None,
        "tokens": {
            "access_token": "eyJ-access",
            "refresh_token": "rt-refresh",
        },
        "last_refresh": iso_days_ago(1),
    })
    assert result.returncode == 0
    assert "WARNING: tokens.account_id is missing or empty" in result.stderr


def test_rejects_missing_last_refresh():
    result = run({
        "OPENAI_API_KEY": None,
        "tokens": {
            "access_token": "eyJ-access",
            "refresh_token": "rt-refresh",
            "account_id": "e7-account",
        },
    })
    assert result.returncode != 0
    assert "last_refresh is missing or empty" in result.stderr


def test_rejects_stale_last_refresh():
    result = run({
        "OPENAI_API_KEY": None,
        "tokens": {
            "access_token": "eyJ-access",
            "refresh_token": "rt-refresh",
            "account_id": "e7-account",
        },
        "last_refresh": iso_days_ago(9),
    })
    assert result.returncode != 0
    assert "last_refresh is stale" in result.stderr


if __name__ == "__main__":
    test_accepts_managed_auth()
    test_accepts_api_key_auth()
    with tempfile.TemporaryDirectory() as tmpdir:
        test_accepts_auth_from_file(Path(tmpdir))
    with tempfile.TemporaryDirectory() as tmpdir:
        test_accepts_newer_last_refresh_than_previous_file(Path(tmpdir))
    with tempfile.TemporaryDirectory() as tmpdir:
        test_rejects_unchanged_last_refresh_vs_previous_file(Path(tmpdir))
    test_rejects_missing_secret()
    test_rejects_invalid_shape()
    test_warns_on_missing_account_id()
    test_rejects_missing_last_refresh()
    test_rejects_stale_last_refresh()
    print("All tests passed.")
