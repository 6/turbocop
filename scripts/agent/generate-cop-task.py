#!/usr/bin/env python3
from __future__ import annotations
"""Generate a self-contained task prompt for a remote agent to fix one cop.

Produces a single markdown document containing everything the agent needs:
- Focused instructions (TDD workflow, fixture format, validation)
- Pre-diagnostic results (code bug vs config issue classification)
- Ready-made test snippets
- The cop's Rust source
- The RuboCop Ruby implementation (ground truth)
- RuboCop spec excerpts (expect_offense / expect_no_offenses blocks)
- Current test fixtures
- Pre-computed FP/FN examples from corpus oracle

Usage:
    python3 scripts/agent/generate-cop-task.py Style/NegatedWhile
    python3 scripts/agent/generate-cop-task.py Style/NegatedWhile --output /tmp/task.md
    python3 scripts/agent/generate-cop-task.py Style/NegatedWhile --extended
    python3 scripts/agent/generate-cop-task.py Style/NegatedWhile --binary target/debug/nitrocop
"""

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

# Allow importing from scripts/
SCRIPTS_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(SCRIPTS_DIR))
from corpus_download import download_corpus_results as _download_corpus

PROJECT_ROOT = SCRIPTS_DIR.parent

# Department → vendor gem directory mapping
DEPT_TO_VENDOR = {
    "Bundler": "vendor/rubocop",
    "FactoryBot": "vendor/rubocop",
    "Gemspec": "vendor/rubocop",
    "Layout": "vendor/rubocop",
    "Lint": "vendor/rubocop",
    "Metrics": "vendor/rubocop",
    "Migration": "vendor/rubocop",
    "Naming": "vendor/rubocop",
    "Security": "vendor/rubocop",
    "Style": "vendor/rubocop",
    "Performance": "vendor/rubocop-performance",
    "Rails": "vendor/rubocop-rails",
    "RSpec": "vendor/rubocop-rspec",
    "RSpecRails": "vendor/rubocop-rspec_rails",
}

# Department name → directory name in vendor gem (usually lowercase)
DEPT_TO_DIR = {
    "FactoryBot": "factory_bot",
    "RSpec": "rspec",
    "RSpecRails": "rspec_rails",
}

# Department PascalCase → snake_case directory name in src/cop/ and tests/fixtures/
# Only needed for departments where pascal_to_snake() gives the wrong result
DEPT_TO_SRC_DIR = {
    "RSpec": "rspec",
    "RSpecRails": "rspec_rails",
    "FactoryBot": "factory_bot",
}

# Known Prism pitfalls: keyword → note
PRISM_PITFALLS = {
    "as_hash_node": (
        "hash splits into HashNode (literal `{}`) and KeywordHashNode "
        "(keyword args `foo(a: 1)`). If you handle one, check if you need the other."
    ),
    "as_constant_read_node": (
        "const splits into ConstantReadNode (simple `Foo`) and ConstantPathNode "
        "(qualified `Foo::Bar`). If you handle one, check if you need the other."
    ),
    "as_begin_node": (
        "begin is overloaded: explicit `begin..end` is BeginNode, implicit method "
        "body is StatementsNode. Check which one(s) your cop needs."
    ),
}


def pascal_to_snake(name: str) -> str:
    """Convert PascalCase to snake_case. E.g. NegatedWhile -> negated_while."""
    s = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", name)
    s = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", s)
    return s.lower()


def parse_cop_name(cop: str) -> tuple[str, str, str]:
    """Parse 'Department/CopName' into (department, cop_name, snake_name)."""
    if "/" not in cop:
        print(f"Error: cop must be in Department/CopName format, got: {cop}",
              file=sys.stderr)
        sys.exit(1)
    dept, name = cop.split("/", 1)
    return dept, name, pascal_to_snake(name)


def read_file_safe(path: Path) -> str | None:
    """Read a file, return None if it doesn't exist."""
    if path.exists():
        return path.read_text(errors="replace")
    return None


def dept_dir_name(dept: str) -> str:
    """Get the directory name for a department in src/cop/ and tests/fixtures/."""
    return DEPT_TO_SRC_DIR.get(dept, pascal_to_snake(dept))


def find_rust_source(dept: str, snake: str) -> Path:
    """Find the cop's Rust source file."""
    return PROJECT_ROOT / "src" / "cop" / dept_dir_name(dept) / f"{snake}.rs"


def find_vendor_ruby_source(dept: str, snake: str) -> Path | None:
    """Find the RuboCop Ruby implementation."""
    vendor_gem = DEPT_TO_VENDOR.get(dept)
    if not vendor_gem:
        return None
    dept_dir = DEPT_TO_DIR.get(dept, dept.lower())
    path = PROJECT_ROOT / vendor_gem / "lib" / "rubocop" / "cop" / dept_dir / f"{snake}.rb"
    return path if path.exists() else None


def find_vendor_spec(dept: str, snake: str) -> Path | None:
    """Find the RuboCop spec file."""
    vendor_gem = DEPT_TO_VENDOR.get(dept)
    if not vendor_gem:
        return None
    dept_dir = DEPT_TO_DIR.get(dept, dept.lower())
    path = PROJECT_ROOT / vendor_gem / "spec" / "rubocop" / "cop" / dept_dir / f"{snake}_spec.rb"
    return path if path.exists() else None


def extract_spec_excerpts(spec_text: str, max_blocks: int = 20) -> str:
    """Extract expect_offense and expect_no_offenses blocks from a spec file.

    Returns a condensed version with just the key test cases."""
    blocks = []
    lines = spec_text.splitlines()
    i = 0
    while i < len(lines) and len(blocks) < max_blocks:
        line = lines[i]
        # Match expect_offense or expect_no_offenses with heredoc
        if re.search(r"expect_(offense|no_offenses)\s*\(\s*<<~?\s*['\"]?(\w+)", line):
            # Find the heredoc delimiter
            m = re.search(r"<<~?\s*['\"]?(\w+)", line)
            if m:
                delimiter = m.group(1)
                block_lines = [line]
                i += 1
                while i < len(lines):
                    block_lines.append(lines[i])
                    if lines[i].strip() == delimiter:
                        break
                    i += 1
                blocks.append("\n".join(block_lines))
        # Also capture it/context/describe blocks for context
        elif re.match(r"\s*(it|context|describe)\s+['\"]", line):
            blocks.append(line)
        i += 1
    return "\n\n".join(blocks) if blocks else "(no expect_offense blocks found)"


def find_fixtures(dept: str, snake: str) -> tuple[str | None, str | None]:
    """Find offense.rb and no_offense.rb fixture files."""
    fixture_dir = PROJECT_ROOT / "tests" / "fixtures" / "cops" / dept_dir_name(dept) / snake
    offense = read_file_safe(fixture_dir / "offense.rb")
    no_offense = read_file_safe(fixture_dir / "no_offense.rb")
    # Check scenario layout
    if offense is None:
        offense_dir = fixture_dir / "offense"
        if offense_dir.is_dir():
            parts = []
            for f in sorted(offense_dir.glob("*.rb")):
                parts.append(f"# --- {f.name} ---\n{f.read_text(errors='replace')}")
            if parts:
                offense = "\n\n".join(parts)
    return offense, no_offense


def get_corpus_data(cop: str, input_path: Path | None, extended: bool) -> dict:
    """Get FP/FN data from corpus-results.json.

    Returns dict with counts and raw example lists."""
    if input_path is None:
        try:
            prefer = "extended" if extended else "standard"
            input_path, _, _ = _download_corpus(prefer=prefer)
        except Exception as e:
            print(f"Warning: could not download corpus data: {e}", file=sys.stderr)
            return {"fp": 0, "fn": 0, "matches": 0,
                    "fp_examples": [], "fn_examples": []}

    data = json.loads(input_path.read_text())
    by_cop = data.get("by_cop", [])
    cop_entry = next((e for e in by_cop if e["cop"] == cop), None)

    if cop_entry is None:
        return {"fp": 0, "fn": 0, "matches": 0,
                "fp_examples": [], "fn_examples": []}

    return {
        "fp": cop_entry.get("fp", 0),
        "fn": cop_entry.get("fn", 0),
        "matches": cop_entry.get("matches", 0),
        "fp_examples": cop_entry.get("fp_examples", []),
        "fn_examples": cop_entry.get("fn_examples", []),
    }


def _normalize_example(ex) -> tuple[str, str, list[str] | None]:
    """Normalize an example to (loc_string, message, embedded_context)."""
    if isinstance(ex, dict):
        return ex.get("loc", ""), ex.get("msg", ""), ex.get("src")
    return ex, "", None


def _extract_source_lines(src: list[str]) -> tuple[list[str], str | None, int | None]:
    """Extract clean source lines from corpus src context.

    Returns (all_source_lines, offense_line, offense_line_index)."""
    source_lines = []
    offense_line = None
    offense_line_idx = None
    for i, s in enumerate(src):
        is_offense = s.strip().startswith(">>>")
        cleaned = re.sub(r"^(>>>\s*)?\s*\d+:\s?", "", s)
        source_lines.append(cleaned)
        if is_offense:
            offense_line = cleaned
            offense_line_idx = i
    return source_lines, offense_line, offense_line_idx


# Ruby block-opening keywords/patterns and their AST significance
_ENCLOSING_PATTERNS = [
    (r'^\s*BEGIN\s*\{', "BEGIN {} block (Prism: PreExecutionNode)"),
    (r'^\s*END\s*\{', "END {} block (Prism: PostExecutionNode)"),
    (r'^\s*class\s+', "class body"),
    (r'^\s*module\s+', "module body"),
    (r'^\s*def\s+', "method body"),
    (r'^\s*if\s+', "if branch"),
    (r'^\s*unless\s+', "unless branch"),
    (r'^\s*while\s+', "while loop"),
    (r'^\s*until\s+', "until loop"),
    (r'^\s*begin\b', "begin block"),
    (r'^\s*rescue\b', "rescue block"),
    (r'^\s*ensure\b', "ensure block"),
    (r'^\s*case\s+', "case expression"),
    (r'.*\bdo\s*(\|.*\|)?\s*$', "block (do..end)"),
    (r'.*\{\s*(\|.*\|)?\s*$', "block ({..})"),
]


def _find_enclosing_structure(
    source_lines: list[str], offense_line_idx: int | None,
) -> str | None:
    """Identify the enclosing Ruby structure around the offense line.

    Scans backwards from the offense line looking for block-opening keywords
    to help the agent understand what context makes a FP/FN different."""
    if offense_line_idx is None or offense_line_idx == 0:
        return None

    # Get indentation of offense line
    offense = source_lines[offense_line_idx]
    offense_indent = len(offense) - len(offense.lstrip())

    # Scan backwards for enclosing structure with less indentation
    for i in range(offense_line_idx - 1, -1, -1):
        line = source_lines[i]
        stripped = line.lstrip()
        if not stripped or stripped.startswith("#"):
            continue
        line_indent = len(line) - len(stripped)
        if line_indent < offense_indent:
            for pattern, desc in _ENCLOSING_PATTERNS:
                if re.match(pattern, line):
                    return f"{desc} (line: `{stripped.rstrip()}`)"
            # Generic: just report the line
            return f"enclosing line: `{stripped.rstrip()}`"
    return None


def _run_nitrocop(binary_path: Path, cwd: str, cop: str = "") -> list[dict]:
    """Run nitrocop on test.rb in the given directory, return offenses list."""
    cmd = [str(binary_path), "--force-default-config", "--format", "json"]
    if cop:
        cmd.extend(["--only", cop])
    cmd.append("test.rb")
    proc = subprocess.run(
        cmd, capture_output=True, text=True, timeout=15, cwd=cwd,
    )
    output_text = proc.stdout.strip()
    if output_text:
        try:
            output = json.loads(output_text)
            return output.get("offenses", [])
        except json.JSONDecodeError:
            pass
    return []


def run_diagnostic(
    binary_path: Path, cop: str,
    fp_examples: list, fn_examples: list,
) -> list[dict]:
    """Run nitrocop on extracted FP/FN source to classify each as code bug vs config issue.

    For each example with source context, creates a temp .rb file from the
    extracted source lines and runs nitrocop with --force-default-config to
    test detection in isolation.

    For FN: detected → config/context issue; not detected → code bug.
    For FP: flagged → confirmed code bug; not flagged → context-dependent.
    """
    results = []
    for kind, examples in [("fn", fn_examples), ("fp", fp_examples)]:
        for ex in examples[:15]:
            loc, msg, src = _normalize_example(ex)
            if not src:
                results.append({
                    "kind": kind, "loc": loc, "msg": msg,
                    "diagnosed": False, "reason": "no source context",
                })
                continue

            source_lines, offense_line, offense_line_idx = _extract_source_lines(src)
            if not source_lines:
                results.append({
                    "kind": kind, "loc": loc, "msg": msg,
                    "diagnosed": False, "reason": "empty source",
                })
                continue

            # Write temp file in its own directory (nitrocop needs a project root)
            tmp_dir = tempfile.mkdtemp(prefix="nitrocop_diag_")
            tmp_path = os.path.join(tmp_dir, "test.rb")
            with open(tmp_path, "w") as f:
                f.write("\n".join(source_lines) + "\n")

            try:
                offenses = _run_nitrocop(binary_path, tmp_dir, cop)

                # If no offenses with full context (may have parse errors from
                # truncated source), retry with just the offense line
                if not offenses and offense_line is not None:
                    with open(tmp_path, "w") as f:
                        f.write(offense_line + "\n")
                    offenses = _run_nitrocop(binary_path, tmp_dir, cop)

                detected = len(offenses) > 0

                # Generate test snippet from offense data
                test_snippet = None
                if offense_line is not None:
                    expected_line_num = (offense_line_idx + 1) if offense_line_idx is not None else 1
                    if detected and offenses:
                        # Find offense on the expected line
                        matching = [o for o in offenses if o.get("line") == expected_line_num]
                        o = matching[0] if matching else offenses[0]
                        col = o.get("column", 1) - 1  # 1-indexed → 0-indexed
                        omsg = o.get("message", msg)
                        test_snippet = f"{offense_line}\n{' ' * col}^ {cop}: {omsg}"
                    else:
                        test_snippet = f"{offense_line}\n^ {cop}: {msg}"

                # Identify enclosing structure for context
                enclosing = _find_enclosing_structure(source_lines, offense_line_idx)

                results.append({
                    "kind": kind, "loc": loc, "msg": msg,
                    "diagnosed": True, "detected": detected,
                    "offense_line": offense_line,
                    "test_snippet": test_snippet,
                    "enclosing": enclosing,
                    "source_context": "\n".join(source_lines),
                })
            except Exception as e:
                results.append({
                    "kind": kind, "loc": loc, "msg": msg,
                    "diagnosed": False, "reason": str(e),
                })
            finally:
                try:
                    os.unlink(tmp_path)
                    os.rmdir(tmp_dir)
                except OSError:
                    pass

    return results


def format_corpus_section(
    cop: str, corpus: dict, diagnostics: list[dict] | None,
) -> str:
    """Format the corpus FP/FN section, optionally enriched with diagnostic results."""
    fp_examples = corpus["fp_examples"]
    fn_examples = corpus["fn_examples"]

    if not fp_examples and not fn_examples:
        return "(no FP/FN examples in corpus data)"

    if diagnostics:
        return _format_with_diagnostics(cop, diagnostics, fp_examples, fn_examples)
    return _format_without_diagnostics(cop, fp_examples, fn_examples)


def _format_with_diagnostics(
    cop: str, diagnostics: list[dict],
    fp_examples: list, fn_examples: list,
) -> str:
    lines = []

    fn_diags = [d for d in diagnostics if d["kind"] == "fn"]
    fp_diags = [d for d in diagnostics if d["kind"] == "fp"]

    # Summary counts
    fn_code_bugs = sum(1 for d in fn_diags if d.get("diagnosed") and not d.get("detected"))
    fn_config = sum(1 for d in fn_diags if d.get("diagnosed") and d.get("detected"))
    fp_code_bugs = sum(1 for d in fp_diags if d.get("diagnosed") and d.get("detected"))
    fp_config = sum(1 for d in fp_diags if d.get("diagnosed") and not d.get("detected"))

    lines.append("### Diagnosis Summary")
    lines.append("Each example was tested by running nitrocop on the extracted source in isolation")
    lines.append("with `--force-default-config` to determine if the issue is a code bug or config issue.")
    lines.append("Note: source context is truncated and may not parse perfectly. If a diagnosis")
    lines.append("seems wrong (e.g., your test passes immediately for a 'CODE BUG'), treat it as")
    lines.append("a config/context issue instead.\n")
    if fn_diags:
        lines.append(f"- **FN:** {fn_code_bugs} code bug(s), {fn_config} config/context issue(s)")
    if fp_diags:
        lines.append(f"- **FP:** {fp_code_bugs} confirmed code bug(s), {fp_config} context-dependent")
    lines.append("")

    # FN details
    for i, d in enumerate(fn_diags, 1):
        lines.append(f"### FN #{i}: `{d['loc']}`")
        if d.get("diagnosed"):
            if d.get("detected"):
                lines.append("**DETECTED in isolation — CONFIG/CONTEXT issue**")
                lines.append("The cop correctly detects this pattern with default config.")
                lines.append("The corpus FN is caused by the target repo's configuration")
                lines.append("(Include/Exclude patterns, cop disabled, file outside scope,")
                lines.append("or `rubocop:disable` comment). Investigate config resolution.")
            else:
                lines.append("**NOT DETECTED — CODE BUG**")
                lines.append("The cop fails to detect this pattern. Fix the detection logic.")
                if d.get("enclosing"):
                    lines.append(f"\n**Enclosing structure:** {d['enclosing']}")
                    lines.append("The offense is inside this structure — the cop may need")
                    lines.append("to handle this context to detect the pattern.")
            lines.append(f"\nMessage: `{d['msg']}`")
            if d.get("test_snippet"):
                lines.append("\nReady-made test snippet (add to offense.rb, adjust `^` count):")
                lines.append(f"```ruby\n{d['test_snippet']}\n```")
            if d.get("source_context"):
                lines.append(f"\nFull source context:")
                lines.append(f"```ruby\n{d['source_context']}\n```")
        else:
            lines.append(f"(could not diagnose: {d.get('reason', 'unknown')})")
            lines.append(f"Message: `{d['msg']}`")
        lines.append("")

    # FP details
    for i, d in enumerate(fp_diags, 1):
        lines.append(f"### FP #{i}: `{d['loc']}`")
        if d.get("diagnosed"):
            if d.get("detected"):
                lines.append("**CONFIRMED false positive — CODE BUG**")
                lines.append("nitrocop incorrectly flags this pattern in isolation.")
                lines.append("Fix the detection logic to not flag this.")
                if d.get("enclosing"):
                    lines.append(f"\n**Enclosing structure:** {d['enclosing']}")
                    lines.append("The offense is inside this structure — this is likely WHY")
                    lines.append("RuboCop does not flag it. Your fix should detect this context.")
                if d.get("source_context"):
                    lines.append(f"\nFull source context (add relevant parts to no_offense.rb):")
                    lines.append(f"```ruby\n{d['source_context']}\n```")
                elif d.get("offense_line"):
                    lines.append(f"\nAdd to no_offense.rb:")
                    lines.append(f"```ruby\n{d['offense_line']}\n```")
            else:
                lines.append("**NOT REPRODUCED in isolation — CONTEXT-DEPENDENT**")
                lines.append("nitrocop does not flag this in isolation. The FP is triggered")
                lines.append("by surrounding code context or file-level state.")
                lines.append("Investigate what full-file context causes the false detection.")
                if d.get("source_context"):
                    lines.append(f"\nSource context:")
                    lines.append(f"```ruby\n{d['source_context']}\n```")
            lines.append(f"\nMessage: `{d['msg']}`")
        else:
            lines.append(f"(could not diagnose: {d.get('reason', 'unknown')})")
            lines.append(f"Message: `{d['msg']}`")
        lines.append("")

    # Additional un-diagnosed examples
    undiag_fn = fn_examples[len(fn_diags):]
    undiag_fp = fp_examples[len(fp_diags):]
    if undiag_fn or undiag_fp:
        lines.append("### Additional examples (not pre-diagnosed)\n")
        for ex in undiag_fn[:15]:
            loc, msg, src = _normalize_example(ex)
            lines.append(f"- FN: `{loc}` — {msg}")
            if src:
                lines.append("  ```ruby")
                for s in src:
                    lines.append(f"  {s}")
                lines.append("  ```")
        for ex in undiag_fp[:15]:
            loc, msg, src = _normalize_example(ex)
            lines.append(f"- FP: `{loc}` — {msg}")
            if src:
                lines.append("  ```ruby")
                for s in src:
                    lines.append(f"  {s}")
                lines.append("  ```")

    return "\n".join(lines)


def _format_without_diagnostics(cop: str, fp_examples: list, fn_examples: list) -> str:
    """Format FP/FN examples without diagnostic enrichment (no --binary)."""
    lines = []

    if fp_examples:
        lines.append(f"### False Positives ({len(fp_examples)} examples)")
        lines.append("nitrocop flags these but RuboCop does not:\n")
        for ex in fp_examples[:30]:
            loc, msg, src = _normalize_example(ex)
            lines.append(f"- `{loc}`")
            if msg:
                lines.append(f"  Message: {msg}")
            if src:
                lines.append("  ```ruby")
                for s in src:
                    lines.append(f"  {s}")
                lines.append("  ```")

    if fn_examples:
        lines.append(f"\n### False Negatives ({len(fn_examples)} examples)")
        lines.append("RuboCop flags these but nitrocop does not:\n")
        for ex in fn_examples[:30]:
            loc, msg, src = _normalize_example(ex)
            lines.append(f"- `{loc}`")
            if msg:
                lines.append(f"  Message: {msg}")
            if src:
                lines.append("  ```ruby")
                for s in src:
                    lines.append(f"  {s}")
                lines.append("  ```")

    return "\n".join(lines) if lines else "(no example locations in corpus data)"


def detect_prism_pitfalls(rust_source: str) -> list[str]:
    """Detect relevant Prism pitfalls from the cop's Rust source."""
    notes = []
    for keyword, note in PRISM_PITFALLS.items():
        if keyword in rust_source:
            notes.append(note)
    return notes


def generate_task(
    cop: str,
    input_path: Path | None = None,
    extended: bool = False,
    binary_path: Path | None = None,
) -> str:
    """Generate the full task markdown for a cop."""
    dept, name, snake = parse_cop_name(cop)
    dept_snake = dept_dir_name(dept)

    # Gather all sources
    rust_path = find_rust_source(dept, snake)
    rust_source = read_file_safe(rust_path)
    if rust_source is None:
        print(f"Error: Rust source not found at {rust_path}", file=sys.stderr)
        sys.exit(1)

    ruby_path = find_vendor_ruby_source(dept, snake)
    ruby_source = read_file_safe(ruby_path) if ruby_path else None

    spec_path = find_vendor_spec(dept, snake)
    spec_source = read_file_safe(spec_path) if spec_path else None
    spec_excerpts = extract_spec_excerpts(spec_source) if spec_source else None

    offense_fixture, no_offense_fixture = find_fixtures(dept, snake)
    corpus = get_corpus_data(cop, input_path, extended)

    # Run pre-diagnostic if binary is available
    diagnostics = None
    if binary_path and binary_path.exists():
        print(f"Running pre-diagnostic with {binary_path}...", file=sys.stderr)
        diagnostics = run_diagnostic(
            binary_path, cop,
            corpus["fp_examples"], corpus["fn_examples"],
        )
        n_diag = sum(1 for d in diagnostics if d.get("diagnosed"))
        print(f"  Diagnosed {n_diag}/{len(diagnostics)} examples", file=sys.stderr)

    # Classify diagnosed examples
    has_code_bugs = False
    has_config_issues = False
    if diagnostics:
        for d in diagnostics:
            if not d.get("diagnosed"):
                continue
            if d["kind"] == "fn" and not d.get("detected"):
                has_code_bugs = True
            elif d["kind"] == "fn" and d.get("detected"):
                has_config_issues = True
            elif d["kind"] == "fp" and d.get("detected"):
                has_code_bugs = True
            elif d["kind"] == "fp" and not d.get("detected"):
                has_config_issues = True

    # Detect Prism pitfalls
    pitfalls = detect_prism_pitfalls(rust_source)

    # Build the task document
    parts = []

    # Header
    parts.append(f"# Fix {cop} — {corpus['fp']} FP, {corpus['fn']} FN\n")

    # Instructions
    focus = "FP" if corpus["fp"] > corpus["fn"] else "FN" if corpus["fn"] > corpus["fp"] else "both FP and FN"
    parts.append(f"""## Instructions

You are fixing ONE cop in **nitrocop**, a Rust Ruby linter that uses Prism for parsing.

**Current state:** {corpus['matches']:,} matches, {corpus['fp']} false positives, {corpus['fn']} false negatives.
**Focus on:** {focus} ({"nitrocop flags code RuboCop does not" if corpus["fp"] > corpus["fn"] else "RuboCop flags code nitrocop misses" if corpus["fn"] > corpus["fp"] else "both directions"}).

### Workflow
1. Read the **Pre-diagnostic Results** and **Corpus FP/FN Examples** sections below first
2. Add a test case FIRST:
   - FN fix: add the missed pattern to `tests/fixtures/cops/{dept_snake}/{snake}/offense.rb` with `^` annotation
   - FP fix: add the false-positive pattern to `tests/fixtures/cops/{dept_snake}/{snake}/no_offense.rb`
3. Verify test fails: `cargo test --lib -- cop::{dept_snake}::{snake}`
4. Fix `src/cop/{dept_snake}/{snake}.rs`
5. Verify test passes: `cargo test --lib -- cop::{dept_snake}::{snake}`
6. Add a `///` doc comment on the cop struct documenting what you found and fixed
7. Commit only your cop's files

### Fixture Format
Mark offenses with `^` markers on the line AFTER the offending source line:
```
x = 1
     ^^ {cop}: Trailing whitespace detected.
```
The `^` characters must align with the offending columns. The message format is `{cop}: <message text>`.""")

    # Add diagnostic-aware guidance
    if diagnostics and has_config_issues and not has_code_bugs:
        parts.append(f"""
### IMPORTANT: This is a config/context issue, NOT a detection bug
Pre-diagnostic shows nitrocop already detects all FP/FN patterns correctly in isolation.
The corpus mismatches are caused by configuration differences in target repos.

**Do NOT loop trying to fix detection logic — the detection code is correct.**

Instead:
1. Investigate why the cop doesn't fire (FN) or fires incorrectly (FP) in the target
   repo's config context. Common causes:
   - Include/Exclude patterns in the cop's config not matching the file path
   - The cop being disabled by the target repo's `.rubocop.yml`
   - `# rubocop:disable` comments in the source file
   - File path patterns (e.g., spec files excluded by default)
2. Look at `src/config/` for how config affects this cop
3. If you can fix the config resolution, do so. Otherwise document your findings as a
   `///` comment on the cop struct and commit what you have.""")

    elif diagnostics and has_config_issues and has_code_bugs:
        parts.append(f"""
### Mixed issues: some code bugs, some config issues
Pre-diagnostic shows SOME patterns are correctly detected in isolation (config issues)
and SOME are genuinely missed (code bugs). See the per-example diagnosis below.

- For examples marked **CODE BUG**: follow the standard TDD workflow
- For examples marked **CONFIG/CONTEXT**: investigate config resolution, not detection logic""")

    parts.append(f"""
### If your test passes immediately
If you add a test case and it passes without code changes, the corpus mismatch is
caused by config/context differences, not a detection bug.
**Do NOT loop** trying to make the test fail. Instead:
1. Investigate config resolution (Include/Exclude, cop enablement, disable comments)
2. The fix is likely in `src/config/` or the cop's config handling, not detection logic
3. If you cannot determine the root cause within 5 minutes, document your findings as
   a `///` comment on the cop struct and commit

### Rules
- Only modify `src/cop/{dept_snake}/{snake}.rs` and `tests/fixtures/cops/{dept_snake}/{snake}/`
- Run `cargo test --lib -- cop::{dept_snake}::{snake}` to verify your fix (do NOT run the full test suite)
- Do NOT touch unrelated files
- Do NOT use `git stash`
""")

    # Prism pitfall notes
    if pitfalls:
        parts.append("### Prism Notes")
        for note in pitfalls:
            parts.append(f"- {note}")
        parts.append("")

    # Pre-diagnostic results (high-value: before source code)
    if diagnostics:
        parts.append("## Pre-diagnostic Results\n")
        parts.append(format_corpus_section(cop, corpus, diagnostics))
        parts.append("")
    else:
        # No diagnostics — put corpus examples in the usual place (at the end)
        pass

    # Rust source
    rust_rel = rust_path.relative_to(PROJECT_ROOT)
    parts.append(f"## Current Rust Implementation\n`{rust_rel}`\n")
    parts.append(f"```rust\n{rust_source}```\n")

    # RuboCop Ruby source
    if ruby_source and ruby_path:
        ruby_rel = ruby_path.relative_to(PROJECT_ROOT)
        parts.append(f"## RuboCop Ruby Implementation (ground truth)\n`{ruby_rel}`\n")
        parts.append(f"```ruby\n{ruby_source}```\n")

    # Spec excerpts
    if spec_excerpts and spec_path:
        spec_rel = spec_path.relative_to(PROJECT_ROOT)
        parts.append(f"## RuboCop Test Excerpts\n`{spec_rel}`\n")
        parts.append(f"```ruby\n{spec_excerpts}\n```\n")

    # Fixtures
    if offense_fixture:
        parts.append(f"## Current Fixture: offense.rb\n`tests/fixtures/cops/{dept_snake}/{snake}/offense.rb`\n")
        parts.append(f"```ruby\n{offense_fixture}```\n")

    if no_offense_fixture:
        parts.append(f"## Current Fixture: no_offense.rb\n`tests/fixtures/cops/{dept_snake}/{snake}/no_offense.rb`\n")
        parts.append(f"```ruby\n{no_offense_fixture}```\n")

    # Corpus data (without diagnostics, for fallback)
    if not diagnostics:
        parts.append("## Corpus FP/FN Examples\n")
        parts.append(format_corpus_section(cop, corpus, None))
        parts.append("")

    return "\n".join(parts)


def main():
    parser = argparse.ArgumentParser(
        description="Generate a self-contained task prompt for a remote agent to fix one cop")
    parser.add_argument("cop", help="Cop name (e.g., Style/NegatedWhile)")
    parser.add_argument("--output", "-o", type=Path,
                        help="Output file path (default: stdout)")
    parser.add_argument("--input", type=Path,
                        help="Path to corpus-results.json (default: download from CI)")
    parser.add_argument("--extended", action="store_true",
                        help="Use extended corpus (5k+ repos)")
    parser.add_argument("--binary", type=Path,
                        help="Path to nitrocop binary for pre-diagnostic "
                             "(runs cop on extracted source to classify FP/FN)")
    args = parser.parse_args()

    # Resolve binary to absolute path so it works from any cwd
    binary = args.binary.resolve() if args.binary else None

    task = generate_task(args.cop, args.input, args.extended, binary)

    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(task)
        print(f"Task written to {args.output}", file=sys.stderr)
    else:
        print(task)


if __name__ == "__main__":
    main()
