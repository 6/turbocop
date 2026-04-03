use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util::{
    RSPEC_DEFAULT_INCLUDE, is_blank_or_whitespace_line, is_rspec_example_group,
    is_rspec_shared_group, line_at,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-08)
///
/// ### Round 1
/// Corpus oracle reported FP=484, FN=7.
/// FP root cause: separator lines containing only spaces/tabs were treated as
/// non-blank by `is_blank_line`, so example groups followed by whitespace-only
/// lines were flagged. RuboCop's separator logic treats whitespace-only lines
/// as blank.
/// Fix: use whitespace-aware blank-line checks while scanning lines after group end.
///
/// ### Round 2 (FP=5, FN=7)
/// FN root cause: calls with receivers were skipped entirely, but `RSpec.describe`,
/// `RSpec.shared_examples`, and `RSpec.shared_context` are valid example groups
/// that should be checked. RuboCop's `spec_group?` matcher accepts both bare
/// `describe` and `RSpec.describe`.
/// Fix: allow calls where the receiver is the `RSpec` constant.
///
/// FP root cause: RuboCop's `last_child?` returns true (skip) when the block's
/// parent is not a `begin` node (e.g., when wrapped in a postfix `if`/`unless`).
/// nitrocop's line-scanning approach didn't account for postfix modifiers on the
/// `end` line (e.g., `end if condition`). After the `end` keyword, remaining
/// content like `if ...`/`unless ...` indicates a postfix conditional, and
/// RuboCop skips such cases.
/// Fix: after finding end_line, check if the end line has a postfix `if`/`unless`
/// after the `end` keyword; if so, skip.
///
/// ### Round 3 (FP=1, FN=0)
/// FP root cause: `__END__` marker (which terminates Ruby source) was treated as
/// a regular code line, causing nitrocop to flag the preceding example group's `end`
/// for lacking a blank line separator. RuboCop's AST-based approach naturally ignores
/// `__END__` since nothing after it is part of the parse tree.
/// Fix: treat `__END__` on a line by itself as end-of-file when scanning lines after
/// the example group end.
pub struct EmptyLineAfterExampleGroup;

impl Cop for EmptyLineAfterExampleGroup {
    fn name(&self) -> &'static str {
        "RSpec/EmptyLineAfterExampleGroup"
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
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        // Allow bare calls (no receiver) and RSpec-prefixed calls (RSpec.describe, etc.)
        if let Some(recv) = call.receiver() {
            // Only allow `RSpec` constant as receiver
            let is_rspec = recv
                .as_constant_read_node()
                .is_some_and(|c| c.name().as_slice() == b"RSpec")
                || recv.as_constant_path_node().is_some_and(|cp| {
                    cp.parent().is_none() && cp.name().is_some_and(|n| n.as_slice() == b"RSpec")
                });
            if !is_rspec {
                return;
            }
        }
        if !is_rspec_example_group(method_name) && !is_rspec_shared_group(method_name) {
            return;
        }

        // Must have a block (multi-line group)
        if call.block().is_none() {
            return;
        }

        let loc = node.location();

        // Skip if the node is part of a larger expression (e.g., `group = RSpec.describe { }`)
        // by checking if there's non-whitespace content before the node on its start line.
        // RuboCop's `last_child?` returns true when the block's parent isn't a `begin` node,
        // which covers these cases.
        let (start_line, start_col) = source.offset_to_line_col(loc.start_offset());
        if let Some(start_line_bytes) = line_at(source, start_line) {
            let prefix = &start_line_bytes[..start_col.min(start_line_bytes.len())];
            if prefix.iter().any(|&b| b != b' ' && b != b'\t') {
                return;
            }
        }

        let end_offset = loc.end_offset().saturating_sub(1).max(loc.start_offset());
        let (end_line, end_col) = source.offset_to_line_col(end_offset);

        // If the end line has a postfix `if`/`unless` after the node's end,
        // the block is wrapped in a conditional. RuboCop skips these because
        // the block's parent is not a `begin` node (it's an IfNode/UnlessNode).
        if let Some(end_line_bytes) = line_at(source, end_line) {
            if has_postfix_conditional_after(end_line_bytes, end_col) {
                return;
            }
        }

        // Check the lines after the example group end.
        // Skip if:
        //   - next line is blank (already has empty line)
        //   - next non-blank/non-comment line is `end` (last item in parent group)
        //   - no more lines (end of file)
        let total_lines = source.lines().count();
        let mut check_line = end_line + 1;
        loop {
            if check_line > total_lines {
                return; // End of file
            }
            match line_at(source, check_line) {
                Some(line) => {
                    if is_blank_or_whitespace_line(line) {
                        return; // Found blank line — OK
                    }
                    let trimmed_start = line.iter().position(|&b| b != b' ' && b != b'\t');
                    if let Some(start) = trimmed_start {
                        let rest = &line[start..];
                        if rest.starts_with(b"#") {
                            // Comment line — keep scanning
                            check_line += 1;
                            continue;
                        }
                        if rest.starts_with(b"end")
                            && (rest.len() == 3 || !rest[3].is_ascii_alphanumeric())
                        {
                            return; // Next meaningful line is `end` — OK
                        }
                        // `}` is also a closing delimiter (e.g., `.each { |x| ... }`)
                        if rest[0] == b'}' {
                            return; // Next meaningful line is `}` — OK (last child)
                        }
                        // `__END__` terminates Ruby source; treat as end-of-file
                        if rest == b"__END__" || rest.starts_with(b"__END__\r") {
                            return;
                        }
                        // Control flow keywords that are part of the enclosing
                        // construct (if/unless/case/begin) — not a new statement
                        if starts_with_keyword(rest, b"else")
                            || starts_with_keyword(rest, b"elsif")
                            || starts_with_keyword(rest, b"rescue")
                            || starts_with_keyword(rest, b"ensure")
                            || starts_with_keyword(rest, b"when")
                            || starts_with_keyword(rest, b"in")
                        {
                            return;
                        }
                    }
                    break; // Found a non-blank, non-comment, non-end line — offense
                }
                None => return,
            }
        }

        let method_str = std::str::from_utf8(method_name).unwrap_or("describe");
        // Report at the `end` keyword line
        let report_col = if let Some(line_bytes) = line_at(source, end_line) {
            line_bytes.iter().take_while(|&&b| b == b' ').count()
        } else {
            0
        };

        diagnostics.push(self.diagnostic(
            source,
            end_line,
            report_col,
            format!("Add an empty line after `{method_str}`."),
        ));
    }
}

/// Check if a line has a postfix `if`/`unless` after a given column.
/// Handles both `end if condition` and `describe('X') { } unless condition`.
fn has_postfix_conditional_after(line: &[u8], end_col: usize) -> bool {
    // Look at content after the node's end position
    if end_col + 1 >= line.len() {
        return false;
    }
    let after = &line[end_col + 1..];
    // Skip whitespace
    let ws_end = after
        .iter()
        .position(|&b| b != b' ' && b != b'\t')
        .unwrap_or(after.len());
    if ws_end == after.len() {
        return false;
    }
    let keyword_part = &after[ws_end..];
    starts_with_keyword(keyword_part, b"if") || starts_with_keyword(keyword_part, b"unless")
}

/// Check if a byte slice starts with a Ruby keyword followed by a non-identifier char
/// (or end of line).
fn starts_with_keyword(rest: &[u8], keyword: &[u8]) -> bool {
    rest.starts_with(keyword)
        && (rest.len() == keyword.len()
            || (!rest[keyword.len()].is_ascii_alphanumeric() && rest[keyword.len()] != b'_'))
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        EmptyLineAfterExampleGroup,
        "cops/rspec/empty_line_after_example_group"
    );
}
