#!/usr/bin/env python3
"""Tests for agent_logs.py summarize mode."""
import json
import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).parents[3] / "scripts" / "workflows" / "agent_logs.py"


def run(events: list[dict], last_message: str = "") -> dict:
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as events_file:
        for ev in events:
            events_file.write(json.dumps(ev) + "\n")
        events_file.flush()
        with tempfile.NamedTemporaryFile(mode="w", suffix=".txt", delete=False) as last_file:
            last_file.write(last_message)
            last_file.flush()
            result = subprocess.run(
                [sys.executable, str(SCRIPT), "summarize", events_file.name, last_file.name],
                capture_output=True,
                text=True,
                check=True,
            )
    return json.loads(result.stdout)


def test_uses_last_message_file():
    summary = run([], last_message="Applied the fix.")
    assert summary["backend"] == "codex"
    assert summary["result"] == "Applied the fix."
    assert summary["events"] == 0


def test_falls_back_to_last_text_event():
    events = [{
        "type": "response.output_item.done",
        "payload": {
            "type": "response.output_item.done",
            "item": {"content": [{"type": "text", "text": "Ran tests successfully."}]},
        },
    }]
    summary = run(events)
    assert summary["result"] == "Ran tests successfully."
    assert summary["num_turns"] == 1


def test_counts_multiple_turns():
    events = [
        {
            "type": "assistant",
            "payload": {"type": "assistant", "content": "Inspecting fixtures."},
        },
        {
            "type": "response.output_item.done",
            "payload": {
                "type": "response.output_item.done",
                "item": {"content": [{"type": "function_call", "name": "shell"}]},
            },
        },
    ]
    summary = run(events)
    assert summary["events"] == 2
    assert summary["num_turns"] == 2


def test_handles_current_codex_item_events():
    events = [
        {
            "type": "item.completed",
            "item": {"type": "agent_message", "text": "Working through the fix."},
        },
        {
            "type": "turn.completed",
            "usage": {"input_tokens": 100, "output_tokens": 25},
        },
    ]
    summary = run(events)
    assert summary["result"] == "Working through the fix."
    assert summary["num_turns"] == 1


if __name__ == "__main__":
    test_uses_last_message_file()
    test_falls_back_to_last_text_event()
    test_counts_multiple_turns()
    test_handles_current_codex_item_events()
    print("All tests passed.")
