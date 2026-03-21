#!/usr/bin/env python3
"""Watch a Claude Code JSONL session log and print progress updates.

Polls for the latest JSONL file and prints a one-line status summary
every --interval seconds. Designed to run as a background process.

Usage: python3 watch_agent_progress.py --newer-than /tmp/final-task.md [--interval 30]
"""
import argparse
import glob
import json
import os
import sys
import time
from datetime import datetime
from pathlib import Path


LOG_PATTERNS = {
    "minimax": "~/.claude/projects/**/*.jsonl",
    "codex": "~/.codex/sessions/**/*.jsonl",
}


def find_logfile(newer_than: Path, backend: str = "minimax") -> str | None:
    """Find the most recent JSONL file newer than the reference file."""
    ref_mtime = newer_than.stat().st_mtime if newer_than.exists() else 0
    pattern = LOG_PATTERNS.get(backend, LOG_PATTERNS["minimax"])
    candidates = glob.glob(os.path.expanduser(pattern), recursive=True)
    for f in sorted(candidates, key=os.path.getmtime, reverse=True):
        if os.path.getmtime(f) > ref_mtime:
            return f
    return None


def _parse_claude_event(ev: dict, status: dict) -> bool:
    """Parse a Claude Code JSONL event. Returns True if status was updated."""
    status["last_type"] = ev.get("type", "?")
    if ev.get("type") != "assistant":
        return False
    for block in reversed(ev.get("message", {}).get("content", [])):
        if block.get("type") == "tool_use" and not status["last_tool"]:
            status["last_tool"] = block.get("name", "?")
        elif block.get("type") == "text" and not status["last_text"]:
            text = block.get("text", "").strip()
            if text:
                status["last_text"] = text[:200]
    return bool(status["last_tool"] or status["last_text"])


def _parse_codex_event(ev: dict, status: dict) -> bool:
    """Parse a Codex rollout JSONL event. Returns True if status was updated."""
    # Codex events use "event_msg" with a "payload" containing type/content
    payload = ev.get("payload", ev)
    msg_type = payload.get("type", ev.get("type", "?"))
    status["last_type"] = msg_type

    # Assistant messages
    if msg_type in ("assistant", "response.output_item.done"):
        content = payload.get("content", payload.get("item", {}).get("content", []))
        if isinstance(content, str):
            status["last_text"] = content.strip()[:200]
            return True
        if isinstance(content, list):
            for block in reversed(content):
                if isinstance(block, str):
                    status["last_text"] = block.strip()[:200]
                    return True
                btype = block.get("type", "")
                if btype in ("function_call", "tool_use") and not status["last_tool"]:
                    status["last_tool"] = block.get("name", block.get("function", {}).get("name", "?"))
                elif btype in ("text", "output_text") and not status["last_text"]:
                    text = block.get("text", "").strip()
                    if text:
                        status["last_text"] = text[:200]
            return bool(status["last_tool"] or status["last_text"])
    return False


def get_status(logfile: str, backend: str = "minimax") -> dict:
    """Read the last few events and extract status info."""
    status = {
        "events": 0,
        "last_type": "?",
        "last_tool": None,
        "last_text": None,
    }

    try:
        with open(logfile) as f:
            lines = f.readlines()
    except OSError:
        return status

    status["events"] = len(lines)
    parser = _parse_codex_event if backend == "codex" else _parse_claude_event

    # Scan last 10 lines for the most recent useful content
    for line in reversed(lines[-10:]):
        try:
            ev = json.loads(line)
        except json.JSONDecodeError:
            continue

        if parser(ev, status):
            break

    return status


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--newer-than", type=Path, required=True,
        help="Reference file — only consider JSONL files newer than this",
    )
    parser.add_argument(
        "--interval", type=int, default=30,
        help="Seconds between progress updates (default: 30)",
    )
    parser.add_argument(
        "--backend", choices=["minimax", "codex"], default="minimax",
        help="Agent backend (determines log location and format)",
    )
    args = parser.parse_args()

    time.sleep(10)  # initial delay for session to start

    while True:
        now = datetime.now().strftime("%H:%M:%S")
        logfile = find_logfile(args.newer_than, args.backend)

        if logfile:
            s = get_status(logfile, args.backend)
            tool = s["last_tool"] or "n/a"
            text = s["last_text"] or "(none)"
            print(
                f"[progress] {now} | {s['events']} events | "
                f"type: {s['last_type']} | tool: {tool} | text: {text}",
                flush=True,
            )
        else:
            print(f"[progress] {now} | waiting for session to start...", flush=True)

        time.sleep(args.interval)


if __name__ == "__main__":
    main()
