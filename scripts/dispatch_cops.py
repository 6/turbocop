#!/usr/bin/env python3
from __future__ import annotations

"""Dispatch-related corpus tooling.

Public subcommands:
- `task` generates a self-contained agent prompt for a single cop
- `changed` maps a git diff to changed cops
- `tiers` groups diverging cops into dispatch difficulty tiers
- `rank` finds cops that look fixable by agents
- `prior-attempts` collects failed PR attempts for a cop
- `backend` selects a recommended backend for a cop
- `issues-sync` syncs one tracker issue per diverging cop
- `dispatch-issues` fills the bounded active queue by dispatching tracker issues

Usage:
    python3 scripts/dispatch_cops.py task Style/NegatedWhile
    python3 scripts/dispatch_cops.py changed --base origin/main --head HEAD
    python3 scripts/dispatch_cops.py tiers --tier 1 --names
    python3 scripts/dispatch_cops.py rank --json
    python3 scripts/dispatch_cops.py prior-attempts --cop Style/NegatedWhile
    python3 scripts/dispatch_cops.py backend --cop Style/NegatedWhile --binary target/debug/nitrocop
    python3 scripts/dispatch_cops.py issues-sync --binary target/debug/nitrocop
    python3 scripts/dispatch_cops.py issues-sync --department Rails --binary target/debug/nitrocop
    python3 scripts/dispatch_cops.py dispatch-issues --max-active 5
    python3 scripts/dispatch_cops.py dispatch-issues --department Rails --max-active 3
"""

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

# Allow importing shared helpers from scripts/
SCRIPTS_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS_DIR))
from shared.corpus_artifacts import download_corpus_results as _download_corpus

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

MAX_DIAGNOSTIC_DETAILS_PER_KIND = 8
MAX_ADDITIONAL_EXAMPLES_PER_KIND = 5

COP_TRACKER_MARKER = "nitrocop-cop-tracker"
PR_ISSUE_MARKER = "nitrocop-cop-issue"
ISSUE_TITLE_PREFIX = "[cop] "
TRACKER_LABEL = "cop-tracker"
STATE_BACKLOG = "state:backlog"
STATE_PR_OPEN = "state:pr-open"
STATE_BLOCKED = "state:blocked"
STATE_LABELS = [STATE_BACKLOG, STATE_PR_OPEN, STATE_BLOCKED]
DIFFICULTY_LABELS = {
    "simple": "difficulty:simple",
    "medium": "difficulty:medium",
    "complex": "difficulty:complex",
}
LABEL_COLORS = {
    TRACKER_LABEL: "1d76db",
    STATE_BACKLOG: "fbca04",
    STATE_PR_OPEN: "0e8a16",
    STATE_BLOCKED: "b60205",
    "difficulty:simple": "0e8a16",
    "difficulty:medium": "fbca04",
    "difficulty:complex": "d73a4a",
}
TITLE_RE = re.compile(r"^\[bot\] Fix (?P<cop>.+?)(?: \(retry\))?$")
TRACKER_RE = re.compile(r"<!--\s*" + re.escape(COP_TRACKER_MARKER) + r":\s*(.*?)\s*-->")
PR_ISSUE_RE = re.compile(r"<!--\s*" + re.escape(PR_ISSUE_MARKER) + r":\s*(.*?)\s*-->")
MAX_GH_PAGE = 500


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


def make_issue_title(cop: str) -> str:
    return f"{ISSUE_TITLE_PREFIX}{cop}"


def parse_marker_fields(body: str, pattern: re.Pattern[str]) -> dict[str, str]:
    match = pattern.search(body or "")
    if not match:
        return {}
    fields: dict[str, str] = {}
    for token in match.group(1).split():
        if "=" not in token:
            continue
        key, value = token.split("=", 1)
        fields[key.strip()] = value.strip()
    return fields


def extract_cop_from_issue(issue: dict) -> str | None:
    body_fields = parse_marker_fields(issue.get("body", ""), TRACKER_RE)
    cop = body_fields.get("cop")
    if cop:
        return cop
    title = issue.get("title", "")
    if title.startswith(ISSUE_TITLE_PREFIX):
        return title[len(ISSUE_TITLE_PREFIX):].strip()
    return None


def extract_cop_from_pr(pr: dict) -> str | None:
    body_fields = parse_marker_fields(pr.get("body", ""), PR_ISSUE_RE)
    cop = body_fields.get("cop")
    if cop:
        return cop
    match = TITLE_RE.match(pr.get("title", "").strip())
    if match:
        return match.group("cop")
    return None


def extract_issue_number_from_pr(pr: dict) -> int | None:
    body_fields = parse_marker_fields(pr.get("body", ""), PR_ISSUE_RE)
    issue_number = body_fields.get("number")
    if issue_number and issue_number.isdigit():
        return int(issue_number)
    return None


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


def get_corpus_data(cop: str, input_path: Path | None) -> dict:
    """Get FP/FN data from corpus-results.json.

    Returns dict with counts and raw example lists."""
    if input_path is None:
        try:
            input_path, _, _ = _download_corpus()
        except Exception as e:
            print(f"Warning: could not download corpus data: {e}", file=sys.stderr)
            return {
                "fp": 0,
                "fn": 0,
                "matches": 0,
                "fp_examples": [],
                "fn_examples": [],
                "repo_breakdown": {},
                "available": False,
            }

    data = json.loads(input_path.read_text())
    by_cop = data.get("by_cop", [])
    by_repo_cop = data.get("by_repo_cop", {})
    cop_entry = next((e for e in by_cop if e["cop"] == cop), None)

    if cop_entry is None:
        return {
            "fp": 0,
            "fn": 0,
            "matches": 0,
            "fp_examples": [],
            "fn_examples": [],
            "repo_breakdown": {},
            "available": True,
        }

    repo_breakdown = {}
    for repo_id, cops in by_repo_cop.items():
        if cop not in cops:
            continue
        entry = cops[cop]
        repo_breakdown[repo_id] = {
            "fp": entry.get("fp", 0),
            "fn": entry.get("fn", 0),
        }

    return {
        "fp": cop_entry.get("fp", 0),
        "fn": cop_entry.get("fn", 0),
        "matches": cop_entry.get("matches", 0),
        "fp_examples": cop_entry.get("fp_examples", []),
        "fn_examples": cop_entry.get("fn_examples", []),
        "repo_breakdown": repo_breakdown,
        "available": True,
    }


def safe_get_corpus_data(
    cop: str,
    *,
    input_path: Path | None = None,
) -> dict | None:
    try:
        return get_corpus_data(cop, input_path)
    except Exception as exc:
        print(f"Warning: could not load corpus data: {exc}", file=sys.stderr)
        return None


def _normalize_example(ex) -> tuple[str, str, list[str] | None]:
    """Normalize an example to (loc_string, message, embedded_context)."""
    if isinstance(ex, dict):
        return ex.get("loc", ""), ex.get("msg", ""), ex.get("src")
    return ex, "", None


def _parse_example_loc(loc: str) -> tuple[str, str, int] | None:
    """Parse `repo_id: path/to/file.rb:line` into components."""
    if ": " not in loc:
        return None
    repo_id, rest = loc.split(": ", 1)
    last_colon = rest.rfind(":")
    if last_colon < 0:
        return None
    filepath = rest[:last_colon]
    try:
        line = int(rest[last_colon + 1:])
    except ValueError:
        return None
    return repo_id, filepath, line


def _top_repos(
    repo_breakdown: dict[str, dict[str, int]], kind: str, limit: int = 3,
) -> list[tuple[str, int]]:
    repos = [
        (repo_id, counts.get(kind, 0))
        for repo_id, counts in repo_breakdown.items()
        if counts.get(kind, 0) > 0
    ]
    repos.sort(key=lambda item: (-item[1], item[0]))
    return repos[:limit]


def _sample_examples_for_repo(examples: list, repo_id: str, limit: int = 1) -> list[str]:
    matches = []
    for ex in examples:
        loc, _msg, _src = _normalize_example(ex)
        parsed = _parse_example_loc(loc)
        if parsed is None:
            continue
        ex_repo_id, filepath, line = parsed
        if ex_repo_id != repo_id:
            continue
        matches.append(f"{filepath}:{line}")
        if len(matches) >= limit:
            break
    return matches


def build_start_here_section(cop: str, corpus: dict) -> str:
    """Build a compact investigation guide from corpus hotspots."""
    repo_breakdown = corpus.get("repo_breakdown", {})
    fp_examples = corpus.get("fp_examples", [])
    fn_examples = corpus.get("fn_examples", [])

    top_fp_repos = _top_repos(repo_breakdown, "fp")
    top_fn_repos = _top_repos(repo_breakdown, "fn")

    if not top_fp_repos and not top_fn_repos and not fp_examples and not fn_examples:
        return ""

    lines = [
        "## Start Here",
        "",
        "Use the existing corpus data to focus on the most concentrated regressions first.",
        "",
        "Helpful local commands:",
        f"- `python3 scripts/investigate_cop.py {cop} --repos-only`",
        f"- `python3 scripts/investigate_cop.py {cop} --context`",
        f"- `python3 scripts/verify_cop_locations.py {cop}`",
        "",
    ]

    if top_fp_repos:
        lines.append("Top FP repos:")
        for repo_id, count in top_fp_repos:
            samples = _sample_examples_for_repo(fp_examples, repo_id)
            suffix = f" — example `{samples[0]}`" if samples else ""
            lines.append(f"- `{repo_id}` ({count} FP){suffix}")
        lines.append("")

    if top_fn_repos:
        lines.append("Top FN repos:")
        for repo_id, count in top_fn_repos:
            samples = _sample_examples_for_repo(fn_examples, repo_id)
            suffix = f" — example `{samples[0]}`" if samples else ""
            lines.append(f"- `{repo_id}` ({count} FN){suffix}")
        lines.append("")

    if fp_examples:
        lines.append("Representative FP examples:")
        for ex in fp_examples[:3]:
            loc, msg, _src = _normalize_example(ex)
            msg_suffix = f" — {msg}" if msg else ""
            lines.append(f"- `{loc}`{msg_suffix}")
        lines.append("")

    if fn_examples:
        lines.append("Representative FN examples:")
        for ex in fn_examples[:3]:
            loc, msg, _src = _normalize_example(ex)
            msg_suffix = f" — {msg}" if msg else ""
            lines.append(f"- `{loc}`{msg_suffix}")
        lines.append("")

    return "\n".join(lines).rstrip() + "\n"


def corpus_total(corpus: dict | None) -> int:
    if not corpus:
        return 0
    return corpus.get("fp", 0) + corpus.get("fn", 0)


def affected_repo_count(corpus: dict | None) -> int:
    if not corpus:
        return 0
    return sum(
        1
        for counts in corpus.get("repo_breakdown", {}).values()
        if counts.get("fp", 0) > 0 or counts.get("fn", 0) > 0
    )



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


def _get_prism_node_type(source: str, line: int) -> str | None:
    """Use Ruby's Prism to identify the AST node type at a given line.

    Returns a string like 'RangeNode > IntegerNode' or None if Ruby/Prism
    is not available."""
    ruby_script = f"""
require 'prism'
result = Prism.parse({source!r})
target_line = {line}

# Walk the AST to find the deepest node at the target line
def find_at_line(node, line, path=[])
  return nil unless node.respond_to?(:child_nodes)
  loc = node.location rescue nil
  if loc && loc.start_line == line
    path << node.class.name.split('::').last
  end
  node.child_nodes.compact.each {{ |c| find_at_line(c, line, path) }}
  path
end

types = find_at_line(result.value, target_line)
puts types.join(' > ') unless types.empty?
"""
    try:
        proc = subprocess.run(
            ["ruby", "-e", ruby_script],
            capture_output=True, text=True, timeout=10,
        )
        out = proc.stdout.strip()
        return out if out else None
    except (FileNotFoundError, subprocess.TimeoutExpired):
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

                # Identify Prism node type at the offense location
                node_type = None
                if offense_line_idx is not None:
                    node_type = _get_prism_node_type(
                        "\n".join(source_lines), offense_line_idx + 1,
                    )

                results.append({
                    "kind": kind, "loc": loc, "msg": msg,
                    "diagnosed": True, "detected": detected,
                    "offense_line": offense_line,
                    "test_snippet": test_snippet,
                    "enclosing": enclosing,
                    "node_type": node_type,
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
    fn_diagnosed = [d for d in fn_diags if d.get("diagnosed")]
    fp_diagnosed = [d for d in fp_diags if d.get("diagnosed")]
    fn_undiagnosed = [d for d in fn_diags if not d.get("diagnosed")]
    fp_undiagnosed = [d for d in fp_diags if not d.get("diagnosed")]

    # Summary counts
    fn_code_bugs = sum(1 for d in fn_diagnosed if not d.get("detected"))
    fn_config = sum(1 for d in fn_diagnosed if d.get("detected"))
    fp_code_bugs = sum(1 for d in fp_diagnosed if d.get("detected"))
    fp_config = sum(1 for d in fp_diagnosed if not d.get("detected"))

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
    if fn_diagnosed and fn_undiagnosed:
        no_context = sum(1 for d in fn_undiagnosed if d.get("reason") == "no source context")
        extra = len(fn_undiagnosed) - no_context
        if no_context:
            lines.append(
                f"- Omitted {no_context} pre-diagnostic FN example(s) with no source context "
                "because diagnosed FN examples were available"
            )
        if extra:
            lines.append(f"- Omitted {extra} additional undiagnosed FN example(s) for brevity")
    if fp_diagnosed and fp_undiagnosed:
        no_context = sum(1 for d in fp_undiagnosed if d.get("reason") == "no source context")
        extra = len(fp_undiagnosed) - no_context
        if no_context:
            lines.append(
                f"- Omitted {no_context} pre-diagnostic FP example(s) with no source context "
                "because diagnosed FP examples were available"
            )
        if extra:
            lines.append(f"- Omitted {extra} additional undiagnosed FP example(s) for brevity")
    lines.append("")

    # FN details
    fn_display = fn_diagnosed[:MAX_DIAGNOSTIC_DETAILS_PER_KIND]
    if not fn_display:
        fn_display = fn_undiagnosed[:MAX_ADDITIONAL_EXAMPLES_PER_KIND]
    for i, d in enumerate(fn_display, 1):
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
                if d.get("node_type"):
                    lines.append(f"\n**Prism AST at offense line:** `{d['node_type']}`")
            lines.append(f"\nMessage: `{d['msg']}`")
            if d.get("test_snippet"):
                lines.append("\nReady-made test snippet (add to offense.rb, adjust `^` count):")
                lines.append(f"```ruby\n{d['test_snippet']}\n```")
            if d.get("source_context"):
                lines.append("\nFull source context:")
                lines.append(f"```ruby\n{d['source_context']}\n```")
        else:
            lines.append(f"(could not diagnose: {d.get('reason', 'unknown')})")
            lines.append(f"Message: `{d['msg']}`")
        lines.append("")
    if len(fn_diagnosed) > len(fn_display):
        omitted = len(fn_diagnosed) - len(fn_display)
        lines.append(f"_Omitted {omitted} additional diagnosed FN example(s) for brevity._")
        lines.append("")
    elif not fn_diagnosed and len(fn_undiagnosed) > len(fn_display):
        omitted = len(fn_undiagnosed) - len(fn_display)
        lines.append(f"_Omitted {omitted} additional undiagnosed FN example(s) for brevity._")
        lines.append("")

    # FP details
    fp_display = fp_diagnosed[:MAX_DIAGNOSTIC_DETAILS_PER_KIND]
    if not fp_display:
        fp_display = fp_undiagnosed[:MAX_ADDITIONAL_EXAMPLES_PER_KIND]
    for i, d in enumerate(fp_display, 1):
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
                if d.get("node_type"):
                    lines.append(f"\n**Prism AST at offense line:** `{d['node_type']}`")
                    lines.append("This shows the Prism node types at the flagged location.")
                if d.get("source_context"):
                    lines.append("\nFull source context (add relevant parts to no_offense.rb):")
                    lines.append(f"```ruby\n{d['source_context']}\n```")
                elif d.get("offense_line"):
                    lines.append("\nAdd to no_offense.rb:")
                    lines.append(f"```ruby\n{d['offense_line']}\n```")
            else:
                lines.append("**NOT REPRODUCED in isolation — CONTEXT-DEPENDENT**")
                lines.append("nitrocop does not flag this in isolation. The FP is triggered")
                lines.append("by surrounding code context or file-level state.")
                lines.append("Investigate what full-file context causes the false detection.")
                if d.get("source_context"):
                    lines.append("\nSource context:")
                    lines.append(f"```ruby\n{d['source_context']}\n```")
            lines.append(f"\nMessage: `{d['msg']}`")
        else:
            lines.append(f"(could not diagnose: {d.get('reason', 'unknown')})")
            lines.append(f"Message: `{d['msg']}`")
        lines.append("")
    if len(fp_diagnosed) > len(fp_display):
        omitted = len(fp_diagnosed) - len(fp_display)
        lines.append(f"_Omitted {omitted} additional diagnosed FP example(s) for brevity._")
        lines.append("")
    elif not fp_diagnosed and len(fp_undiagnosed) > len(fp_display):
        omitted = len(fp_undiagnosed) - len(fp_display)
        lines.append(f"_Omitted {omitted} additional undiagnosed FP example(s) for brevity._")
        lines.append("")

    # Additional un-diagnosed examples
    undiag_fn = [] if fn_diagnosed else fn_examples[len(fn_diags):]
    undiag_fp = [] if fp_diagnosed else fp_examples[len(fp_diags):]
    if undiag_fn or undiag_fp:
        lines.append("### Additional examples (not pre-diagnosed)\n")
        for ex in undiag_fn[:MAX_ADDITIONAL_EXAMPLES_PER_KIND]:
            loc, msg, src = _normalize_example(ex)
            lines.append(f"- FN: `{loc}` — {msg}")
            if src:
                lines.append("  ```ruby")
                for s in src:
                    lines.append(f"  {s}")
                lines.append("  ```")
        for ex in undiag_fp[:MAX_ADDITIONAL_EXAMPLES_PER_KIND]:
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
    corpus = get_corpus_data(cop, input_path)

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
2. **Verify with RuboCop first** (for FP fixes): before writing any code, confirm RuboCop's
   behavior on BOTH the specific FP case AND the general pattern:
   ```bash
   echo '<specific FP case>' > /tmp/test.rb && rubocop --only {cop} /tmp/test.rb
   echo '<general pattern>' > /tmp/test.rb && rubocop --only {cop} /tmp/test.rb
   ```
   If RuboCop flags the general pattern, your fix must be narrow enough to not suppress it.
3. Add a test case FIRST:
   - FN fix: add the missed pattern to `tests/fixtures/cops/{dept_snake}/{snake}/offense.rb` with `^` annotation
   - FP fix: add the false-positive pattern to `tests/fixtures/cops/{dept_snake}/{snake}/no_offense.rb`
4. Verify test fails: `cargo test --lib -- cop::{dept_snake}::{snake}`
5. Fix `src/cop/{dept_snake}/{snake}.rs`
6. Verify test passes: `cargo test --lib -- cop::{dept_snake}::{snake}`
7. Add a `///` doc comment on the cop struct documenting what you found and fixed
8. Commit only your cop's files

### Fixture Format
Mark offenses with `^` markers on the line AFTER the offending source line:
```
x = 1
     ^^ {cop}: Trailing whitespace detected.
```
The `^` characters must align with the offending columns. The message format is `{cop}: <message text>`.""")

    # Add diagnostic-aware guidance
    if diagnostics and has_config_issues and not has_code_bugs:
        parts.append("""
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
        parts.append("""
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

### CRITICAL: Avoid regressions in the opposite direction
When fixing FPs, your change MUST NOT suppress legitimate detections. When fixing FNs,
your change MUST NOT flag code that RuboCop accepts. A fix that eliminates a few issues
in one direction but introduces hundreds in the other is a catastrophic regression.

**Before exempting a category of patterns**, verify with RuboCop that the general case
is still an offense:
```bash
rubocop --only {cop} /tmp/test.rb
```
If RuboCop flags the general pattern but not your specific case, the difference is in
a narrow context (e.g., enclosing structure, receiver type, argument count) — your fix
must target that specific context, not the broad category.

**Rule of thumb:** if your fix adds an early `return` or `continue` that skips a whole
node type, operator class, or naming pattern, it's probably too broad. Prefer adding a
condition that matches the SPECIFIC differentiating context.

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

    start_here = build_start_here_section(cop, corpus)
    if start_here:
        parts.append(start_here)

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


DEPT_OVERRIDES = {
    "rspec": "RSpec",
    "rspec_rails": "RSpecRails",
    "factory_bot": "FactoryBot",
}

TIER_THRESHOLDS = {
    1: (1, 50),
    2: (51, 1000),
    3: (1001, 999999),
}


def snake_to_pascal(name: str) -> str:
    return "".join(word.capitalize() for word in name.split("_"))


def dept_snake_to_pascal(name: str) -> str:
    return DEPT_OVERRIDES.get(name, snake_to_pascal(name))


def detect_cops(base: str, head: str) -> list[str]:
    """Find all cop names affected by changes between base and head."""
    result = subprocess.run(
        ["git", "diff", "--name-only", f"{base}...{head}"],
        capture_output=True,
        text=True,
        check=True,
    )
    changed = result.stdout.strip().splitlines()

    cops = set()
    for path in changed:
        match = re.match(r"src/cop/([^/]+)/([^/]+)\.rs$", path)
        if match:
            dept, name = match.group(1), match.group(2)
            if name not in {"mod", "node_type"}:
                cops.add(f"{dept_snake_to_pascal(dept)}/{snake_to_pascal(name)}")
            continue

        match = re.match(r"tests/fixtures/cops/([^/]+)/([^/]+)/", path)
        if match:
            dept, name = match.group(1), match.group(2)
            cops.add(f"{dept_snake_to_pascal(dept)}/{snake_to_pascal(name)}")

    return sorted(cops)


def load_dispatch_corpus(input_path: Path | None) -> dict:
    if input_path:
        return json.loads(input_path.read_text())
    path, _, _ = _download_corpus()
    return json.loads(path.read_text())


def tier_cops(data: dict) -> tuple[list[dict], dict[int, list[dict]]]:
    cops = []
    for entry in data.get("by_cop", []):
        fp = entry.get("fp", 0)
        fn = entry.get("fn", 0)
        total = fp + fn
        if total == 0:
            continue
        cops.append({
            "cop": entry["cop"],
            "fp": fp,
            "fn": fn,
            "total": total,
            "matches": entry.get("matches", 0),
            "match_rate": entry.get("match_rate", 0),
        })

    cops.sort(key=lambda entry: entry["total"])
    tiers: dict[int, list[dict]] = {1: [], 2: [], 3: []}
    for cop in cops:
        for tier, (low, high) in TIER_THRESHOLDS.items():
            if low <= cop["total"] <= high:
                tiers[tier].append(cop)
                break
    return cops, tiers


def tier_for_total(total: int) -> int:
    for tier, (low, high) in TIER_THRESHOLDS.items():
        if low <= total <= high:
            return tier
    return 3


def total_for_entry(entry: dict) -> int:
    return entry.get("total", entry.get("fp", 0) + entry.get("fn", 0))


def backend_family(backend: str) -> str:
    if backend.startswith("codex-"):
        return "codex"
    if backend.startswith("claude-"):
        return "claude"
    if backend == "minimax":
        return "minimax"
    return backend


def backend_strength(backend: str) -> str:
    if backend.endswith("-hard"):
        return "hard"
    return "normal"


def backend_display_label(backend: str) -> str:
    return f"{backend_family(backend)} / {backend_strength(backend)}"


def should_consider_easy_candidate(
    entry: dict, min_total: int = 3, max_total: int = 15, min_matches: int = 50,
) -> bool:
    total = total_for_entry(entry)
    return min_total <= total <= max_total and entry.get("matches", 0) >= min_matches



def select_backend_for_entry(
    cop: str,
    entry: dict | None,
    *,
    mode: str,
    binary: Path | None,
    prior_prs: list[dict] | None = None,
    issue_difficulty: str | None = None,
    min_total: int = 3,
    max_total: int = 15,
    min_matches: int = 50,
    min_bugs: int = 1,
) -> dict[str, object]:
    prior_prs = prior_prs or []
    total = total_for_entry(entry or {})
    tier = tier_for_total(total) if total else 3

    def _result(backend: str, reason: str, code_bugs: int = 0,
                config_issues: int = 0, easy: bool = False) -> dict[str, object]:
        return {
            "backend": backend,
            "reason": reason,
            "tier": tier,
            "code_bugs": code_bugs,
            "config_issues": config_issues,
            "easy": easy,
        }

    # Routing strategy:
    # - codex-normal:        easy mechanical fixes with confirmed code bugs (fast, cheap)
    # - codex-hard:          complex cops with many divergences, or medium difficulty
    # - claude-oauth-normal: config/parser-only issues needing investigation judgment
    # - claude-oauth-hard:   retries, prior failures

    # Retries and prior failures need fresh thinking, not more brute force
    if mode == "retry":
        return _result("claude-oauth-hard", "retry mode needs fresh investigation approach")

    # Explicit issue difficulty labels
    if issue_difficulty:
        if issue_difficulty == "simple":
            return _result("codex-normal", "issue difficulty label is simple", easy=True)
        if issue_difficulty == "complex":
            return _result("claude-oauth-hard", f"issue difficulty label is {issue_difficulty}")
        # medium → codex-hard (mechanical but needs more reasoning)
        return _result("codex-hard", f"issue difficulty label is {issue_difficulty}")

    if not entry:
        return _result("codex-hard", "cop is missing from corpus data")

    # Run prediagnosis to classify code bugs vs config/parser issues
    code_bugs = 0
    config_issues = 0
    if binary and binary.exists() and should_consider_easy_candidate(
        entry, min_total=min_total, max_total=max_total, min_matches=min_matches,
    ):
        fn_bugs, fn_cfg = diagnose_examples(binary, cop, entry.get("fn_examples", []), "fn")
        fp_bugs, fp_cfg = diagnose_examples(binary, cop, entry.get("fp_examples", []), "fp")
        code_bugs = fn_bugs + fp_bugs
        config_issues = fn_cfg + fp_cfg

        if code_bugs >= min_bugs:
            # Confirmed code bugs in an easy cop — codex handles these well
            return _result(
                "codex-normal",
                f"easy cop: total={total_for_entry(entry)}, matches={entry.get('matches', 0)}, "
                f"diagnosed_code_bugs={code_bugs}",
                code_bugs=code_bugs, config_issues=config_issues, easy=True,
            )

        if config_issues > 0 and code_bugs == 0:
            # All issues are config/parser-level, not cop bugs — claude
            # has better judgment on whether to fix or document
            return _result(
                "claude-oauth-normal",
                f"all {config_issues} issues are config/parser-level, not code bugs; "
                f"needs investigation to determine correct action",
                code_bugs=code_bugs, config_issues=config_issues,
            )

    # Complex cop (outside easy thresholds) or no binary for prediagnosis —
    # codex-hard handles high-volume mechanical work well
    if total > max_total:
        return _result(
            "codex-hard",
            f"complex cop with many divergences (total={total})",
            code_bugs=code_bugs, config_issues=config_issues,
        )

    # Default: moderate complexity, codex-hard for mechanical work
    return _result(
        "codex-hard",
        f"moderate cop (total={total}, matches={entry.get('matches', 0)}); "
        f"code_bugs={code_bugs}, config_issues={config_issues}",
        code_bugs=code_bugs, config_issues=config_issues,
    )


def classify_issue_difficulty(entry: dict, recommendation: dict[str, object]) -> str:
    if recommendation.get("easy"):
        return "simple"
    if tier_for_total(total_for_entry(entry)) >= 3:
        return "complex"
    return "medium"


def run_nitrocop(binary: Path, cwd: str, cop: str) -> list[dict]:
    proc = subprocess.run(
        [str(binary), "--force-default-config", "--only", cop, "--format", "json", "test.rb"],
        capture_output=True,
        text=True,
        timeout=15,
        cwd=cwd,
    )
    if proc.stdout.strip():
        try:
            return json.loads(proc.stdout).get("offenses", [])
        except json.JSONDecodeError:
            pass
    return []


def extract_diagnostic_lines(src: list[str]) -> tuple[list[str], str | None]:
    lines, offense = [], None
    for source_line in src:
        is_offense = source_line.strip().startswith(">>>")
        cleaned = re.sub(r"^(>>>\s*)?\s*\d+:\s?", "", source_line)
        lines.append(cleaned)
        if is_offense:
            offense = cleaned
    return lines, offense


def diagnose_examples(binary: Path, cop: str, examples: list, kind: str) -> tuple[int, int]:
    bugs, config_issues = 0, 0
    for example in examples[:5]:
        if not isinstance(example, dict) or not example.get("src"):
            continue
        lines, offense = extract_diagnostic_lines(example["src"])
        if not lines:
            continue
        tmp = tempfile.mkdtemp()
        try:
            with open(os.path.join(tmp, "test.rb"), "w") as file_handle:
                file_handle.write("\n".join(lines) + "\n")
            offenses = run_nitrocop(binary, tmp, cop)
            if not offenses and offense:
                with open(os.path.join(tmp, "test.rb"), "w") as file_handle:
                    file_handle.write(offense + "\n")
                offenses = run_nitrocop(binary, tmp, cop)
            detected = len(offenses) > 0
            if (kind == "fn" and not detected) or (kind == "fp" and detected):
                bugs += 1
            else:
                config_issues += 1
        except Exception:
            pass
        finally:
            try:
                os.unlink(os.path.join(tmp, "test.rb"))
                os.rmdir(tmp)
            except OSError:
                pass
    return bugs, config_issues


def run_gh(args: list[str], check: bool = True) -> str:
    result = subprocess.run(
        ["gh", *args],
        capture_output=True,
        text=True,
        check=check,
    )
    return result.stdout.strip()


def ensure_labels(repo: str) -> None:
    for label, color in LABEL_COLORS.items():
        subprocess.run(
            ["gh", "label", "create", label, "--repo", repo, "--color", color],
            capture_output=True,
            text=True,
            check=False,
        )


def list_agent_fix_prs(repo: str, state: str = "all") -> list[dict]:
    try:
        output = run_gh([
            "pr",
            "list",
            "--repo",
            repo,
            "--state",
            state,
            "--label",
            "agent-fix",
            "--limit",
            str(MAX_GH_PAGE),
            "--json",
            "number,title,state,url,headRefName,mergedAt,closedAt,body",
        ])
    except subprocess.CalledProcessError:
        return []
    return json.loads(output) if output else []


def index_prs_by_cop(prs: list[dict]) -> dict[str, list[dict]]:
    by_cop: dict[str, list[dict]] = {}
    for pr in prs:
        cop = extract_cop_from_pr(pr)
        if not cop:
            continue
        by_cop.setdefault(cop, []).append(pr)
    return by_cop


def list_tracker_issues(repo: str) -> list[dict]:
    try:
        output = run_gh([
            "issue",
            "list",
            "--repo",
            repo,
            "--state",
            "all",
            "--label",
            TRACKER_LABEL,
            "--limit",
            str(MAX_GH_PAGE),
            "--json",
            "number,title,state,url,body,labels",
        ])
    except subprocess.CalledProcessError:
        return []
    return json.loads(output) if output else []


def index_issues_by_cop(issues: list[dict]) -> dict[str, dict]:
    by_cop: dict[str, dict] = {}
    for issue in issues:
        cop = extract_cop_from_issue(issue)
        if not cop:
            continue
        by_cop[cop] = issue
    return by_cop


def issue_label_names(issue: dict) -> set[str]:
    return {label["name"] for label in issue.get("labels", [])}


def choose_issue_state(existing_issue: dict | None, has_open_pr: bool) -> str:
    if has_open_pr:
        return STATE_PR_OPEN
    if existing_issue and STATE_BLOCKED in issue_label_names(existing_issue):
        return STATE_BLOCKED
    return STATE_BACKLOG


def render_tracker_marker(
    cop: str, fp: int, fn: int, matches: int, difficulty: str,
) -> str:
    total = fp + fn
    return (
        f"<!-- {COP_TRACKER_MARKER}: cop={cop} fp={fp} fn={fn} "
        f"total={total} matches={matches} difficulty={difficulty} -->"
    )


def render_issue_body(
    cop: str,
    entry: dict,
    *,
    repo: str,
    difficulty: str,
    state_label: str,
    open_pr: dict | None,
    corpus_kind: str,
    run_id: str | None,
    head_sha: str | None,
) -> str:
    fp = entry.get("fp", 0)
    fn = entry.get("fn", 0)
    matches = entry.get("matches", 0)
    total = fp + fn
    lines = [
        f"# {make_issue_title(cop)}",
        "",
        "This issue is managed by the corpus dispatch backlog automation.",
        "",
        "## Corpus Status",
        "",
        f"- Cop: `{cop}`",
        f"- Corpus: `{corpus_kind}`",
        f"- False positives: `{fp}`",
        f"- False negatives: `{fn}`",
        f"- Total divergence: `{total}`",
        f"- Matches: `{matches}`",
        f"- Difficulty: `{difficulty}`",
        f"- Current state label: `{state_label}`",
    ]
    if run_id:
        lines.append(f"- Source run: [#{run_id}](https://github.com/{repo}/actions/runs/{run_id})")
    if head_sha:
        lines.append(f"- Corpus head SHA: `{head_sha}`")
    if open_pr:
        lines.append(f"- Open bot PR: #{open_pr['number']} ({open_pr['url']})")
    lines.extend([
        "",
        "## Automation Notes",
        "",
        "- This issue stays open while the cop still diverges or a bot PR is active.",
        "- If a later corpus run shows the cop diverging again after merge, this same issue should be reopened and reused.",
        "",
        render_tracker_marker(cop, fp, fn, matches, difficulty),
        "",
    ])
    return "\n".join(lines)


def sync_issue_labels(repo: str, issue_number: int, labels: list[str]) -> None:
    remove = STATE_LABELS + list(DIFFICULTY_LABELS.values())
    subprocess.run(
        [
            "gh", "issue", "edit", str(issue_number),
            "--repo", repo,
            "--remove-label", ",".join(remove),
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    subprocess.run(
        [
            "gh", "issue", "edit", str(issue_number),
            "--repo", repo,
            "--add-label", ",".join([TRACKER_LABEL, *labels]),
        ],
        capture_output=True,
        text=True,
        check=True,
    )


def find_prior_prs(cop: str) -> list[dict]:
    try:
        output = run_gh([
            "pr",
            "list",
            "--state",
            "all",
            "--search",
            f"{cop} in:title",
            "--json",
            "number,title,state,headRefName,mergedAt,closedAt,url",
            "--limit",
            "20",
        ])
    except subprocess.CalledProcessError:
        return []

    prs = json.loads(output) if output else []
    cop_lower = cop.lower().replace("/", "")
    return [
        pr for pr in prs
        if cop_lower in pr.get("title", "").lower().replace("/", "").replace(" ", "")
        or cop_lower in pr.get("headRefName", "").lower().replace("-", "").replace("_", "")
    ]


def get_pr_diff(pr_number: int) -> str:
    try:
        return run_gh(["pr", "diff", str(pr_number)], check=False)
    except Exception:
        return "(could not fetch diff)"


def get_pr_checks(pr_number: int) -> str:
    try:
        return run_gh(["pr", "checks", str(pr_number)], check=False)
    except Exception:
        return "(could not fetch checks)"


def get_failed_check_logs(pr_number: int) -> str:
    try:
        checks_json = run_gh([
            "pr",
            "checks",
            str(pr_number),
            "--json",
            "name,state,detailsUrl",
        ], check=False)
        if not checks_json:
            return ""

        checks = json.loads(checks_json)
        failed = [check for check in checks if check.get("state") in ("FAILURE", "ERROR")]
        if not failed:
            return ""

        logs = []
        for check in failed[:3]:
            name = check.get("name", "unknown")
            url = check.get("detailsUrl", "")
            if "/actions/runs/" not in url:
                logs.append(f"#### {name} (FAILED)\nSee: {url}")
                continue

            run_id = url.split("/actions/runs/")[1].split("/")[0]
            try:
                log = run_gh(["run", "view", run_id, "--log-failed"], check=False)
            except Exception:
                log = ""
            if not log:
                logs.append(f"#### {name} (FAILED)\nSee: {url}")
                continue

            log_lines = log.splitlines()
            if len(log_lines) > 100:
                log = "\n".join(
                    ["... (truncated, showing last 100 lines) ...", *log_lines[-100:]]
                )
            logs.append(f"#### {name} (FAILED)\n```\n{log}\n```")

        return "\n\n".join(logs)
    except Exception:
        return ""


def collect_attempts(cop: str) -> str:
    prs = find_prior_prs(cop)
    if not prs:
        return ""

    prs.sort(key=lambda pr: pr.get("number", 0))
    failed = [pr for pr in prs if not pr.get("mergedAt")]
    merged = [pr for pr in prs if pr.get("mergedAt")]
    if not failed and not merged:
        return ""

    parts = [f"## Prior Attempts ({len(failed)} failed, {len(merged)} merged)", ""]
    for index, pr in enumerate(failed, 1):
        number = pr["number"]
        parts.extend([
            f"### Attempt {index}: PR #{number} ({pr.get('state', '')})",
            f"Title: {pr.get('title', '')}",
            f"URL: {pr.get('url', '')}",
            "",
        ])

        diff = get_pr_diff(number)
        if diff and diff != "(could not fetch diff)":
            diff_lines = diff.splitlines()
            if len(diff_lines) > 80:
                diff = "\n".join([*diff_lines[:80], f"... ({len(diff_lines) - 80} more lines truncated)"])
            parts.extend(["#### What was changed", f"```diff\n{diff}\n```", ""])

        checks = get_pr_checks(number)
        if checks:
            parts.extend(["#### CI Status", f"```\n{checks}\n```", ""])

        logs = get_failed_check_logs(number)
        if logs:
            parts.extend(["#### Failure Logs", logs, ""])

    if merged:
        parts.append("### Previously Merged PRs (for reference)")
        parts.extend(
            f"- PR #{pr['number']}: {pr.get('title', '')} ({pr.get('url', '')})"
            for pr in merged
        )
        parts.append("")

    return "\n".join(parts)


def build_entry_index(data: dict) -> dict[str, dict]:
    return {entry["cop"]: entry for entry in data.get("by_cop", [])}


def comment_on_issue(repo: str, issue_number: int, body: str) -> None:
    subprocess.run(
        ["gh", "issue", "comment", str(issue_number), "--repo", repo, "--body", body],
        capture_output=True,
        text=True,
        check=True,
    )


def create_tracker_issue(repo: str, title: str, body: str, labels: list[str]) -> int:
    output = run_gh([
        "issue",
        "create",
        "--repo",
        repo,
        "--title",
        title,
        "--body",
        body,
        "--label",
        ",".join([TRACKER_LABEL, *labels]),
    ])
    match = re.search(r"/issues/(\d+)$", output.strip())
    if not match:
        raise RuntimeError(f"Could not parse issue number from gh issue create output: {output}")
    return int(match.group(1))


def update_tracker_issue(repo: str, issue_number: int, title: str, body: str, labels: list[str]) -> None:
    subprocess.run(
        [
            "gh", "issue", "edit", str(issue_number),
            "--repo", repo,
            "--title", title,
            "--body", body,
        ],
        capture_output=True,
        text=True,
        check=True,
    )
    sync_issue_labels(repo, issue_number, labels)


def close_tracker_issue(repo: str, issue_number: int, body: str) -> None:
    subprocess.run(
        [
            "gh", "issue", "close", str(issue_number),
            "--repo", repo,
            "--comment", body,
        ],
        capture_output=True,
        text=True,
        check=True,
    )


def reopen_tracker_issue(repo: str, issue_number: int) -> None:
    subprocess.run(
        ["gh", "issue", "reopen", str(issue_number), "--repo", repo],
        capture_output=True,
        text=True,
        check=True,
    )


def fetch_corpus_for_sync(input_path: Path | None) -> tuple[dict, str | None, str | None]:
    if input_path:
        return json.loads(input_path.read_text()), None, None
    path, run_id, head_sha = _download_corpus()
    return json.loads(path.read_text()), str(run_id), head_sha



def cmd_backend(args: argparse.Namespace) -> int:
    data = load_dispatch_corpus(args.input)
    entry = build_entry_index(data).get(args.cop)
    prior_prs = index_prs_by_cop(list_agent_fix_prs(args.repo, state="all")).get(args.cop, [])
    binary = args.binary.resolve() if args.binary else None
    recommendation = select_backend_for_entry(
        args.cop,
        entry,
        mode=args.mode,
        binary=binary,
        prior_prs=prior_prs,
        issue_difficulty=args.issue_difficulty,
    )
    print(f"backend={recommendation['backend']}")
    print(f"family={backend_family(recommendation['backend'])}")
    print(f"strength={backend_strength(recommendation['backend'])}")
    print(f"display_label={backend_display_label(recommendation['backend'])}")
    print(f"reason={recommendation['reason']}")
    print(f"tier={recommendation['tier']}")
    print(f"code_bugs={recommendation['code_bugs']}")
    print(f"config_issues={recommendation['config_issues']}")
    print(f"easy={'true' if recommendation['easy'] else 'false'}")
    return 0


def cmd_issues_sync(args: argparse.Namespace) -> int:
    repo = args.repo
    ensure_labels(repo)
    data, run_id, head_sha = fetch_corpus_for_sync(args.input)
    corpus_kind = "corpus" if not args.input else "custom"
    entries = build_entry_index(data)
    issues = list_tracker_issues(repo)
    issues_by_cop = index_issues_by_cop(issues)
    prs_by_cop = index_prs_by_cop(list_agent_fix_prs(repo, state="all"))
    open_prs_by_cop = {
        cop: next((pr for pr in prs if pr.get("state") == "OPEN"), None)
        for cop, prs in prs_by_cop.items()
    }
    binary = args.binary.resolve() if args.binary else None
    diverging_cops = {cop for cop, entry in entries.items() if total_for_entry(entry) > 0}

    # Filter by department if requested
    dept_filter = args.department
    if dept_filter:
        diverging_cops = {cop for cop in diverging_cops if cop.startswith(dept_filter + "/")}

    created = updated = reopened = closed = 0

    for cop in sorted(diverging_cops):
        entry = entries[cop]
        prior_prs = prs_by_cop.get(cop, [])
        recommendation = select_backend_for_entry(
            cop,
            entry,
            mode="fix",
            binary=binary,
            prior_prs=prior_prs,
        )
        difficulty = classify_issue_difficulty(entry, recommendation)
        open_pr = open_prs_by_cop.get(cop)
        existing_issue = issues_by_cop.get(cop)
        state_label = choose_issue_state(existing_issue, open_pr is not None)
        labels = [state_label, DIFFICULTY_LABELS[difficulty]]
        title = make_issue_title(cop)
        body = render_issue_body(
            cop,
            entry,
            repo=repo,
            difficulty=difficulty,
            state_label=state_label,
            open_pr=open_pr,
            corpus_kind=corpus_kind,
            run_id=run_id,
            head_sha=head_sha,
        )

        if existing_issue is None:
            create_tracker_issue(repo, title, body, labels)
            created += 1
            continue

        if existing_issue.get("state") == "CLOSED":
            reopen_tracker_issue(repo, existing_issue["number"])
            comment_on_issue(
                repo,
                existing_issue["number"],
                (
                    f"Reopened from the latest {corpus_kind} corpus sync"
                    + (f" (run #{run_id})." if run_id else ".")
                ),
            )
            reopened += 1

        update_tracker_issue(repo, existing_issue["number"], title, body, labels)
        updated += 1

    for cop, issue in issues_by_cop.items():
        if cop in diverging_cops:
            continue
        # When filtering by department, don't close issues outside the filter scope
        if dept_filter and not cop.startswith(dept_filter + "/"):
            continue
        open_pr = open_prs_by_cop.get(cop)
        if open_pr is not None or issue.get("state") != "OPEN":
            continue
        close_tracker_issue(
            repo,
            issue["number"],
            (
                f"No longer diverges in the latest {corpus_kind} corpus sync"
                + (f" (run #{run_id})." if run_id else ".")
            ),
        )
        closed += 1

    print(
        json.dumps(
            {
                "created": created,
                "updated": updated,
                "reopened": reopened,
                "closed": closed,
                "diverging_cops": len(diverging_cops),
            },
            indent=2,
        )
    )
    return 0


def active_agent_fix_count(repo: str) -> tuple[int, int, int]:
    open_prs = list_agent_fix_prs(repo, state="open")
    open_count = len(open_prs)
    try:
        runs_json = run_gh([
            "api",
            f"repos/{repo}/actions/workflows/agent-cop-fix.yml/runs?per_page=100",
        ])
    except subprocess.CalledProcessError:
        return open_count, 0, open_count

    runs = json.loads(runs_json or "{}").get("workflow_runs", [])
    in_progress = sum(
        1
        for run in runs
        if run.get("status") in {"queued", "in_progress"}
    )
    return open_count, in_progress, max(open_count, in_progress)


def sorted_dispatch_candidates(issues: list[dict]) -> list[dict]:
    def key(issue: dict) -> tuple[int, int, str]:
        fields = parse_marker_fields(issue.get("body", ""), TRACKER_RE)
        difficulty = fields.get("difficulty", "complex")
        difficulty_rank = {"simple": 0, "medium": 1, "complex": 2}.get(difficulty, 2)
        total = int(fields.get("total", "999999"))
        cop = extract_cop_from_issue(issue) or issue.get("title", "")
        return difficulty_rank, total, cop

    return sorted(issues, key=key)


def _main_checks_healthy(repo: str) -> tuple[bool, str]:
    """Check if the latest Checks run on main succeeded."""
    try:
        result = subprocess.run(
            ["gh", "run", "list", "--workflow=checks.yml", "--branch=main",
             "--repo", repo, "--limit", "1", "--json", "conclusion,status",
             "-q", ".[0] // empty"],
            capture_output=True, text=True, timeout=15,
        )
        if not result.stdout.strip():
            return True, "no runs found"
        import json as _json
        run = _json.loads(result.stdout.strip())
        status = run.get("status", "")
        conclusion = run.get("conclusion", "")
        if status != "completed":
            return False, f"latest Checks on main is still {status}"
        if conclusion == "success":
            return True, "passing"
        return False, f"latest Checks on main concluded: {conclusion}"
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return True, "could not check (proceeding anyway)"


def cmd_dispatch_issues(args: argparse.Namespace) -> int:
    repo = args.repo

    if not args.dry_run:
        import time as _time
        max_wait, interval = 600, 30
        for elapsed in range(0, max_wait + 1, interval):
            healthy, health_reason = _main_checks_healthy(repo)
            if healthy:
                break
            if "in_progress" not in health_reason and "queued" not in health_reason:
                # Hard failure (not just pending) — don't wait
                print(f"ERROR: {health_reason}. Fix main before dispatching.", file=sys.stderr)
                return 1
            print(f"Waiting for main Checks ({health_reason})... {elapsed}s/{max_wait}s", file=sys.stderr)
            _time.sleep(interval)
        else:
            print(f"ERROR: main Checks did not pass within {max_wait}s. Last: {health_reason}", file=sys.stderr)
            return 1

    issues = list_tracker_issues(repo)
    dept_filter = args.department
    eligible = [
        issue for issue in issues
        if issue.get("state") == "OPEN" and STATE_BACKLOG in issue_label_names(issue)
        and (not dept_filter or (extract_cop_from_issue(issue) or "").startswith(dept_filter + "/"))
    ]
    eligible = sorted_dispatch_candidates(eligible)
    open_count, in_progress, active = active_agent_fix_count(repo)
    capacity = max(args.max_active - active, 0)
    selected = eligible[:capacity]

    result = {
        "open_agent_fix_prs": open_count,
        "in_progress_agent_fix_runs": in_progress,
        "active_count": active,
        "max_active": args.max_active,
        "capacity": capacity,
        "selected": [],
    }

    for issue in selected:
        cop = extract_cop_from_issue(issue)
        if not cop:
            continue
        fields = parse_marker_fields(issue.get("body", ""), TRACKER_RE)
        difficulty = fields.get("difficulty", "complex")
        backend_family = args.backend_family_override
        strength = args.strength_override
        result["selected"].append(
            {
                "issue": issue["number"],
                "cop": cop,
                "difficulty": difficulty,
                "backend_family": backend_family,
                "strength": strength,
            }
        )
        if args.dry_run:
            continue
        cmd = [
            "gh", "workflow", "run", "agent-cop-fix.yml",
            "--repo", repo,
            "-f", f"cop={cop}",
            "-f", f"backend={backend_family}",
            "-f", f"strength={strength}",
            "-f", "mode=fix",
            "-f", f"issue_number={issue['number']}",
        ]
        proc = subprocess.run(cmd, capture_output=True, text=True)
        if proc.returncode != 0:
            print(f"ERROR dispatching {cop}: {proc.stderr.strip()}", file=sys.stderr)
            proc.check_returncode()
        # Mark issue as dispatched immediately to prevent double-dispatch
        subprocess.run(
            ["gh", "issue", "edit", str(issue["number"]), "--repo", repo,
             "--remove-label", "state:backlog",
             "--add-label", "state:dispatched"],
            capture_output=True, text=True,
        )

    print(json.dumps(result, indent=2))
    return 0


def main():
    parser = argparse.ArgumentParser(description="Dispatch-related corpus tooling")
    subparsers = parser.add_subparsers(dest="command", required=True)

    task_parser = subparsers.add_parser("task", help="Generate a self-contained task prompt")
    task_parser.add_argument("cop", help="Cop name (e.g., Style/NegatedWhile)")
    task_parser.add_argument("--output", "-o", type=Path, help="Output file path (default: stdout)")
    task_parser.add_argument("--input", type=Path, help="Path to corpus-results.json")
    task_parser.add_argument("--binary", type=Path, help="Path to nitrocop binary for pre-diagnostic classification")

    changed_parser = subparsers.add_parser("changed", help="Detect cops changed between two refs")
    changed_parser.add_argument("--base", default="origin/main", help="Base ref")
    changed_parser.add_argument("--head", default="HEAD", help="Head ref")

    tiers_parser = subparsers.add_parser("tiers", help="Classify diverging cops into tiers")
    tiers_parser.add_argument("--input", type=Path, help="Path to corpus-results.json")
    tiers_parser.add_argument("--tier", type=int, choices=[1, 2, 3], help="Show only one tier")
    tiers_parser.add_argument("--names", action="store_true", help="Output just cop names")

    rank_parser = subparsers.add_parser("rank", help="Rank cops by dispatchability")
    rank_parser.add_argument("--binary", type=Path, help="Path to nitrocop binary")
    rank_parser.add_argument("--min-bugs", type=int, default=1)
    rank_parser.add_argument("--max-total", type=int, default=15)
    rank_parser.add_argument("--min-total", type=int, default=3)
    rank_parser.add_argument("--min-matches", type=int, default=50)
    rank_parser.add_argument("--json", action="store_true")

    prior_parser = subparsers.add_parser("prior-attempts", help="Collect prior failed PR attempts")
    prior_parser.add_argument("--cop", required=True, help="Cop name (e.g., Style/NegatedWhile)")
    prior_parser.add_argument("--output", "-o", type=Path, help="Output file path")

    backend_parser = subparsers.add_parser("backend", help="Select a backend for a cop")
    backend_parser.add_argument("--cop", required=True, help="Cop name (e.g., Style/NegatedWhile)")
    backend_parser.add_argument("--mode", choices=["fix", "retry"], default="fix")
    backend_parser.add_argument("--binary", type=Path, help="Path to nitrocop binary")
    backend_parser.add_argument("--input", type=Path, help="Path to corpus-results.json")
    backend_parser.add_argument(
        "--issue-difficulty",
        choices=["simple", "medium", "complex"],
        help="Use linked issue difficulty label as the routing hint",
    )
    backend_parser.add_argument(
        "--repo",
        default=os.environ.get("GITHUB_REPOSITORY", ""),
        help="GitHub repo (owner/name) for prior-attempt lookup",
    )

    issues_sync = subparsers.add_parser("issues-sync", help="Sync one tracker issue per diverging cop")
    issues_sync.add_argument("--input", type=Path, help="Path to corpus-results.json")
    issues_sync.add_argument("--binary", type=Path, help="Path to nitrocop binary for backend routing")
    issues_sync.add_argument(
        "--department",
        help="Only sync cops in this department (e.g., Rails, Style, Performance)",
    )
    issues_sync.add_argument(
        "--repo",
        default=os.environ.get("GITHUB_REPOSITORY", ""),
        help="GitHub repo (owner/name)",
    )

    dispatch_issues = subparsers.add_parser("dispatch-issues", help="Dispatch backlog issues into agent-cop-fix")
    dispatch_issues.add_argument("--max-active", type=int, default=5)
    dispatch_issues.add_argument("--dry-run", action="store_true")
    dispatch_issues.add_argument(
        "--department",
        help="Only dispatch cops in this department (e.g., Rails, Style, Performance)",
    )
    dispatch_issues.add_argument(
        "--backend-family-override",
        choices=["auto", "codex", "claude", "claude-oauth", "minimax"],
        default="auto",
    )
    dispatch_issues.add_argument(
        "--strength-override",
        choices=["auto", "normal", "hard"],
        default="auto",
    )
    dispatch_issues.add_argument(
        "--repo",
        default=os.environ.get("GITHUB_REPOSITORY", ""),
        help="GitHub repo (owner/name)",
    )

    args = parser.parse_args()

    if args.command == "task":
        binary = args.binary.resolve() if args.binary else None
        task = generate_task(args.cop, args.input, binary)
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(task)
            print(f"Task written to {args.output}", file=sys.stderr)
        else:
            print(task)
        return

    if args.command == "changed":
        for cop in detect_cops(args.base, args.head):
            print(cop)
        return

    if args.command == "tiers":
        data = load_dispatch_corpus(args.input)
        cops, tiers = tier_cops(data)
        if args.names:
            target = tiers[args.tier] if args.tier else cops
            for cop in target:
                print(cop["cop"])
            return
        if args.tier:
            tier_entries = tiers[args.tier]
            low, high = TIER_THRESHOLDS[args.tier]
            print(f"Tier {args.tier} ({low}-{high} FP+FN): {len(tier_entries)} cops\n")
            print(f"{'Cop':<50} {'FP':>6} {'FN':>6} {'Total':>6} {'Match%':>7}")
            print(f"{'-'*50} {'-'*6} {'-'*6} {'-'*6} {'-'*7}")
            for cop in tier_entries:
                pct = f"{cop['match_rate']*100:.1f}%" if cop['match_rate'] else "?"
                print(f"{cop['cop']:<50} {cop['fp']:>6} {cop['fn']:>6} {cop['total']:>6} {pct:>7}")
            return

        print(f"Total diverging cops: {len(cops)}\n")
        for tier in [1, 2, 3]:
            tier_entries = tiers[tier]
            low, high = TIER_THRESHOLDS[tier]
            total = sum(cop["total"] for cop in tier_entries)
            print(f"Tier {tier} ({low}-{high} FP+FN): {len(tier_entries)} cops, {total:,} total FP+FN")
        print(f"\nTotal FP+FN: {sum(cop['total'] for cop in cops):,}")
        print("\nUse --tier N to see individual cops in a tier.")
        print("Use --tier N --names for scripting (one cop name per line).")
        return

    if args.command == "rank":
        binary = args.binary
        if not binary:
            for candidate in [
                Path(os.environ.get("CARGO_TARGET_DIR", "target")) / "debug" / "nitrocop",
                Path("target-linux/debug/nitrocop"),
                Path("target/debug/nitrocop"),
            ]:
                if candidate.exists():
                    binary = candidate.resolve()
                    break
        if not binary or not binary.exists():
            print("Error: nitrocop binary not found. Build with 'cargo build' or pass --binary", file=sys.stderr)
            raise SystemExit(1)

        print(f"Using binary: {binary}", file=sys.stderr)
        path, _, _ = _download_corpus()
        data = json.loads(path.read_text())
        results = []
        for entry in sorted(data["by_cop"], key=lambda item: item.get("fp", 0) + item.get("fn", 0)):
            fp = entry.get("fp", 0)
            fn = entry.get("fn", 0)
            total = fp + fn
            if total < args.min_total or total > args.max_total:
                continue
            if entry.get("matches", 0) < args.min_matches:
                continue

            cop_name = entry["cop"]
            fn_bugs, fn_cfg = diagnose_examples(binary, cop_name, entry.get("fn_examples", []), "fn")
            fp_bugs, fp_cfg = diagnose_examples(binary, cop_name, entry.get("fp_examples", []), "fp")
            bugs = fn_bugs + fp_bugs
            cfg = fn_cfg + fp_cfg

            if bugs >= args.min_bugs:
                results.append({
                    "cop": cop_name,
                    "fp": fp,
                    "fn": fn,
                    "code_bugs": bugs,
                    "config_issues": cfg,
                    "matches": entry.get("matches", 0),
                })

        if args.json:
            json.dump(results, sys.stdout, indent=2)
            sys.stdout.write("\n")
        else:
            print(f"\n{'Cop':<42} {'FP':>3} {'FN':>3} {'Bugs':>4} {'Cfg':>4} {'Matches':>7}")
            print("-" * 68)
            for result in results:
                print(
                    f"{result['cop']:<42} {result['fp']:>3} {result['fn']:>3} "
                    f"{result['code_bugs']:>4} {result['config_issues']:>4} {result['matches']:>7}"
                )
            print(f"\n{len(results)} cops with {args.min_bugs}+ code bugs", file=sys.stderr)
        return

    if args.command == "prior-attempts":
        result = collect_attempts(args.cop)
        if args.output:
            args.output.write_text(result)
            if result:
                print(f"Collected prior attempts → {args.output}", file=sys.stderr)
            else:
                print(f"No prior attempts found for {args.cop}", file=sys.stderr)
        else:
            if result:
                print(result)
            else:
                print(f"No prior attempts found for {args.cop}", file=sys.stderr)
        return

    if args.command == "backend":
        if not args.repo:
            print("Error: --repo or GITHUB_REPOSITORY is required", file=sys.stderr)
            raise SystemExit(1)
        raise SystemExit(cmd_backend(args))

    if args.command == "issues-sync":
        if not args.repo:
            print("Error: --repo or GITHUB_REPOSITORY is required", file=sys.stderr)
            raise SystemExit(1)
        raise SystemExit(cmd_issues_sync(args))

    if args.command == "dispatch-issues":
        if not args.repo:
            print("Error: --repo or GITHUB_REPOSITORY is required", file=sys.stderr)
            raise SystemExit(1)
        raise SystemExit(cmd_dispatch_issues(args))


if __name__ == "__main__":
    main()
