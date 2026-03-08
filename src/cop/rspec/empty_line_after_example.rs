use crate::cop::node_type::CALL_NODE;
use crate::cop::util::{
    RSPEC_DEFAULT_INCLUDE, RSPEC_EXAMPLES, is_blank_or_whitespace_line, is_rspec_example, line_at,
    node_on_single_line,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-08)
///
/// Initially reported FP=1,547, FN=2.
///
/// FP=1,547→15: Fixed whitespace-only separator lines treated as non-blank.
/// Also fixed heredoc content extending past the example call location.
///
/// ## Corpus investigation (2026-03-08, pass 2)
///
/// Remaining FP=15 across 6 repos, FN=2 across 2 repos (match rate 99.5%).
///
/// FP root cause 1 (trailing semicolons, 6 FPs in puppetlabs/puppet): One-liner
/// `do;...;end;` examples with a trailing semicolon after `end` were not recognized
/// by `is_single_line_block` because it checked `ends_with(b"end")` but the trailing
/// `;` prevented the match.  Fix: strip trailing semicolons in the function.
///
/// FP root cause 2 (nested last child, 4+ FPs in activegraph, others): Examples
/// nested as the only/last child inside a parent block on the same line (e.g.,
/// `wrapper(...) { it { ... } }`) were not recognized as "last child" because our
/// text-based terminator check only looked at the NEXT line, not the remaining
/// content on the SAME line after the example node.  RuboCop uses AST `last_child?`
/// to detect this.  Fix: after the example node ends, check if the rest of the
/// end_line is only closing syntax (whitespace, `;`, `}`, `end`).
///
/// FN=2: Not addressed in this pass.
pub struct EmptyLineAfterExample;

impl Cop for EmptyLineAfterExample {
    fn name(&self) -> &'static str {
        "RSpec/EmptyLineAfterExample"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        if call.receiver().is_some() || !is_rspec_example(method_name) {
            return;
        }

        // RuboCop's EmptyLineAfterExample uses `on_block` — it only fires on example
        // calls that have a block (do..end or { }).  Bare calls like `skip('reason')`
        // inside a `before` block, or `scenario` used as a variable-like method from
        // `let(:scenario)`, are not example declarations and must be ignored.
        if call.block().is_none() {
            return;
        }

        let allow_consecutive = config.get_bool("AllowConsecutiveOneLiners", true);

        // Determine the end line of this example, accounting for heredocs
        // whose content extends past the node's own location.
        let loc = node.location();
        let mut max_end_offset = loc.end_offset();
        let heredoc_max = find_max_heredoc_end_offset(source, node);
        if heredoc_max > max_end_offset {
            max_end_offset = heredoc_max;
        }
        let end_offset = max_end_offset.saturating_sub(1).max(loc.start_offset());
        let (end_line, _) = source.offset_to_line_col(end_offset);

        let is_one_liner = node_on_single_line(source, &loc);

        // Check if the example is the "last child" by inspecting the rest of
        // the end_line after the node.  RuboCop uses AST `last_child?` for this;
        // we approximate by checking if the remaining content on the same line
        // is only closing syntax (whitespace, `;`, `}`, `end`).  This handles
        // patterns like `wrapper(...) { it { ... } }` where the `it` block ends
        // mid-line and the rest is just the parent's closing brace.
        if is_last_child_on_line(source, max_end_offset, end_line) {
            return;
        }

        // Check if the next non-blank line is another node
        let next_line = end_line + 1;
        let next_content = line_at(source, next_line);
        match next_content {
            Some(line) => {
                if is_blank_or_whitespace_line(line) {
                    return; // already has blank line
                }

                // Determine the effective "check line" — skip past comments to find
                // the first non-comment, non-blank line.  If a blank line or EOF is
                // encountered while scanning comments, the example is properly
                // separated and we return early.
                let check_line = if is_comment_line(line) {
                    let mut scan = next_line + 1;
                    loop {
                        match line_at(source, scan) {
                            Some(l) if is_blank_or_whitespace_line(l) => return,
                            Some(l) if is_comment_line(l) => {}
                            Some(l) => break l,
                            None => return, // end of file
                        }
                        scan += 1;
                    }
                } else {
                    line
                };

                // If consecutive one-liners are allowed, check if the next
                // meaningful line is also a one-liner example.
                // Both the current AND next example must be one-liners.
                if allow_consecutive && is_one_liner {
                    let trimmed = check_line.iter().position(|&b| b != b' ' && b != b'\t');
                    if let Some(start) = trimmed {
                        let rest = &check_line[start..];
                        if starts_with_example_keyword(rest) && is_single_line_block(rest) {
                            return;
                        }
                    }
                }

                // Check for terminator keywords (last example before closing
                // construct).  RuboCop uses `last_child?` on the AST; we
                // approximate by recognising `end`, `else`, `elsif`, `when`,
                // `rescue`, `ensure`, and `in` (pattern matching).
                if is_terminator_line(check_line) {
                    return;
                }
            }
            None => return, // end of file
        }

        // Report on the end line of the example
        let method_str = std::str::from_utf8(method_name).unwrap_or("it");
        let report_col_actual = if is_one_liner {
            let (_, start_col) = source.offset_to_line_col(loc.start_offset());
            start_col
        } else {
            // For multi-line, report at the `end` keyword column
            if let Some(line_bytes) = line_at(source, end_line) {
                line_bytes.iter().take_while(|&&b| b == b' ').count()
            } else {
                0
            }
        };

        diagnostics.push(self.diagnostic(
            source,
            end_line,
            report_col_actual,
            format!("Add an empty line after `{method_str}`."),
        ));
    }
}

/// Walk descendants of `node` to find the maximum `closing_loc().end_offset()`
/// among heredoc StringNode/InterpolatedStringNode children. Heredocs in Prism
/// have their `location()` covering only the opening delimiter (`<<-OUT`), but
/// `closing_loc()` covers the terminator line. Returns 0 if no heredocs found.
fn find_max_heredoc_end_offset(source: &SourceFile, node: &ruby_prism::Node<'_>) -> usize {
    use ruby_prism::Visit;

    struct MaxHeredocVisitor<'a> {
        source: &'a SourceFile,
        max_offset: usize,
    }

    impl<'pr> Visit<'pr> for MaxHeredocVisitor<'_> {
        fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
            if let Some(opening) = node.opening_loc() {
                let bytes = &self.source.as_bytes()[opening.start_offset()..opening.end_offset()];
                if bytes.starts_with(b"<<") {
                    if let Some(closing) = node.closing_loc() {
                        self.max_offset = self.max_offset.max(closing.end_offset());
                    }
                    return;
                }
            }
            ruby_prism::visit_string_node(self, node);
        }

        fn visit_interpolated_string_node(
            &mut self,
            node: &ruby_prism::InterpolatedStringNode<'pr>,
        ) {
            if let Some(opening) = node.opening_loc() {
                let bytes = &self.source.as_bytes()[opening.start_offset()..opening.end_offset()];
                if bytes.starts_with(b"<<") {
                    if let Some(closing) = node.closing_loc() {
                        self.max_offset = self.max_offset.max(closing.end_offset());
                    }
                    return;
                }
            }
            ruby_prism::visit_interpolated_string_node(self, node);
        }
    }

    let mut visitor = MaxHeredocVisitor {
        source,
        max_offset: 0,
    };
    visitor.visit(node);
    visitor.max_offset
}

/// Check if the example is the "last child" by examining what comes after it on
/// the same line.  If the remaining content (after the node's end offset) on the
/// end_line consists only of whitespace, semicolons, closing braces `}`, and/or
/// the `end` keyword, the example is effectively the last child of its parent.
fn is_last_child_on_line(source: &SourceFile, node_end_offset: usize, end_line: usize) -> bool {
    let line_bytes = match line_at(source, end_line) {
        Some(l) => l,
        None => return false,
    };

    // Find the start offset of the end_line to calculate the position within the line
    let line_start = match source.line_col_to_offset(end_line, 0) {
        Some(offset) => offset,
        None => return false,
    };
    if node_end_offset < line_start {
        return false;
    }
    let pos_in_line = node_end_offset - line_start;

    // If the node ends at or past the end of the line, there's nothing after it
    if pos_in_line >= line_bytes.len() {
        return false; // Nothing after — handled by next-line checks
    }

    let rest = &line_bytes[pos_in_line..];

    // Check if the rest is only closing syntax: whitespace, `;`, `}`, `end`
    is_only_closing_syntax(rest)
}

/// Returns true if the byte slice contains only whitespace, semicolons, closing
/// braces `}`, and/or the `end` keyword (with word boundaries).
fn is_only_closing_syntax(bytes: &[u8]) -> bool {
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b';' | b'}' => {
                i += 1;
            }
            b'e' => {
                // Check for `end` keyword with proper boundaries
                if bytes[i..].starts_with(b"end") {
                    let after = i + 3;
                    if after == bytes.len()
                        || matches!(bytes[after], b' ' | b'\t' | b';' | b'}' | b'\n')
                    {
                        i = after;
                        continue;
                    }
                }
                return false;
            }
            _ => return false,
        }
    }
    true
}

/// Returns true if the trimmed line starts with `#`.
fn is_comment_line(line: &[u8]) -> bool {
    let trimmed_pos = line.iter().position(|&b| b != b' ' && b != b'\t');
    matches!(trimmed_pos, Some(start) if line[start] == b'#')
}

/// Check if a line is a block/construct terminator — i.e. the example is
/// the last child before the closing keyword.
fn is_terminator_line(line: &[u8]) -> bool {
    let trimmed = line.iter().position(|&b| b != b' ' && b != b'\t');
    if let Some(start) = trimmed {
        let rest = &line[start..];
        if rest.starts_with(b"}") {
            return true;
        }
        for keyword in &[
            b"end" as &[u8],
            b"else",
            b"elsif",
            b"when",
            b"rescue",
            b"ensure",
            b"in ",
        ] {
            if rest.starts_with(keyword) {
                // Ensure keyword isn't part of a longer identifier
                if rest.len() == keyword.len()
                    || !rest[keyword.len()].is_ascii_alphanumeric() && rest[keyword.len()] != b'_'
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a line represents a single-line block (contains closing `end` or `}` on same line).
fn is_single_line_block(line: &[u8]) -> bool {
    // Single-line brace block: `it { something }`
    if line.contains(&b'{') && line.contains(&b'}') {
        return true;
    }

    // Single-line do..end: `it "foo" do something end` or `it "foo" do; stuff; end;`
    // Require `end` as the trailing keyword to avoid matching description text.
    // Strip trailing semicolons and whitespace (some codebases use `; end;` style).
    let mut trimmed = trim_ascii_whitespace(line);
    while trimmed.ends_with(b";") {
        trimmed = trim_ascii_whitespace(&trimmed[..trimmed.len() - 1]);
    }
    if trimmed.ends_with(b"end") && contains_keyword(trimmed, b"do") {
        return true;
    }
    false
}

fn trim_ascii_whitespace(mut line: &[u8]) -> &[u8] {
    while let Some((first, rest)) = line.split_first() {
        if *first == b' ' || *first == b'\t' {
            line = rest;
        } else {
            break;
        }
    }
    while let Some((last, rest)) = line.split_last() {
        if *last == b' ' || *last == b'\t' {
            line = rest;
        } else {
            break;
        }
    }
    line
}

fn contains_keyword(line: &[u8], keyword: &[u8]) -> bool {
    if keyword.is_empty() || line.len() < keyword.len() {
        return false;
    }
    line.windows(keyword.len()).enumerate().any(|(i, window)| {
        if window != keyword {
            return false;
        }
        let left_ok = i == 0 || !line[i - 1].is_ascii_alphanumeric() && line[i - 1] != b'_';
        let right_idx = i + keyword.len();
        let right_ok = right_idx == line.len()
            || !line[right_idx].is_ascii_alphanumeric() && line[right_idx] != b'_';
        left_ok && right_ok
    })
}

/// Check if a line starts with any RSpec example keyword followed by a
/// delimiter (space, `(`, `{`, or ` {`).  Uses the canonical
/// `RSPEC_EXAMPLES` list so that all example variants (`its`, `xit`, `fit`,
/// `pending`, etc.) are recognised for the consecutive-one-liner check.
fn starts_with_example_keyword(line: &[u8]) -> bool {
    for keyword in RSPEC_EXAMPLES {
        let kw = keyword.as_bytes();
        if line.starts_with(kw) {
            // keyword must be followed by a delimiter or be the entire line
            if line.len() == kw.len() {
                return true;
            }
            let next = line[kw.len()];
            if next == b' ' || next == b'(' || next == b'{' {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyLineAfterExample, "cops/rspec/empty_line_after_example");
}
