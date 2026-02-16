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
        let is_guard = is_guard_stmt(first_stmt);

        if !is_guard {
            return Vec::new();
        }

        // Check the next line after this guard clause
        let (if_end_line, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));

        let lines: Vec<&[u8]> = source.lines().collect();

        // Check if next line exists
        if if_end_line >= lines.len() {
            return Vec::new();
        }

        let next_line = lines[if_end_line]; // 0-indexed for next line

        // If next line is blank, that's fine
        if util::is_blank_line(next_line) {
            return Vec::new();
        }

        // If next line is end/else/elsif/rescue/ensure/when, skip
        let trimmed = next_line.iter().position(|&b| b != b' ' && b != b'\t');
        if let Some(start) = trimmed {
            let content = &next_line[start..];
            if content.starts_with(b"end")
                || content.starts_with(b"else")
                || content.starts_with(b"elsif")
                || content.starts_with(b"rescue")
                || content.starts_with(b"ensure")
                || content.starts_with(b"when")
            {
                return Vec::new();
            }
            // Skip if next line is another guard clause (consecutive guards are OK)
            if is_guard_line(content) {
                return Vec::new();
            }
        }

        let (line, col) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));
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

fn is_guard_line(content: &[u8]) -> bool {
    // Check if line starts with a guard keyword (e.g., `return if cond`)
    for keyword in GUARD_METHODS {
        if content.starts_with(keyword) {
            let after = content.get(keyword.len()..);
            if let Some(rest) = after {
                if rest.is_empty() || rest.starts_with(b" ") || rest.starts_with(b"(") {
                    return true;
                }
            }
        }
    }
    // Check if line contains a guard keyword embedded in an expression with
    // modifier if/unless (e.g., `expr && return if cond`).
    // RuboCop considers these guard clauses via AST-level guard_clause? check.
    if contains_modifier_guard(content) {
        return true;
    }
    false
}

/// Check if a line contains a guard keyword (return/raise/throw/break/next)
/// combined with a modifier `if` or `unless`, indicating it's a guard clause
/// even when the guard keyword isn't at the start of the line.
/// Matches patterns like `expr && return if cond` or `do_thing || raise "err" unless ok?`.
fn contains_modifier_guard(content: &[u8]) -> bool {
    // Must contain modifier if/unless somewhere
    if !contains_word(content, b"if") && !contains_word(content, b"unless") {
        return false;
    }
    // Must contain a guard keyword somewhere as a standalone word
    for keyword in GUARD_METHODS {
        if contains_word(content, keyword) {
            return true;
        }
    }
    false
}

/// Check if `haystack` contains `word` as a standalone word (not part of a larger identifier).
fn contains_word(haystack: &[u8], word: &[u8]) -> bool {
    let wlen = word.len();
    if haystack.len() < wlen {
        return false;
    }
    for i in 0..=(haystack.len() - wlen) {
        if &haystack[i..i + wlen] == word {
            // Check word boundary before
            let before_ok = i == 0 || !is_ident_char(haystack[i - 1]);
            // Check word boundary after
            let after_ok =
                i + wlen >= haystack.len() || !is_ident_char(haystack[i + wlen]);
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

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        EmptyLineAfterGuardClause,
        "cops/layout/empty_line_after_guard_clause"
    );
}
