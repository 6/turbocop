#!/usr/bin/env python3
"""Workflow-time agent log tooling.

Subcommands:
- `watch` prints live progress updates from a session log
- `extract` renders an agent conversation excerpt as markdown
- `summarize` normalizes session output into `agent-result.json`
"""
import argparse
import glob
import json
import os
import sys
import time
from datetime import datetime
from pathlib import Path
from typing import Optional

import resolve_backend

LOG_FORMAT_PATTERNS = {
    "claude": "~/.claude/projects/**/*.jsonl",
    "codex": "~/.codex/sessions/**/*.jsonl",
}

CODEX_LOOKBACK_LINES = 50
CODEX_NOISE_TYPES = {
    "?",
    "event_msg",
    "response_item",
    "item.started",
    "item.completed",
    "session_meta",
    "token_count",
    "task_started",
    "task_complete",
    "user_message",
    "reasoning",
    "function_call_output",
    "custom_tool_call_output",
}


def _set_meaningful_type(status: dict, type_name: str) -> None:
    if status["last_type"] in CODEX_NOISE_TYPES:
        status["last_type"] = type_name


def normalize_backend(backend: str) -> str:
    if backend in LOG_FORMAT_PATTERNS:
        return backend
    if backend in resolve_backend.BACKENDS:
        return resolve_backend.resolve(backend)["log_format"]
    if backend.startswith("codex"):
        return "codex"
    if backend.startswith("claude") or backend == "minimax":
        return "claude"
    return "codex"


def find_logfile(newer_than: Path, backend: str = "codex") -> Optional[str]:
    """Find the most recent JSONL file newer than the reference file."""
    ref_mtime = newer_than.stat().st_mtime if newer_than.exists() else 0
    pattern = LOG_FORMAT_PATTERNS[normalize_backend(backend)]
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
    event_type = ev.get("type", "?")
    if status["last_type"] == "?":
        status["last_type"] = event_type

    payload = ev.get("payload")
    if isinstance(payload, dict):
        payload_type = payload.get("type", event_type)
        if status["last_type"] in CODEX_NOISE_TYPES:
            status["last_type"] = payload_type

        if event_type == "event_msg":
            if payload_type == "agent_message":
                text = payload.get("message", "").strip()
                if text:
                    if not status["last_text"]:
                        status["last_text"] = text[:200]
                    _set_meaningful_type(status, "agent_message")
                    return True
            if payload_type in ("token_count", "task_started", "task_complete", "user_message"):
                return False

        if event_type == "response_item":
            if payload_type in (
                "reasoning",
                "function_call_output",
                "custom_tool_call_output",
            ):
                return False

            if payload_type == "message" and payload.get("role") == "assistant":
                for block in reversed(payload.get("content", [])):
                    if not isinstance(block, dict):
                        continue
                    if block.get("type") in ("output_text", "text"):
                        text = block.get("text", "").strip()
                        if text:
                            if not status["last_text"]:
                                status["last_text"] = text[:200]
                            _set_meaningful_type(status, payload_type)
                            return True
                return False

            if payload_type in ("function_call", "custom_tool_call", "web_search_call"):
                if not status["last_tool"]:
                    status["last_tool"] = payload.get("name", payload_type)
                _set_meaningful_type(status, payload_type)
                return True

    item = ev.get("item")
    if isinstance(item, dict):
        item_type = item.get("type", event_type)
        if item_type == "agent_message":
            text = item.get("text", "").strip()
            if text:
                if not status["last_text"]:
                    status["last_text"] = text[:200]
                _set_meaningful_type(status, item_type)
                return True
        if item_type == "file_change":
            changes = item.get("changes", [])
            if changes:
                path = changes[0].get("path", "")
                if not status["last_tool"]:
                    status["last_tool"] = f"file_change:{Path(path).name}" if path else "file_change"
            else:
                if not status["last_tool"]:
                    status["last_tool"] = "file_change"
            _set_meaningful_type(status, item_type)
            return True
        if item_type == "todo_list":
            return False

    # Older Codex event shapes use a payload containing content blocks.
    payload = ev.get("payload", ev)
    msg_type = payload.get("type", event_type)
    if status["last_type"] in CODEX_NOISE_TYPES:
        status["last_type"] = msg_type

    # Assistant messages
    if msg_type in ("assistant", "response.output_item.done"):
        content = payload.get("content", payload.get("item", {}).get("content", []))
        if isinstance(content, str):
            if not status["last_text"]:
                status["last_text"] = content.strip()[:200]
            _set_meaningful_type(status, msg_type)
            return True
        if isinstance(content, list):
            for block in reversed(content):
                if isinstance(block, str):
                    if not status["last_text"]:
                        status["last_text"] = block.strip()[:200]
                    _set_meaningful_type(status, msg_type)
                    return True
                btype = block.get("type", "")
                if btype in ("function_call", "tool_use") and not status["last_tool"]:
                    status["last_tool"] = block.get("name", block.get("function", {}).get("name", "?"))
                    _set_meaningful_type(status, btype)
                elif btype in ("text", "output_text") and not status["last_text"]:
                    text = block.get("text", "").strip()
                    if text:
                        status["last_text"] = text[:200]
                        _set_meaningful_type(status, btype)
            return bool(status["last_tool"] or status["last_text"])
    return False


def get_status(logfile: str, backend: str = "codex") -> dict:
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
    is_codex_backend = normalize_backend(backend) == "codex"
    parser = _parse_codex_event if is_codex_backend else _parse_claude_event

    # Scan recent lines for the most recent useful content.
    lookback = CODEX_LOOKBACK_LINES if is_codex_backend else 10
    for line in reversed(lines[-lookback:]):
        try:
            ev = json.loads(line)
        except json.JSONDecodeError:
            continue

        parser(ev, status)
        if is_codex_backend:
            if status["last_text"] and status["last_tool"]:
                break
        elif status["last_text"] or status["last_tool"]:
            break

    return status


def _iter_blocks(ev: dict) -> list[dict]:
    event_type = ev.get("type")
    if event_type == "assistant":
        payload = ev.get("payload")
        if payload is not None:
            content = payload.get("content", payload.get("item", {}).get("content", []))
        else:
            content = ev.get("message", {}).get("content", [])
    elif event_type == "response.output_item.done":
        payload = ev.get("payload", {})
        content = payload.get("item", {}).get("content", payload.get("content", []))
    elif event_type == "event_msg":
        payload = ev.get("payload", {})
        if payload.get("type") == "agent_message":
            text = payload.get("message", "").strip()
            return [{"type": "text", "text": text}] if text else []
        return []
    elif event_type == "response_item":
        payload = ev.get("payload", {})
        payload_type = payload.get("type")
        if payload_type == "message" and payload.get("role") == "assistant":
            content = payload.get("content", [])
        elif payload_type in ("function_call", "custom_tool_call", "web_search_call"):
            return [{
                "type": "function_call",
                "name": payload.get("name", payload_type),
                "arguments": payload.get("arguments", {}),
            }]
        else:
            return []
    elif event_type in ("item.completed", "item.started"):
        item = ev.get("item", {})
        item_type = item.get("type")
        if item_type == "agent_message":
            text = item.get("text", "").strip()
            return [{"type": "text", "text": text}] if text else []
        if item_type == "file_change":
            changes = item.get("changes", [])
            path = changes[0].get("path", "") if changes else ""
            return [{
                "type": "function_call",
                "name": "file_change",
                "arguments": {"file_path": path},
            }]
        return []
    else:
        return []

    if isinstance(content, str):
        text = content.strip()
        return [{"type": "text", "text": text}] if text else []
    if isinstance(content, list):
        return [block for block in content if isinstance(block, dict)]
    return []


def _tool_summary(block: dict) -> Optional[str]:
    block_type = block.get("type")
    if block_type not in ("tool_use", "function_call"):
        return None

    name = block.get("name", block.get("function", {}).get("name", "?"))
    tool_input = block.get("input", block.get("arguments", {}))
    if isinstance(tool_input, str):
        try:
            tool_input = json.loads(tool_input)
        except json.JSONDecodeError:
            tool_input = {"command": tool_input}

    if name in ("Bash", "shell"):
        command = tool_input.get("command", "") if isinstance(tool_input, dict) else str(tool_input)
        return f"> `{name}`: `{command[:200]}`"
    if name in ("exec_command", "write_stdin", "read_thread_terminal"):
        if isinstance(tool_input, dict):
            command = tool_input.get("cmd") or tool_input.get("chars") or ""
        else:
            command = str(tool_input)
        return f"> `{name}`: `{str(command)[:200]}`"
    if name in ("Read", "Glob", "Grep"):
        if isinstance(tool_input, dict):
            arg = tool_input.get("file_path") or tool_input.get("pattern") or ""
        else:
            arg = str(tool_input)
        return f"> `{name}`: `{str(arg)[:200]}`"
    if name == "Edit":
        file_path = tool_input.get("file_path", "") if isinstance(tool_input, dict) else str(tool_input)
        return f"> `{name}`: `{file_path}`"
    if name == "file_change":
        file_path = tool_input.get("file_path", "") if isinstance(tool_input, dict) else str(tool_input)
        return f"> `{name}`: `{os.path.basename(file_path) or file_path}`"
    if name == "web_search_call":
        query = tool_input.get("query", "") if isinstance(tool_input, dict) else str(tool_input)
        return f"> `{name}`: `{str(query)[:200]}`"
    return f"> `{name}`"


def extract_markdown(path: str, max_lines: int = 500) -> None:
    lines_printed = 0
    with open(path) as handle:
        for line in handle:
            if lines_printed >= max_lines:
                break
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            for block in _iter_blocks(event):
                if lines_printed >= max_lines:
                    break
                if block.get("type") in ("text", "output_text") and block.get("text", "").strip():
                    text = block["text"].strip()
                    print(text)
                    print()
                    lines_printed += text.count("\n") + 2
                else:
                    summary = _tool_summary(block)
                    if summary is None:
                        continue
                    print(summary)
                    print()
                    lines_printed += 2


def _content_blocks(ev: dict) -> list[dict]:
    if ev.get("type") in ("item.completed", "item.started"):
        item = ev.get("item", {})
        if item.get("type") == "agent_message":
            text = item.get("text", "").strip()
            return [{"type": "text", "text": text}] if text else []
        return []

    payload = ev.get("payload", ev)
    msg_type = payload.get("type", ev.get("type"))
    if msg_type == "assistant":
        content = payload.get("content", [])
    elif msg_type == "response.output_item.done":
        item = payload.get("item", {})
        content = item.get("content", payload.get("content", []))
    else:
        return []

    if isinstance(content, str):
        text = content.strip()
        return [{"type": "text", "text": text}] if text else []
    if isinstance(content, list):
        return [block for block in content if isinstance(block, dict)]
    return []


def _block_text(block: dict) -> str:
    block_type = block.get("type")
    if block_type in ("text", "output_text"):
        return block.get("text", "").strip()
    if block_type == "output_text.delta":
        return block.get("delta", "").strip()
    return ""


def _last_text(events: list[dict]) -> str:
    for event in reversed(events):
        for block in reversed(_content_blocks(event)):
            text = _block_text(block)
            if text:
                return text
    return ""


def _count_turns(events: list[dict]) -> int:
    completed_turns = sum(1 for event in events if event.get("type") == "turn.completed")
    if completed_turns:
        return completed_turns
    return sum(1 for event in events if _content_blocks(event))


def _extract_cost(events: list[dict]):
    for event in reversed(events):
        usage = event.get("usage")
        if isinstance(usage, dict) and usage.get("total_cost_usd") is not None:
            return usage["total_cost_usd"]
        payload = event.get("payload", event)
        for obj in (payload, payload.get("response", {}), payload.get("item", {})):
            if isinstance(obj, dict) and obj.get("total_cost_usd") is not None:
                return obj["total_cost_usd"]
    return None


def summarize_result(events_path: Path, last_message_path: Path) -> dict:
    events: list[dict] = []
    if events_path.exists():
        with events_path.open() as handle:
            for line in handle:
                line = line.strip()
                if not line:
                    continue
                try:
                    events.append(json.loads(line))
                except json.JSONDecodeError:
                    continue

    result_text = last_message_path.read_text().strip() if last_message_path.exists() else ""
    if not result_text:
        result_text = _last_text(events)

    return {
        "backend": "codex",
        "events": len(events),
        "num_turns": _count_turns(events),
        "total_cost_usd": _extract_cost(events),
        "result": result_text,
    }


def main():
    parser = argparse.ArgumentParser(description="Workflow-time agent log tooling")
    subparsers = parser.add_subparsers(dest="command", required=True)

    watch_parser = subparsers.add_parser("watch", help="Print live progress updates")
    watch_parser.add_argument("--newer-than", type=Path, required=True)
    watch_parser.add_argument("--interval", type=int, default=30)
    watch_parser.add_argument(
        "--backend",
        choices=sorted(LOG_FORMAT_PATTERNS),
        default="codex",
    )

    extract_parser = subparsers.add_parser("extract", help="Render a conversation excerpt as markdown")
    extract_parser.add_argument("path", help="Path to JSONL session log")
    extract_parser.add_argument("--max-lines", type=int, default=500)

    summarize_parser = subparsers.add_parser("summarize", help="Normalize agent output into JSON")
    summarize_parser.add_argument("events", type=Path, help="Path to JSONL event log")
    summarize_parser.add_argument("last_message", type=Path, help="Path to final message text file")

    args = parser.parse_args()

    if args.command == "extract":
        extract_markdown(args.path, args.max_lines)
        return

    if args.command == "summarize":
        json.dump(summarize_result(args.events, args.last_message), sys.stdout)
        sys.stdout.write("\n")
        return

    time.sleep(10)
    while True:
        now = datetime.now().strftime("%H:%M:%S")
        logfile = find_logfile(args.newer_than, args.backend)
        if logfile:
            status = get_status(logfile, args.backend)
            tool = status["last_tool"] or "n/a"
            text = status["last_text"] or "(none)"
            print(
                f"[progress] {now} | {status['events']} events | "
                f"type: {status['last_type']} | tool: {tool} | text: {text}",
                flush=True,
            )
        else:
            print(f"[progress] {now} | waiting for session to start...", flush=True)
        time.sleep(args.interval)


if __name__ == "__main__":
    main()
