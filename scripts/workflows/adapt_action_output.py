#!/usr/bin/env python3
"""Adapt claude-code-action execution_file to AGENT_RESULT_FILE format.

claude-code-action outputs an array of SDKMessage objects in its
execution_file. The downstream workflow expects a simpler JSON object
with ``total_cost_usd``, ``num_turns``, and ``result`` keys (the format
produced by ``claude -p --output-format json``).

Usage:
    python3 adapt_action_output.py <execution_file> <output_file>
"""
import json
import sys
from pathlib import Path


def adapt(execution_file: Path, output_file: Path) -> None:
    raw = execution_file.read_text()
    if not raw.strip():
        output_file.write_text(json.dumps({"result": "no result"}))
        return

    data = json.loads(raw)

    # execution_file may be a list of SDKMessage objects or a single
    # result dict (format varies by action version).
    if isinstance(data, dict):
        # Already in the expected format or close to it.
        adapted = {
            "total_cost_usd": data.get("total_cost_usd"),
            "num_turns": data.get("num_turns"),
            "result": data.get("result", "no result"),
            "duration_ms": data.get("duration_ms"),
        }
        output_file.write_text(json.dumps(adapted, indent=2))
        return

    if not isinstance(data, list):
        output_file.write_text(json.dumps({"result": "no result"}))
        return

    # Walk the message array to find the result message and the last
    # assistant text.
    result_msg = None
    last_text = ""

    for msg in data:
        if not isinstance(msg, dict):
            continue
        if msg.get("type") == "result":
            result_msg = msg
        # Extract last assistant text block.
        if msg.get("type") == "assistant":
            for block in msg.get("message", {}).get("content", []):
                if isinstance(block, dict) and block.get("type") == "text":
                    last_text = block.get("text", "")

    adapted = {
        "total_cost_usd": result_msg.get("total_cost_usd") if result_msg else None,
        "num_turns": result_msg.get("num_turns") if result_msg else None,
        "result": last_text or (result_msg.get("result", "no result") if result_msg else "no result"),
        "duration_ms": result_msg.get("duration_ms") if result_msg else None,
    }

    output_file.write_text(json.dumps(adapted, indent=2))


def main() -> None:
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <execution_file> <output_file>", file=sys.stderr)
        sys.exit(1)
    adapt(Path(sys.argv[1]), Path(sys.argv[2]))


if __name__ == "__main__":
    main()
