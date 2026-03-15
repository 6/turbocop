use std::collections::HashSet;

use ruby_prism::Visit;

use crate::cop::node_type::PROGRAM_NODE;
use crate::cop::util::{
    RSPEC_DEFAULT_INCLUDE, is_blank_or_whitespace_line, is_rspec_example, line_at,
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
/// FN=2: Not addressed in pass 2.
///
/// ## Corpus investigation (2026-03-11)
///
/// FP=2, FN=41. Rewrote to AST-based approach (Visit over StatementsNode) to
/// match RuboCop's `last_child?` and `missing_separating_line` logic precisely.
///
/// Root causes of FN=41:
/// 1. Text-based `is_terminator_line` was too aggressive — falsely treating
///    lines as terminators when they weren't actually the parent's closing keyword.
/// 2. `rubocop:enable` directive reporting: RuboCop reports the offense on the
///    enable directive line, but nitrocop was reporting on the example's `end` line,
///    causing FP/FN location mismatches.
/// 3. Text-based consecutive one-liner check was less precise than AST sibling check.
///
/// Fix: Switched to PROGRAM_NODE + Visit pattern (same as EmptyLineAfterSubject).
/// Uses AST siblings for `last_child?`, `build_comment_line_sets` for rubocop:enable
/// directive handling, and AST right-sibling for consecutive one-liner detection.
///
/// ## Corpus investigation (2026-03-14)
///
/// FP=0, FN=292 across 37 repos (parslet=38, ransack=30, daru=28, thredded=27, etc.).
///
/// Root cause: the cop was incorrectly scoped to only visit top-level spec group
/// calls, borrowing the `InsideExampleGroup` restriction from EmptyLineAfterSubject.
/// However, EmptyLineAfterExample does NOT use InsideExampleGroup — RuboCop's
/// `on_block` handler fires for ALL blocks in the file, including examples inside
/// `module` and `class` wrappers. Any `it`/`specify`/etc. block without a receiver
/// anywhere in a spec file is checked.
///
/// Fix: removed the top-level spec group filter; the visitor now starts from the
/// program root and walks all nodes. The FN count was 292; removing the scoping
/// restriction captures examples inside module/class wrapping at file scope.
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
        &[PROGRAM_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let program = match node.as_program_node() {
            Some(p) => p,
            None => return,
        };

        let allow_consecutive = config.get_bool("AllowConsecutiveOneLiners", true);
        let (comment_lines, enable_directive_lines) = build_comment_line_sets(source, parse_result);

        // RuboCop's EmptyLineAfterExample uses `on_block` which fires for ALL blocks
        // in the file, not just those inside top-level spec groups. Unlike
        // EmptyLineAfterSubject (which uses InsideExampleGroup), this cop checks
        // any example block found anywhere — including inside module/class wrappers.
        let mut visitor = ExampleSeparationVisitor {
            source,
            cop: self,
            diagnostics,
            comment_lines: &comment_lines,
            enable_directive_lines: &enable_directive_lines,
            allow_consecutive,
        };
        visitor.visit_program_node(&program);
    }
}

struct ExampleSeparationVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a EmptyLineAfterExample,
    diagnostics: &'a mut Vec<Diagnostic>,
    comment_lines: &'a HashSet<usize>,
    enable_directive_lines: &'a HashSet<usize>,
    allow_consecutive: bool,
}

impl<'a> ExampleSeparationVisitor<'a> {
    fn check_example_in_list<'pr>(
        &mut self,
        siblings: &[ruby_prism::Node<'pr>],
        idx: usize,
        example_stmt: &ruby_prism::Node<'pr>,
        example_call: &ruby_prism::CallNode<'pr>,
    ) {
        // RuboCop's last_child? — if this is the last sibling, skip
        if idx + 1 >= siblings.len() {
            return;
        }

        // Check consecutive one-liner exemption
        if self.allow_consecutive
            && is_consecutive_one_liner(self.source, example_stmt, siblings, idx)
        {
            return;
        }

        let report_line = match missing_separating_line(
            self.source,
            example_stmt,
            self.comment_lines,
            self.enable_directive_lines,
        ) {
            Some(line) => line,
            None => return,
        };

        let report_col = line_at(self.source, report_line)
            .map(|line| {
                line.iter()
                    .take_while(|&&b| b == b' ' || b == b'\t')
                    .count()
            })
            .unwrap_or(0);

        let method_name = std::str::from_utf8(example_call.name().as_slice()).unwrap_or("it");
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            report_line,
            report_col,
            format!("Add an empty line after `{method_name}`."),
        ));
    }
}

impl<'a, 'pr> Visit<'pr> for ExampleSeparationVisitor<'a> {
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        let siblings: Vec<_> = node.body().iter().collect();

        for (idx, stmt) in siblings.iter().enumerate() {
            let call = match stmt.as_call_node() {
                Some(c) => c,
                None => continue,
            };

            if call.receiver().is_some() || !is_rspec_example(call.name().as_slice()) {
                continue;
            }

            // RuboCop's on_block — only fires on example calls with a block
            if call.block().is_none() {
                continue;
            }

            self.check_example_in_list(&siblings, idx, stmt, &call);
        }

        ruby_prism::visit_statements_node(self, node);
    }
}

/// Check if the current example is a consecutive one-liner that should be exempt.
/// RuboCop: `consecutive_one_liner?` = node.single_line? && next_one_line_example?(node)
/// where `next_one_line_example?` checks `node.right_sibling` is an example AND single_line.
fn is_consecutive_one_liner(
    source: &SourceFile,
    example_stmt: &ruby_prism::Node<'_>,
    siblings: &[ruby_prism::Node<'_>],
    idx: usize,
) -> bool {
    // Current example must be single-line
    if !node_on_single_line(source, &example_stmt.location()) {
        return false;
    }

    // Right sibling must exist
    let next_idx = idx + 1;
    if next_idx >= siblings.len() {
        return false;
    }

    let next_sibling = &siblings[next_idx];
    // Right sibling must be an example call with a block
    let next_call = match next_sibling.as_call_node() {
        Some(c) => c,
        None => return false,
    };
    if next_call.receiver().is_some() || !is_rspec_example(next_call.name().as_slice()) {
        return false;
    }
    if next_call.block().is_none() {
        return false;
    }

    // Right sibling must be single-line
    node_on_single_line(source, &next_sibling.location())
}

/// Determine if an empty line is missing after a node, following RuboCop's
/// EmptyLineSeparation mixin logic. Returns the line number to report on,
/// or None if no offense.
fn missing_separating_line(
    source: &SourceFile,
    stmt: &ruby_prism::Node<'_>,
    comment_lines: &HashSet<usize>,
    enable_directive_lines: &HashSet<usize>,
) -> Option<usize> {
    // Match RuboCop's FinalEndLocation: heredocs may extend past the node's own location
    let loc = stmt.location();
    let mut max_end_offset = loc.end_offset();
    let heredoc_max = find_max_heredoc_end_offset(source, stmt);
    if heredoc_max > max_end_offset {
        max_end_offset = heredoc_max;
    }
    let end_offset = max_end_offset.saturating_sub(1).max(loc.start_offset());
    let (end_line, _) = source.offset_to_line_col(end_offset);

    // Check if the example is the "last child" on the same line — i.e., the rest
    // of the end_line after the node is only closing syntax (whitespace, `;`, `}`, `end`).
    // This handles inline patterns like `wrapper(...) { it { ... } }`.
    if is_last_child_on_line(source, max_end_offset, end_line) {
        return None;
    }

    // RuboCop's EmptyLineSeparation:
    // - walk past directly-following comment lines
    // - track rubocop:enable directives
    // - if the next non-comment line is blank or EOF, no offense
    // - otherwise report at enable directive line (if any) or end line
    let mut line = end_line;
    let mut enable_directive_line = None;
    while comment_lines.contains(&(line + 1)) {
        line += 1;
        if enable_directive_lines.contains(&line) {
            enable_directive_line = Some(line);
        }
    }

    match line_at(source, line + 1) {
        Some(next_line) if is_blank_or_whitespace_line(next_line) => None,
        Some(_) => Some(enable_directive_line.unwrap_or(end_line)),
        None => None,
    }
}

fn build_comment_line_sets(
    source: &SourceFile,
    parse_result: &ruby_prism::ParseResult<'_>,
) -> (HashSet<usize>, HashSet<usize>) {
    let mut comment_lines = HashSet::new();
    let mut enable_directive_lines = HashSet::new();

    for comment in parse_result.comments() {
        let loc = comment.location();
        let (start_line, _) = source.offset_to_line_col(loc.start_offset());
        let end_offset = loc.end_offset().saturating_sub(1).max(loc.start_offset());
        let (end_line, _) = source.offset_to_line_col(end_offset);

        for line in start_line..=end_line {
            comment_lines.insert(line);
        }

        let comment_bytes = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
        if comment_bytes
            .windows(b"rubocop:enable".len())
            .any(|window| window == b"rubocop:enable")
        {
            enable_directive_lines.insert(start_line);
        }
    }

    (comment_lines, enable_directive_lines)
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

    let line_start = match source.line_col_to_offset(end_line, 0) {
        Some(offset) => offset,
        None => return false,
    };
    if node_end_offset < line_start {
        return false;
    }
    let pos_in_line = node_end_offset - line_start;

    if pos_in_line >= line_bytes.len() {
        return false;
    }

    let rest = &line_bytes[pos_in_line..];
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

/// Walk descendants of `node` to find the maximum `closing_loc().end_offset()`
/// among heredoc StringNode/InterpolatedStringNode children.
fn find_max_heredoc_end_offset(source: &SourceFile, node: &ruby_prism::Node<'_>) -> usize {
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

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyLineAfterExample, "cops/rspec/empty_line_after_example");
}
