#!/usr/bin/env python3
"""Tests for guard_backend_secrets.py."""
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).parents[3] / "scripts" / "workflows" / "guard_backend_secrets.py"


def managed_auth_payload():
    return {
        "OPENAI_API_KEY": None,
        "tokens": {
            "access_token": "eyJ-access",
            "refresh_token": "rt-refresh",
            "id_token": "eyJ-id",
            "account_id": "e7-account",
        },
        "last_refresh": "2026-03-22T00:00:00Z",
    }


def run(args, env_vars=None):
    env = os.environ.copy()
    env.pop("CODEX_AUTH_JSON", None)
    env.pop("MINIMAX_API_KEY", None)
    env.pop("ANTHROPIC_API_KEY", None)
    if env_vars:
        env.update(env_vars)
    return subprocess.run(
        [sys.executable, str(SCRIPT), *args],
        capture_output=True,
        text=True,
        env=env,
    )


def test_emit_masks_outputs_commands_for_codex_auth():
    payload = managed_auth_payload()
    result = run(
        ["--from-env", "CODEX_AUTH_JSON", "emit-masks"],
        {"CODEX_AUTH_JSON": json.dumps(payload)},
    )
    assert result.returncode == 0
    assert "::add-mask::eyJ-access" in result.stdout
    assert "::add-mask::rt-refresh" in result.stdout
    assert '"access_token": "eyJ-access"' not in result.stdout
    assert json.dumps(payload) not in result.stdout


def test_emit_masks_outputs_commands_for_api_key():
    result = run(
        ["--from-env", "MINIMAX_API_KEY", "emit-masks"],
        {"MINIMAX_API_KEY": "mm-secret-key"},
    )
    assert result.returncode == 0
    assert "::add-mask::mm-secret-key" in result.stdout


def test_scan_files_passes_when_clean():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".log", delete=False) as f:
        f.write("all clear")
        f.flush()
        result = run(
            ["--from-env", "CODEX_AUTH_JSON", "scan-files", f.name],
            {"CODEX_AUTH_JSON": json.dumps(managed_auth_payload())},
        )
    assert result.returncode == 0
    assert "No backend secret leakage" in result.stdout


def test_scan_files_fails_on_codex_leak():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".log", delete=False) as f:
        f.write("oops rt-refresh leaked")
        f.flush()
        result = run(
            ["--from-env", "CODEX_AUTH_JSON", "scan-files", f.name],
            {"CODEX_AUTH_JSON": json.dumps(managed_auth_payload())},
        )
    assert result.returncode != 0
    assert "potential backend secret leakage" in result.stderr
    assert "CODEX_AUTH_JSON:refresh_token" in result.stderr


def test_scan_files_fails_on_api_key_leak():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".log", delete=False) as f:
        f.write("oops mm-secret-key leaked")
        f.flush()
        result = run(
            ["--from-env", "MINIMAX_API_KEY", "scan-files", f.name],
            {"MINIMAX_API_KEY": "mm-secret-key"},
        )
    assert result.returncode != 0
    assert "MINIMAX_API_KEY:raw" in result.stderr


def test_scan_manifest_reads_patterns_from_file():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".log", delete=False) as f:
        f.write("all clear")
        f.flush()
        manifest = Path(tempfile.mkdtemp()) / "paths.txt"
        manifest.write_text(f"{f.name}\n")
        result = run(
            ["--from-env", "MINIMAX_API_KEY", "scan-manifest", str(manifest)],
            {"MINIMAX_API_KEY": "mm-secret-key"},
        )
    assert result.returncode == 0
    assert "No backend secret leakage" in result.stdout


def test_ignore_missing_skips_absent_vars():
    result = run(
        [
            "--ignore-missing",
            "--from-env",
            "MINIMAX_API_KEY",
            "--from-env",
            "CODEX_AUTH_JSON",
            "emit-masks",
        ],
        {"MINIMAX_API_KEY": "mm-secret-key"},
    )
    assert result.returncode == 0
    assert "::add-mask::mm-secret-key" in result.stdout


if __name__ == "__main__":
    test_emit_masks_outputs_commands_for_codex_auth()
    test_emit_masks_outputs_commands_for_api_key()
    test_scan_files_passes_when_clean()
    test_scan_files_fails_on_codex_leak()
    test_scan_files_fails_on_api_key_leak()
    test_scan_manifest_reads_patterns_from_file()
    test_ignore_missing_skips_absent_vars()
    print("All tests passed.")
