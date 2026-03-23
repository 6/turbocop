#!/usr/bin/env python3
"""Tests for adapt_action_output.py."""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[3] / "scripts" / "workflows"))
import adapt_action_output


def test_sdk_message_array(tmp_path):
    """Standard SDKMessage array with result and assistant messages."""
    messages = [
        {
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "text", "text": "I fixed the cop."},
                ],
            },
        },
        {
            "type": "result",
            "subtype": "success",
            "num_turns": 12,
            "total_cost_usd": 0.85,
            "duration_ms": 120000,
            "is_error": False,
        },
    ]
    execution_file = tmp_path / "execution.json"
    output_file = tmp_path / "result.json"
    execution_file.write_text(json.dumps(messages))

    adapt_action_output.adapt(execution_file, output_file)

    result = json.loads(output_file.read_text())
    assert result["total_cost_usd"] == 0.85
    assert result["num_turns"] == 12
    assert result["result"] == "I fixed the cop."
    assert result["duration_ms"] == 120000


def test_dict_format(tmp_path):
    """Already a dict (e.g. future action version or direct JSON output)."""
    data = {
        "total_cost_usd": 1.5,
        "num_turns": 8,
        "result": "Done.",
        "duration_ms": 60000,
    }
    execution_file = tmp_path / "execution.json"
    output_file = tmp_path / "result.json"
    execution_file.write_text(json.dumps(data))

    adapt_action_output.adapt(execution_file, output_file)

    result = json.loads(output_file.read_text())
    assert result["total_cost_usd"] == 1.5
    assert result["num_turns"] == 8
    assert result["result"] == "Done."


def test_empty_file(tmp_path):
    """Empty execution file produces a fallback result."""
    execution_file = tmp_path / "execution.json"
    output_file = tmp_path / "result.json"
    execution_file.write_text("")

    adapt_action_output.adapt(execution_file, output_file)

    result = json.loads(output_file.read_text())
    assert result["result"] == "no result"


def test_no_result_message(tmp_path):
    """Array without a result message still extracts assistant text."""
    messages = [
        {
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "text", "text": "Partial work done."},
                ],
            },
        },
    ]
    execution_file = tmp_path / "execution.json"
    output_file = tmp_path / "result.json"
    execution_file.write_text(json.dumps(messages))

    adapt_action_output.adapt(execution_file, output_file)

    result = json.loads(output_file.read_text())
    assert result["total_cost_usd"] is None
    assert result["num_turns"] is None
    assert result["result"] == "Partial work done."


def test_multiple_assistant_messages(tmp_path):
    """Last assistant text is used as result."""
    messages = [
        {
            "type": "assistant",
            "message": {
                "content": [{"type": "text", "text": "First message."}],
            },
        },
        {
            "type": "assistant",
            "message": {
                "content": [{"type": "text", "text": "Final summary."}],
            },
        },
        {
            "type": "result",
            "num_turns": 5,
            "total_cost_usd": 0.3,
        },
    ]
    execution_file = tmp_path / "execution.json"
    output_file = tmp_path / "result.json"
    execution_file.write_text(json.dumps(messages))

    adapt_action_output.adapt(execution_file, output_file)

    result = json.loads(output_file.read_text())
    assert result["result"] == "Final summary."


if __name__ == "__main__":
    import tempfile

    for test_fn in [
        test_sdk_message_array,
        test_dict_format,
        test_empty_file,
        test_no_result_message,
        test_multiple_assistant_messages,
    ]:
        with tempfile.TemporaryDirectory() as d:
            test_fn(Path(d))
    print("All tests passed.")
