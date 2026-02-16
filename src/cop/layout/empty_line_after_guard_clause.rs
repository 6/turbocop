use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyLineAfterGuardClause;

/// Guard clause keywords that appear at the start of an expression.
const GUARD_METHODS: &[&[u8]] = &[b"return", b"raise", b"fail", b"throw", b"next", b"break"];

impl Cop for EmptyLineAfterGuardClause {
    fn name(&self) -> &'static str {
        "Layout/EmptyLineAfterGuardClause"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Handle modifier `if` and modifier `unless` forms
        let (body_stmts, loc) = if let Some(if_node) = node.as_if_node() {
            // Must be modifier form (no `end` keyword)
            if if_node.end_keyword_loc().is_some() {
                return Vec::new();
            }
            match if_node.statements() {
                Some(s) => (s, if_node.location()),
                None => return Vec::new(),
            }
        } else if let Some(unless_node) = node.as_unless_node() {
            // Must be modifier form (no `end` keyword)
            if unless_node.end_keyword_loc().is_some() {
                return Vec::new();
            }
            match unless_node.statements() {
                Some(s) => (s, unless_node.location()),
                None => return Vec::new(),
            }
        } else {
            return Vec::new();
        };

        let stmts: Vec<_> = body_stmts.body().iter().collect();
        if stmts.is_empty() {
            return Vec::new();
        }

        let first_stmt = &stmts[0];
        if !is_guard_stmt(first_stmt) {
            return Vec::new();
        }

        let lines: Vec<&[u8]> = source.lines().collect();
        let end_offset = loc.end_offset().saturating_sub(1);
        let (if_end_line, end_col) = source.offset_to_line_col(end_offset);

        // Check if the guard clause is embedded inside a larger expression on the
        // same line (e.g. `arr.each { |x| return x if cond }`). If there is
        // non-comment code after the if node on the same line, skip.
        if let Some(cur_line) = lines.get(if_end_line.saturating_sub(1)) {
            let after_pos = end_col + 1;
            if after_pos < cur_line.len() {
                let rest = &cur_line[after_pos..];
                if let Some(idx) = rest.iter().position(|&b| b != b' ' && b != b'\t') {
                    if rest[idx] != b'#' {
                        return Vec::new();
                    }
                }
            }
        }

        // Check if next line exists
        if if_end_line >= lines.len() {
            return Vec::new();
        }

        // Find the next meaningful code line, skipping comment lines.
        // A blank line means the guard is properly followed by whitespace (no offense).
        if let Some(code_content) = find_next_code_line(&lines, if_end_line) {
            if is_scope_close_or_clause_keyword(code_content) {
                return Vec::new();
            }
            if is_guard_line(code_content) {
                return Vec::new();
            }
            if is_multiline_guard_block(code_content, &lines, if_end_line) {
                return Vec::new();
            }
        } else {
            // No more code lines (only comments/blanks until EOF)
            return Vec::new();
        }

        // Check for rubocop directive or nocov comments followed by blank line
        let next_line = lines[if_end_line];
        if is_rubocop_directive_or_nocov(next_line) {
            if if_end_line + 1 >= lines.len() || util::is_blank_line(lines[if_end_line + 1]) {
                return Vec::new();
            }
        }

        let (line, col) = source.offset_to_line_col(end_offset);
        vec![self.diagnostic(
            source,
            line,
            col,
            "Add empty line after guard clause.".to_string(),
        )]
    }
}

fn is_guard_stmt(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if GUARD_METHODS.iter().any(|m| *m == name) && call.receiver().is_none() {
            return true;
        }
    }
    // Bare return/break/next
    node.as_return_node().is_some()
        || node.as_break_node().is_some()
        || node.as_next_node().is_some()
}

/// Find the next non-blank, non-comment line starting from `start_idx` (0-indexed).
/// Returns None if a blank line is found first or we reach EOF.
fn find_next_code_line<'a>(lines: &[&'a [u8]], start_idx: usize) -> Option<&'a [u8]> {
    for i in start_idx..lines.len() {
        let line = lines[i];
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
    for keyword in GUARD_METHODS {
        if content.starts_with(keyword) {
            if let Some(rest) = content.get(keyword.len()..) {
                if rest.is_empty() || rest.starts_with(b" ") || rest.starts_with(b"(") {
                    return true;
                }
            }
        }
    }
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
    for i in (content_line_idx + 1)..lines.len() {
        let line = lines[i];
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
    for i in from_idx..lines.len() {
        let line = lines[i];
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

fn is_rubocop_directive_or_nocov(line: &[u8]) -> bool {
    let Some(start) = line.iter().position(|&b| b != b' ' && b != b'\t') else {
        return false;
    };
    let content = &line[start..];
    if !content.starts_with(b"#") {
        return false;
    }
    let after_hash = &content[1..];
    let trimmed = after_hash
        .iter()
        .position(|&b| b != b' ')
        .map(|i| &after_hash[i..])
        .unwrap_or(b"");
    trimmed.starts_with(b"rubocop:disable")
        || trimmed.starts_with(b"rubocop:enable")
        || trimmed.starts_with(b":nocov:")
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        EmptyLineAfterGuardClause,
        "cops/layout/empty_line_after_guard_clause"
    );
}
