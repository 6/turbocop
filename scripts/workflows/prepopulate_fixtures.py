#!/usr/bin/env python3
"""Pre-populate offense fixtures with failing corpus FN examples.

Confirmed FN code bugs append ready-made test snippets to offense.rb.
Confirmed FP code bugs stay in task.md as source context for the agent
to distill into a clean no_offense.rb case manually.

This gives the agent a workspace where `cargo test` already fails for
known FN bugs, without committing raw corpus snippets into no_offense.rb.

Usage:
    python3 prepopulate_fixtures.py <task.md> <cop> <fixture_dir>

Reads pre-diagnostic results from task.md, extracts confirmed code bug
examples, and appends only FN snippets to offense.rb.
"""
import re
import sys
from pathlib import Path


def extract_diagnostics_from_task(task_path: Path) -> list[dict]:
    """Parse pre-diagnostic results from the task markdown.

    Looks for FP/FN sections with CODE BUG markers and extracts
    the source context and test snippets."""
    text = task_path.read_text()
    results = []

    # Find all FP CODE BUG sections with source context
    fp_pattern = re.compile(
        r'### FP #\d+:.*?\n'
        r'\*\*CONFIRMED false positive — CODE BUG\*\*.*?'
        r'(?:Full source context.*?```ruby\n(.*?)```|Add to no_offense\.rb:\n```ruby\n(.*?)```)',
        re.DOTALL,
    )
    for m in fp_pattern.finditer(text):
        source = m.group(1) or m.group(2)
        if source and source.strip():
            results.append({"kind": "fp", "source": source.strip()})

    # Find all FN CODE BUG sections with test snippets
    fn_pattern = re.compile(
        r'### FN #\d+:.*?\n'
        r'\*\*NOT DETECTED — CODE BUG\*\*.*?'
        r'Ready-made test snippet.*?```ruby\n(.*?)```',
        re.DOTALL,
    )
    for m in fn_pattern.finditer(text):
        snippet = m.group(1)
        if snippet and snippet.strip():
            results.append({"kind": "fn", "source": snippet.strip()})

    return results


def normalize_fixture_snippet(source: str) -> str:
    """Trim noisy boundary lines from extracted corpus snippets.

    The corpus context sometimes includes leading/trailing blank lines or
    comment-only spacer lines (`#`) that are not useful fixture content.
    Keep interior spacing intact, but strip those boundary markers so the
    pre-populated fixtures stay readable.
    """
    lines = source.splitlines()

    def is_boundary_noise(line: str) -> bool:
        stripped = line.strip()
        return stripped == "" or stripped == "#"

    while lines and is_boundary_noise(lines[0]):
        lines.pop(0)
    while lines and is_boundary_noise(lines[-1]):
        lines.pop()

    return "\n".join(lines).rstrip()


def prepopulate(task_path: Path, cop: str, fixture_dir: Path) -> dict:
    """Append confirmed FN code bug examples to offense.rb.

    Returns {"fp_context": int, "fn_added": int}."""
    diagnostics = extract_diagnostics_from_task(task_path)
    if not diagnostics:
        return {"fp_context": 0, "fn_added": 0}

    offense_path = fixture_dir / "offense.rb"
    fn_added = 0

    fp_examples = [d for d in diagnostics if d["kind"] == "fp"]

    # Append FN examples to offense.rb
    fn_examples = [d for d in diagnostics if d["kind"] == "fn"]
    if fn_examples and offense_path.exists():
        with open(offense_path, "a") as f:
            for ex in fn_examples:
                snippet = normalize_fixture_snippet(ex["source"])
                if not snippet:
                    continue
                f.write(f"\n{snippet}\n")
                fn_added += 1

    return {"fp_context": len(fp_examples), "fn_added": fn_added}


def main():
    if len(sys.argv) != 4:
        print(f"Usage: {sys.argv[0]} <task.md> <cop> <fixture_dir>", file=sys.stderr)
        sys.exit(1)

    task_path = Path(sys.argv[1])
    cop = sys.argv[2]
    fixture_dir = Path(sys.argv[3])

    if not task_path.exists():
        print(f"Error: {task_path} not found", file=sys.stderr)
        sys.exit(1)

    if not fixture_dir.exists():
        print(f"Error: {fixture_dir} not found", file=sys.stderr)
        sys.exit(1)

    result = prepopulate(task_path, cop, fixture_dir)
    print(
        f"Left {result['fp_context']} FP examples in task.md for manual no_offense.rb distillation"
    )
    print(f"Added {result['fn_added']} FN examples to offense.rb")


if __name__ == "__main__":
    main()
