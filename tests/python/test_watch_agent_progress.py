#!/usr/bin/env python3
"""Tests for watch_agent_progress.py."""
import json
import os
import sys
import tempfile
from pathlib import Path

# Import the module directly for unit testing get_status
sys.path.insert(0, str(Path(__file__).parents[2] / "scripts" / "ci"))
import watch_agent_progress


def write_jsonl(path: str, events: list[dict]) -> None:
    with open(path, "w") as f:
        for ev in events:
            f.write(json.dumps(ev) + "\n")


def make_assistant(content_blocks: list) -> dict:
    return {"type": "assistant", "message": {"content": content_blocks}}


def test_get_status_text():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        write_jsonl(f.name, [
            make_assistant([{"type": "text", "text": "Investigating the bug."}]),
        ])
        s = watch_agent_progress.get_status(f.name)
    assert s["events"] == 1
    assert s["last_type"] == "assistant"
    assert "Investigating the bug" in s["last_text"]
    assert s["last_tool"] is None


def test_get_status_tool():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        write_jsonl(f.name, [
            make_assistant([{
                "type": "tool_use",
                "name": "Bash",
                "input": {"command": "cargo test"},
            }]),
        ])
        s = watch_agent_progress.get_status(f.name)
    assert s["last_tool"] == "Bash"


def test_get_status_mixed():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        write_jsonl(f.name, [
            make_assistant([{"type": "text", "text": "First message"}]),
            {"type": "user", "message": {"content": "ok"}},
            make_assistant([
                {"type": "text", "text": "Running tests now."},
                {"type": "tool_use", "name": "Edit", "input": {"file_path": "foo.rs"}},
            ]),
        ])
        s = watch_agent_progress.get_status(f.name)
    assert s["events"] == 3
    assert s["last_tool"] == "Edit"
    assert "Running tests now" in s["last_text"]


def test_get_status_empty():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        f.write("")
        f.flush()
        s = watch_agent_progress.get_status(f.name)
    assert s["events"] == 0
    assert s["last_type"] == "?"


def test_get_status_malformed():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        f.write("not json\n")
        f.write(json.dumps(make_assistant([{"type": "text", "text": "OK"}])) + "\n")
        f.flush()
        s = watch_agent_progress.get_status(f.name)
    assert s["events"] == 2
    assert s["last_text"] == "OK"


def test_get_status_skips_empty_text():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        write_jsonl(f.name, [
            make_assistant([{"type": "text", "text": "   "}]),
        ])
        s = watch_agent_progress.get_status(f.name)
    assert s["last_text"] is None


def test_get_status_truncates_long_text():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        write_jsonl(f.name, [
            make_assistant([{"type": "text", "text": "x" * 500}]),
        ])
        s = watch_agent_progress.get_status(f.name)
    assert len(s["last_text"]) == 200


def test_codex_text_event():
    """Codex rollout format with assistant text content."""
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        write_jsonl(f.name, [
            {"type": "response.output_item.done", "payload": {
                "type": "response.output_item.done",
                "item": {"content": [{"type": "text", "text": "Fixing the cop now."}]},
            }},
        ])
        s = watch_agent_progress.get_status(f.name, backend="codex")
    assert "Fixing the cop now" in s["last_text"]


def test_codex_tool_event():
    """Codex rollout format with function_call tool use."""
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        write_jsonl(f.name, [
            {"type": "response.output_item.done", "payload": {
                "type": "response.output_item.done",
                "item": {"content": [
                    {"type": "function_call", "name": "shell", "function": {"name": "shell"}},
                ]},
            }},
        ])
        s = watch_agent_progress.get_status(f.name, backend="codex")
    assert s["last_tool"] == "shell"


def test_codex_string_content():
    """Codex event with plain string content."""
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        write_jsonl(f.name, [
            {"type": "assistant", "payload": {
                "type": "assistant",
                "content": "Running cargo test now.",
            }},
        ])
        s = watch_agent_progress.get_status(f.name, backend="codex")
    assert "Running cargo test" in s["last_text"]


def test_find_logfile_returns_none():
    with tempfile.NamedTemporaryFile(suffix=".md", delete=False) as ref:
        # No JSONL files in a temp dir — should return None
        result = watch_agent_progress.find_logfile(Path(ref.name))
    assert result is None


if __name__ == "__main__":
    test_get_status_text()
    test_get_status_tool()
    test_get_status_mixed()
    test_get_status_empty()
    test_get_status_malformed()
    test_get_status_skips_empty_text()
    test_get_status_truncates_long_text()
    test_codex_text_event()
    test_codex_tool_event()
    test_codex_string_content()
    test_find_logfile_returns_none()
    print("All tests passed.")
