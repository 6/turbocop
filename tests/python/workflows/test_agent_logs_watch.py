#!/usr/bin/env python3
"""Tests for agent_logs.py watch helpers."""
import json
import sys
import tempfile
from pathlib import Path

# Import the module directly for unit testing get_status
sys.path.insert(0, str(Path(__file__).parents[3] / "scripts" / "workflows"))
import agent_logs
import resolve_backend


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
        s = agent_logs.get_status(f.name, backend="claude-normal")
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
        s = agent_logs.get_status(f.name, backend="claude-normal")
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
        s = agent_logs.get_status(f.name, backend="claude-normal")
    assert s["events"] == 3
    assert s["last_tool"] == "Edit"
    assert "Running tests now" in s["last_text"]


def test_get_status_empty():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        f.write("")
        f.flush()
        s = agent_logs.get_status(f.name, backend="claude-normal")
    assert s["events"] == 0
    assert s["last_type"] == "?"


def test_get_status_malformed():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        f.write("not json\n")
        f.write(json.dumps(make_assistant([{"type": "text", "text": "OK"}])) + "\n")
        f.flush()
        s = agent_logs.get_status(f.name, backend="claude-normal")
    assert s["events"] == 2
    assert s["last_text"] == "OK"


def test_get_status_skips_empty_text():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        write_jsonl(f.name, [
            make_assistant([{"type": "text", "text": "   "}]),
        ])
        s = agent_logs.get_status(f.name, backend="claude-normal")
    assert s["last_text"] is None


def test_get_status_truncates_long_text():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        write_jsonl(f.name, [
            make_assistant([{"type": "text", "text": "x" * 500}]),
        ])
        s = agent_logs.get_status(f.name, backend="claude-normal")
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
        s = agent_logs.get_status(f.name, backend="codex-hard")
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
        s = agent_logs.get_status(f.name, backend="codex-hard")
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
        s = agent_logs.get_status(f.name, backend="codex-hard")
    assert "Running cargo test" in s["last_text"]


def test_codex_current_agent_message_event():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        write_jsonl(f.name, [
            {
                "type": "item.completed",
                "item": {"type": "agent_message", "text": "Updating the fixture first."},
            },
        ])
        s = agent_logs.get_status(f.name, backend="codex-hard")
    assert s["last_type"] == "agent_message"
    assert "Updating the fixture first" in s["last_text"]


def test_codex_current_file_change_event():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        write_jsonl(f.name, [
            {
                "type": "item.completed",
                "item": {
                    "type": "file_change",
                    "changes": [{"path": "/tmp/src/cop/style/mixin_usage.rs"}],
                },
            },
        ])
        s = agent_logs.get_status(f.name, backend="codex-hard")
    assert s["last_type"] == "file_change"
    assert s["last_tool"] == "file_change:mixin_usage.rs"


def test_codex_event_msg_agent_message():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        write_jsonl(f.name, [
            {
                "type": "event_msg",
                "payload": {
                    "type": "agent_message",
                    "message": "Inspecting the latest session format now.",
                },
            },
        ])
        s = agent_logs.get_status(f.name, backend="codex-hard")
    assert s["last_type"] == "agent_message"
    assert "Inspecting the latest session format" in s["last_text"]


def test_codex_response_item_function_call_sets_tool():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        write_jsonl(f.name, [
            {
                "type": "response_item",
                "payload": {
                    "type": "function_call",
                    "name": "exec_command",
                    "arguments": "{\"cmd\":\"pwd\"}",
                },
            },
        ])
        s = agent_logs.get_status(f.name, backend="codex-hard")
    assert s["last_type"] == "function_call"
    assert s["last_tool"] == "exec_command"


def test_codex_looks_past_token_count_noise():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        events = [
            {
                "type": "event_msg",
                "payload": {
                    "type": "agent_message",
                    "message": "Useful status before token churn.",
                },
            }
        ]
        for _ in range(20):
            events.append({
                "type": "event_msg",
                "payload": {
                    "type": "token_count",
                    "info": {"total_token_usage": {"input_tokens": 1}},
                },
            })
        write_jsonl(f.name, events)
        s = agent_logs.get_status(f.name, backend="codex-hard")
    assert s["last_type"] == "agent_message"
    assert "Useful status before token churn" in s["last_text"]


def test_codex_ignores_function_call_output_noise():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        write_jsonl(f.name, [
            {
                "type": "response_item",
                "payload": {
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": "Planning the fix."}],
                },
            },
            {
                "type": "response_item",
                "payload": {
                    "type": "function_call",
                    "name": "exec_command",
                    "arguments": "{\"cmd\":\"pwd\"}",
                },
            },
            {
                "type": "response_item",
                "payload": {
                    "type": "function_call_output",
                    "call_id": "abc",
                    "output": "ok",
                },
            },
        ])
        s = agent_logs.get_status(f.name, backend="codex-hard")
    assert s["last_type"] == "function_call"
    assert s["last_tool"] == "exec_command"
    assert "Planning the fix" in s["last_text"]


def test_find_logfile_returns_none():
    with tempfile.NamedTemporaryFile(suffix=".md", delete=False) as ref:
        # No JSONL files in a temp dir — should return None
        result = agent_logs.find_logfile(Path(ref.name))
    assert result is None


def test_agent_log_formats_match_resolve_backend_outputs():
    expected = {config["log_format"] for config in resolve_backend.BACKENDS.values()}
    assert expected == set(agent_logs.LOG_FORMAT_PATTERNS)


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
    test_codex_current_agent_message_event()
    test_codex_current_file_change_event()
    test_codex_event_msg_agent_message()
    test_codex_response_item_function_call_sets_tool()
    test_codex_looks_past_token_count_noise()
    test_codex_ignores_function_call_output_noise()
    test_find_logfile_returns_none()
    test_agent_log_formats_match_resolve_backend_outputs()
    print("All tests passed.")
