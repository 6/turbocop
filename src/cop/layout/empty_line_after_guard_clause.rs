use crate::cop::node_type::{BREAK_NODE, CALL_NODE, IF_NODE, NEXT_NODE, RETURN_NODE, UNLESS_NODE};
use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Enforces empty line after guard clause.
///
/// ## Corpus conformance investigation (2026-03-11, updated 2026-03-15)
///
/// **Root causes of FN (nitrocop misses offenses RuboCop catches):**
/// - `and`/`or` guard clauses: Fixed by recursing into the `right` child
///   of and/or nodes, matching RuboCop's `operator_keyword?` → `rhs` handling.
/// - Heredoc guard clauses: `raise "msg", <<-MSG unless cond` has the heredoc
///   body after the if node's location. Fixed by walking the guard's AST to find
///   heredoc arguments and using the heredoc end marker line as the effective end.
/// - Ternary guard clauses: `a ? raise(e) : b` is an IfNode with no if_keyword.
///   Fixed by detecting ternary if nodes where either branch is a guard statement.
///
/// **Root causes of FP (nitrocop flags things RuboCop doesn't):**
/// - Comment-then-blank pattern: `guard; # comment; blank; code` — fixed by
///   matching RuboCop's behavior: check the immediate next line for blank/directive
///   instead of skipping all comments to find the first code line.
/// - Heredoc interference: Fixed via heredoc end line detection.
///
/// **Remaining gaps:** Some edge cases with heredocs inside conditions
/// (e.g., `return true if <<~TEXT.length > bar`) may still differ.
pub struct EmptyLineAfterGuardClause;

/// Guard clause keywords that appear at the start of an expression.
const GUARD_METHODS: &[&[u8]] = &[b"return", b"raise", b"fail", b"next", b"break"];

impl Cop for EmptyLineAfterGuardClause {
    fn name(&self) -> &'static str {
        "Layout/EmptyLineAfterGuardClause"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BREAK_NODE,
            CALL_NODE,
            IF_NODE,
            NEXT_NODE,
            RETURN_NODE,
            UNLESS_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Extract body statements, the overall location, and whether it's block form.
        // We handle both modifier and block-form if/unless, plus ternaries.
        let (body_stmts, loc, end_keyword_loc, is_ternary) =
            if let Some(if_node) = node.as_if_node() {
                // Skip elsif nodes
                if let Some(kw) = if_node.if_keyword_loc() {
                    if kw.as_slice() == b"elsif" {
                        return;
                    }
                }
                // Ternary: no if_keyword_loc, has else branch
                if if_node.if_keyword_loc().is_none() {
                    // Ternary guard: check if either branch contains a guard
                    if if_node.subsequent().is_some() {
                        // Has else branch — check if the if-branch is a guard
                        if let Some(stmts) = if_node.statements() {
                            let body: Vec<_> = stmts.body().iter().collect();
                            if body.len() == 1 && is_guard_stmt(&body[0]) {
                                // Ternary with guard in if-branch
                                return self.check_ternary_guard(
                                    source,
                                    &if_node.location(),
                                    diagnostics,
                                    &mut corrections,
                                );
                            }
                        }
                    }
                    return;
                }
                // Skip if/else or if/elsif forms — only simple if/unless (no else branch)
                if if_node.subsequent().is_some() {
                    return;
                }
                match if_node.statements() {
                    Some(s) => (s, if_node.location(), if_node.end_keyword_loc(), false),
                    None => return,
                }
            } else if let Some(unless_node) = node.as_unless_node() {
                // Skip unless/else forms
                if unless_node.else_clause().is_some() {
                    return;
                }
                match unless_node.statements() {
                    Some(s) => (
                        s,
                        unless_node.location(),
                        unless_node.end_keyword_loc(),
                        false,
                    ),
                    None => return,
                }
            } else {
                return;
            };

        let is_modifier = end_keyword_loc.is_none() && !is_ternary;

        let stmts: Vec<_> = body_stmts.body().iter().collect();
        if stmts.is_empty() {
            return;
        }

        let first_stmt = &stmts[0];
        if !is_guard_stmt(first_stmt) {
            return;
        }

        // For block form, the body must be a single guard statement
        if !is_modifier && stmts.len() != 1 {
            return;
        }

        // RuboCop's guard_clause? requires the guard statement to be single-line.
        // A multi-line `next foo && bar && ...` inside a block-form if is not
        // considered a guard clause.
        if !is_modifier {
            let stmt_start_line = source
                .offset_to_line_col(first_stmt.location().start_offset())
                .0;
            let stmt_end_line = source
                .offset_to_line_col(first_stmt.location().end_offset().saturating_sub(1))
                .0;
            if stmt_start_line != stmt_end_line {
                return;
            }
        }

        let lines: Vec<&[u8]> = source.lines().collect();

        // Determine the end offset to use for computing the "last line" of the guard.
        // For modifier form: end of the whole if node.
        // For block form: end of the `end` keyword.
        let effective_end_offset = if let Some(ref end_kw) = end_keyword_loc {
            end_kw.end_offset().saturating_sub(1)
        } else {
            loc.end_offset().saturating_sub(1)
        };

        let (if_end_line, end_col) = source.offset_to_line_col(effective_end_offset);

        // Check for heredoc arguments — if present, the "end line" is after the
        // heredoc closing delimiter, not after the if node's source range.
        let heredoc_end_line = if is_modifier {
            find_heredoc_end_line(source, node)
        } else {
            None
        };
        let effective_end_line = heredoc_end_line.unwrap_or(if_end_line);

        // For the offense location:
        // - Heredoc: start of heredoc end marker content (first non-whitespace on that line)
        // - Block form: start of `end` keyword
        // - Modifier form: start of the if expression
        let offense_offset = if let Some(h_line) = heredoc_end_line {
            // Find the first non-whitespace char on the heredoc end marker line
            let heredoc_line_content = lines[h_line.saturating_sub(1)];
            let indent = heredoc_line_content
                .iter()
                .position(|&b| b != b' ' && b != b'\t')
                .unwrap_or(0);
            source
                .line_col_to_offset(h_line, indent)
                .unwrap_or(loc.start_offset())
        } else if let Some(ref end_kw) = end_keyword_loc {
            end_kw.start_offset()
        } else {
            loc.start_offset()
        };

        // Check if the guard clause is embedded inside a larger expression on the
        // same line (e.g. `arr.each { |x| return x if cond }`). If there is
        // non-comment code after the if node on the same line, skip.
        // Only check this for non-heredoc guards (heredoc guards span multiple lines).
        if heredoc_end_line.is_none() {
            if let Some(cur_line) = lines.get(if_end_line.saturating_sub(1)) {
                let after_pos = end_col + 1;
                if after_pos < cur_line.len() {
                    let rest = &cur_line[after_pos..];
                    if let Some(idx) = rest.iter().position(|&b| b != b' ' && b != b'\t') {
                        if rest[idx] != b'#' {
                            return;
                        }
                    }
                }
            }
        }

        // Check if next line exists
        if effective_end_line >= lines.len() {
            return;
        }

        // Match RuboCop's logic: check the IMMEDIATE next line after the guard.
        // RuboCop does not skip comments — it checks:
        // 1. Is the next line blank? → no offense
        // 2. Is the next line an allowed directive comment, and the line after that
        //    is blank? → no offense
        // 3. Is the next sibling a guard clause or scope-closing keyword? → no offense
        // 4. Otherwise → offense

        let next_line = lines[effective_end_line]; // 0-indexed: effective_end_line is 1-indexed line number

        // Step 1: immediate next line is blank → no offense
        if util::is_blank_line(next_line) {
            return;
        }

        // Step 2: directive/nocov comment followed by blank → no offense
        if is_allowed_directive_comment(next_line)
            && (effective_end_line + 1 >= lines.len()
                || util::is_blank_line(lines[effective_end_line + 1]))
        {
            return;
        }

        // Step 3: Check the next non-comment code line for scope-close or guard.
        // This skips comments (which RuboCop ignores at AST level) to find the
        // actual next sibling statement.
        //
        // If `find_next_code_line` returns None, it either hit a blank line after
        // comments, or reached EOF after comments. In both cases:
        // - If the guard is followed only by comments → end of scope → no offense
        //   (but only if the comments lead to a scope-close like `end`)
        // - If comments → blank → code → it IS an offense (no blank immediately after guard)
        if let Some(code_content) = find_next_code_line(&lines, effective_end_line) {
            if is_scope_close_or_clause_keyword(code_content) {
                return;
            }
            if is_guard_line(code_content) {
                return;
            }
            if is_multiline_guard_block(code_content, &lines, effective_end_line) {
                return;
            }
        } else {
            // find_next_code_line returned None — either hit a blank line (after
            // skipping comments) or reached EOF. Since the immediate next line was
            // NOT blank (checked in step 1), we have comments before the blank/EOF.
            // Check if a scope-closing keyword follows the blank line.
            if let Some(code_after_blank) =
                find_first_code_line_anywhere(&lines, effective_end_line)
            {
                if is_scope_close_or_clause_keyword(code_after_blank) {
                    return;
                }
            } else {
                // Only comments/blanks until EOF — guard is effectively last stmt
                return;
            }
            // If there's code after the blank that's not a scope-close, fall through
            // to flag the offense (guard → comment → blank → code without blank after guard).
        }

        let (line, col) = source.offset_to_line_col(offense_offset);
        let mut diag = self.diagnostic(
            source,
            line,
            col,
            "Add empty line after guard clause.".to_string(),
        );
        if let Some(ref mut corr) = corrections {
            // Insert blank line after the guard clause's last line.
            // If a directive comment follows, insert after the directive line.
            let insert_after_line = if is_allowed_directive_comment(next_line) {
                effective_end_line + 1
            } else {
                effective_end_line
            };
            if let Some(offset) = source.line_col_to_offset(insert_after_line + 1, 0) {
                corr.push(crate::correction::Correction {
                    start: offset,
                    end: offset,
                    replacement: "\n".to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
        }
        diagnostics.push(diag);
    }
}

impl EmptyLineAfterGuardClause {
    /// Handle ternary guard clauses like `a ? raise(e) : other_thing`.
    /// RuboCop treats the entire ternary as a guard clause if one branch
    /// contains a guard statement (raise, return, etc.).
    fn check_ternary_guard(
        &self,
        source: &SourceFile,
        loc: &ruby_prism::Location<'_>,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let lines: Vec<&[u8]> = source.lines().collect();
        let (end_line, end_col) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));

        // Check for embedded expression on same line
        if let Some(cur_line) = lines.get(end_line.saturating_sub(1)) {
            let after_pos = end_col + 1;
            if after_pos < cur_line.len() {
                let rest = &cur_line[after_pos..];
                if let Some(idx) = rest.iter().position(|&b| b != b' ' && b != b'\t') {
                    if rest[idx] != b'#' {
                        return;
                    }
                }
            }
        }

        if end_line >= lines.len() {
            return;
        }

        let next_line = lines[end_line];
        if util::is_blank_line(next_line) {
            return;
        }

        if is_allowed_directive_comment(next_line)
            && (end_line + 1 >= lines.len() || util::is_blank_line(lines[end_line + 1]))
        {
            return;
        }

        if let Some(code_content) = find_next_code_line(&lines, end_line) {
            if is_scope_close_or_clause_keyword(code_content) {
                return;
            }
        } else {
            return;
        }

        let (line, col) = source.offset_to_line_col(loc.start_offset());
        let mut diag = self.diagnostic(
            source,
            line,
            col,
            "Add empty line after guard clause.".to_string(),
        );
        if let Some(corr) = corrections {
            if let Some(offset) = source.line_col_to_offset(end_line + 1, 0) {
                corr.push(crate::correction::Correction {
                    start: offset,
                    end: offset,
                    replacement: "\n".to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
        }
        diagnostics.push(diag);
    }
}

/// Find the line number of the heredoc end marker if the guard clause
/// contains a heredoc argument. Returns None if no heredoc is found.
/// The returned line number is 1-indexed.
fn find_heredoc_end_line(source: &SourceFile, node: &ruby_prism::Node<'_>) -> Option<usize> {
    use ruby_prism::Visit;

    struct HeredocEndFinder<'a> {
        source: &'a SourceFile,
        max_end_line: Option<usize>,
    }

    impl<'pr> Visit<'pr> for HeredocEndFinder<'_> {
        fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
            if let Some(opening) = node.opening_loc() {
                let bytes = &self.source.as_bytes()[opening.start_offset()..opening.end_offset()];
                if bytes.starts_with(b"<<") {
                    if let Some(closing) = node.closing_loc() {
                        let end_off = closing
                            .end_offset()
                            .saturating_sub(1)
                            .max(closing.start_offset());
                        let (end_line, _) = self.source.offset_to_line_col(end_off);
                        self.max_end_line = Some(
                            self.max_end_line
                                .map_or(end_line, |prev| prev.max(end_line)),
                        );
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
                        let end_off = closing
                            .end_offset()
                            .saturating_sub(1)
                            .max(closing.start_offset());
                        let (end_line, _) = self.source.offset_to_line_col(end_off);
                        self.max_end_line = Some(
                            self.max_end_line
                                .map_or(end_line, |prev| prev.max(end_line)),
                        );
                    }
                    return;
                }
            }
            ruby_prism::visit_interpolated_string_node(self, node);
        }
    }

    let mut finder = HeredocEndFinder {
        source,
        max_end_line: None,
    };
    finder.visit(node);
    finder.max_end_line
}

fn is_guard_stmt(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if GUARD_METHODS.contains(&name) && call.receiver().is_none() {
            return true;
        }
    }
    // Bare return/break/next
    if node.as_return_node().is_some()
        || node.as_break_node().is_some()
        || node.as_next_node().is_some()
    {
        return true;
    }
    // `and`/`or` guard clauses: `render :foo and return`, `do_thing || return`
    // RuboCop's guard_clause? checks operator_keyword? and then the rhs.
    if let Some(and_node) = node.as_and_node() {
        return is_guard_stmt(&and_node.right());
    }
    if let Some(or_node) = node.as_or_node() {
        return is_guard_stmt(&or_node.right());
    }
    false
}

/// Find the first non-blank, non-comment line starting from `start_idx` (0-indexed),
/// looking across blank lines (unlike `find_next_code_line` which stops at blanks).
fn find_first_code_line_anywhere<'a>(lines: &[&'a [u8]], start_idx: usize) -> Option<&'a [u8]> {
    for line in &lines[start_idx..] {
        if util::is_blank_line(line) {
            continue;
        }
        if let Some(start) = line.iter().position(|&b| b != b' ' && b != b'\t') {
            let content = &line[start..];
            if content.starts_with(b"#") {
                continue;
            }
            return Some(content);
        }
    }
    None
}

/// Find the next non-blank, non-comment line starting from `start_idx` (0-indexed).
/// Returns None if a blank line is found first or we reach EOF.
fn find_next_code_line<'a>(lines: &[&'a [u8]], start_idx: usize) -> Option<&'a [u8]> {
    for line in &lines[start_idx..] {
        if util::is_blank_line(line) {
            return None;
        }
        if let Some(start) = line.iter().position(|&b| b != b' ' && b != b'\t') {
            let content = &line[start..];
            if content.starts_with(b"#") {
                continue;
            }
            return Some(content);
        }
    }
    None
}

/// Check if trimmed content starts with a scope-closing or clause keyword.
fn is_scope_close_or_clause_keyword(content: &[u8]) -> bool {
    starts_with_keyword(content, b"end")
        || starts_with_keyword(content, b"else")
        || starts_with_keyword(content, b"elsif")
        || starts_with_keyword(content, b"rescue")
        || starts_with_keyword(content, b"ensure")
        || starts_with_keyword(content, b"when")
        || starts_with_keyword(content, b"in")
        || content.starts_with(b"}")
        || content.starts_with(b")")
}

fn starts_with_keyword(content: &[u8], keyword: &[u8]) -> bool {
    content.starts_with(keyword)
        && (content.len() == keyword.len() || !is_ident_char(content[keyword.len()]))
}

fn is_guard_line(content: &[u8]) -> bool {
    // RuboCop's next_sibling_empty_or_guard_clause? only skips when the next
    // sibling is an if/unless node that contains a guard clause. It does NOT
    // skip for bare guard statements (return, raise, etc.).
    //
    // So we only match:
    // 1. Modifier form on the same line: `return x if cond`, `raise "..." unless something`
    // 2. Lines that start with `if`/`unless` keyword followed by a guard inside
    //    (handled separately by is_multiline_guard_block)
    //
    // Bare guard statements like `raise "error"` or `return foo` are NOT
    // considered guard lines for the purpose of this check.
    for keyword in GUARD_METHODS {
        if starts_with_keyword(content, keyword) {
            // Check if this line also has a modifier `if` or `unless`
            if contains_word(content, b"if") || contains_word(content, b"unless") {
                return true;
            }
            // Bare guard statement without modifier — not a guard clause
            return false;
        }
    }
    // Also check modifier if/unless containing a guard
    if contains_modifier_guard(content) {
        return true;
    }
    false
}

/// Check if the next code line starts a multi-line if/unless block that contains
/// a guard clause (return/raise/fail/throw/next/break).
fn is_multiline_guard_block(content: &[u8], lines: &[&[u8]], start_idx: usize) -> bool {
    if !starts_with_keyword(content, b"if") && !starts_with_keyword(content, b"unless") {
        return false;
    }

    let content_line_idx = match find_line_index_from(lines, start_idx, content) {
        Some(idx) => idx,
        None => return false,
    };

    let mut depth: i32 = 1;
    for line in &lines[(content_line_idx + 1)..] {
        let Some(start) = line.iter().position(|&b| b != b' ' && b != b'\t') else {
            continue;
        };
        let trimmed = &line[start..];

        if starts_with_keyword(trimmed, b"if")
            || starts_with_keyword(trimmed, b"unless")
            || starts_with_keyword(trimmed, b"def")
            || starts_with_keyword(trimmed, b"class")
            || starts_with_keyword(trimmed, b"module")
            || starts_with_keyword(trimmed, b"do")
            || starts_with_keyword(trimmed, b"begin")
            || starts_with_keyword(trimmed, b"case")
        {
            depth += 1;
        }

        if starts_with_keyword(trimmed, b"end") {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }

        if depth == 1 {
            for keyword in GUARD_METHODS {
                if starts_with_keyword(trimmed, keyword) {
                    return true;
                }
            }
            if is_guard_line(trimmed) {
                return true;
            }
        }
    }
    false
}

fn find_line_index_from(lines: &[&[u8]], from_idx: usize, content: &[u8]) -> Option<usize> {
    for (i, line) in lines.iter().enumerate().skip(from_idx) {
        if let Some(start) = line.iter().position(|&b| b != b' ' && b != b'\t') {
            let trimmed = &line[start..];
            if std::ptr::eq(trimmed.as_ptr(), content.as_ptr()) || trimmed == content {
                return Some(i);
            }
        }
    }
    None
}

fn contains_modifier_guard(content: &[u8]) -> bool {
    if !contains_word(content, b"if") && !contains_word(content, b"unless") {
        return false;
    }
    for keyword in GUARD_METHODS {
        if contains_word(content, keyword) {
            return true;
        }
    }
    false
}

fn contains_word(haystack: &[u8], word: &[u8]) -> bool {
    let wlen = word.len();
    if haystack.len() < wlen {
        return false;
    }
    for i in 0..=(haystack.len() - wlen) {
        if &haystack[i..i + wlen] == word {
            let before_ok = i == 0 || !is_ident_char(haystack[i - 1]);
            let after_ok = i + wlen >= haystack.len() || !is_ident_char(haystack[i + wlen]);
            if before_ok && after_ok {
                return true;
            }
        }
    }
    false
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Check if a line is an "allowed directive comment" per RuboCop's definition.
/// This includes `rubocop:enable` directives and `:nocov:` comments, but NOT
/// `rubocop:disable` directives. RuboCop treats `rubocop:enable` specially
/// because it pairs with a preceding `rubocop:disable` that wraps the guard,
/// so the blank line should go after the `enable` comment, not between the
/// guard and the `enable`.
fn is_allowed_directive_comment(line: &[u8]) -> bool {
    let Some(trimmed) = trim_to_comment_content(line) else {
        return false;
    };
    // rubocop:enable is allowed (but NOT rubocop:disable)
    trimmed.starts_with(b"rubocop:enable") || trimmed.starts_with(b":nocov:")
}

/// Extract the content after `#` from a comment line, trimming whitespace.
/// Returns None if the line is not a comment.
fn trim_to_comment_content(line: &[u8]) -> Option<&[u8]> {
    let start = line.iter().position(|&b| b != b' ' && b != b'\t')?;
    let content = &line[start..];
    if !content.starts_with(b"#") {
        return None;
    }
    let after_hash = &content[1..];
    let trimmed = after_hash
        .iter()
        .position(|&b| b != b' ')
        .map(|i| &after_hash[i..])
        .unwrap_or(b"");
    Some(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        EmptyLineAfterGuardClause,
        "cops/layout/empty_line_after_guard_clause"
    );
    crate::cop_autocorrect_fixture_tests!(
        EmptyLineAfterGuardClause,
        "cops/layout/empty_line_after_guard_clause"
    );

    #[test]
    fn and_return_guard_detected() {
        let source = b"def bar\n  render :foo and return if condition\n  do_something\nend\n";
        let diags = crate::testutil::run_cop_full(&EmptyLineAfterGuardClause, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for `and return` guard, got {}: {:?}",
            diags.len(),
            diags
        );
    }

    #[test]
    fn or_return_guard_detected() {
        let source = b"def baz\n  render :foo or return if condition\n  do_something\nend\n";
        let diags = crate::testutil::run_cop_full(&EmptyLineAfterGuardClause, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for `or return` guard, got {}: {:?}",
            diags.len(),
            diags
        );
    }

    #[test]
    fn guard_before_begin_detected() {
        let source = b"def foo\n  return another_object if something_different?\n  begin\n    bar\n  rescue SomeException\n    baz\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&EmptyLineAfterGuardClause, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for guard before begin, got {}: {:?}",
            diags.len(),
            diags
        );
    }

    #[test]
    fn guard_then_rubocop_disable_detected() {
        let source = b"def foo\n  return if condition\n  # rubocop:disable Department/Cop\n  bar\n  # rubocop:enable Department/Cop\nend\n";
        let diags = crate::testutil::run_cop_full(&EmptyLineAfterGuardClause, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for guard then rubocop:disable, got {}: {:?}",
            diags.len(),
            diags
        );
    }

    #[test]
    fn ternary_guard_detected() {
        let source = b"def foo\n  puts 'some action happens here'\nrescue => e\n  a_check ? raise(e) : other_thing\n  true\nend\n";
        let diags = crate::testutil::run_cop_full(&EmptyLineAfterGuardClause, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for ternary guard, got {}: {:?}",
            diags.len(),
            diags
        );
    }

    #[test]
    fn guard_then_rubocop_enable_then_code_detected() {
        let source = b"def foo\n  # rubocop:disable Department/Cop\n  return if condition\n  # rubocop:enable Department/Cop\n  bar\nend\n";
        let diags = crate::testutil::run_cop_full(&EmptyLineAfterGuardClause, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for guard then rubocop:enable then code, got {}: {:?}",
            diags.len(),
            diags
        );
    }
}
