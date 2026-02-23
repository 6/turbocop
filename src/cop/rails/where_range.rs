use crate::cop::node_type::{CALL_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct WhereRange;

impl Cop for WhereRange {
    fn name(&self) -> &'static str {
        "Rails/WhereRange"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE]
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

        let method = call.name().as_slice();
        if method != b"where" && method != b"not" {
            return;
        }

        // For `not`, check that the receiver is a `where` call
        if method == b"not" {
            if let Some(recv) = call.receiver() {
                if let Some(recv_call) = recv.as_call_node() {
                    if recv_call.name().as_slice() != b"where" {
                        return;
                    }
                } else {
                    return;
                }
            } else {
                return;
            }
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // First argument should be a string containing a simple comparison pattern
        let string_node = match arg_list[0].as_string_node() {
            Some(s) => s,
            None => return,
        };

        let content = string_node.unescaped();

        // Must match one of RuboCop's specific patterns:
        // - "column >= ?" (with optional table prefix)
        // - "column <[=] ?"
        // - "column >= ? AND column <[=] ?"
        // - "column >= :name"
        // - "column <[=] :name"
        // - "column >= :name1 AND column <[=] :name2"
        if !matches_where_range_pattern(content) {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use a range in `where` instead of manually constructing SQL conditions.".to_string(),
        ));
    }
}

/// Check if content matches one of the specific SQL patterns that RuboCop's WhereRange flags.
/// These are simple column comparisons, not complex SQL expressions.
fn matches_where_range_pattern(content: &[u8]) -> bool {
    let s = match std::str::from_utf8(content) {
        Ok(s) => s.trim(),
        Err(_) => return false,
    };

    // Pattern: column >= ?
    if matches_simple_pattern(s, ">=", "?") {
        return true;
    }
    // Pattern: column <= ? or column < ?
    if matches_simple_pattern(s, "<=", "?") || matches_simple_pattern(s, "<", "?") {
        return true;
    }
    // Pattern: column >= :name
    if matches_named_pattern(s, ">=") {
        return true;
    }
    // Pattern: column <= :name or column < :name
    if matches_named_pattern(s, "<=") || matches_named_pattern(s, "<") {
        return true;
    }
    // Pattern: column >= ? AND column <[=] ?
    if matches_range_anonymous_pattern(s) {
        return true;
    }
    // Pattern: column >= :name AND column <[=] :name
    if matches_range_named_pattern(s) {
        return true;
    }

    false
}

/// Match pattern: `identifier >= ?` (simple column with anonymous placeholder)
fn matches_simple_pattern(s: &str, op: &str, placeholder: &str) -> bool {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() != 3 {
        return false;
    }
    is_column_identifier(parts[0]) && parts[1] == op && parts[2] == placeholder
}

/// Match pattern: `identifier >= :name` (simple column with named placeholder)
fn matches_named_pattern(s: &str, op: &str) -> bool {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() != 3 {
        return false;
    }
    is_column_identifier(parts[0])
        && parts[1] == op
        && parts[2].starts_with(':')
        && parts[2].len() > 1
        && parts[2][1..]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Match pattern: `column >= ? AND column <[=] ?`
fn matches_range_anonymous_pattern(s: &str) -> bool {
    let upper = s.to_uppercase();
    let parts: Vec<&str> = upper.split_whitespace().collect();
    // column >= ? AND column <[=] ?
    if parts.len() != 7 {
        return false;
    }
    // Use original case for column names
    let orig_parts: Vec<&str> = s.split_whitespace().collect();
    is_column_identifier(orig_parts[0])
        && parts[1] == ">="
        && parts[2] == "?"
        && parts[3] == "AND"
        && orig_parts[0] == orig_parts[4] // same column
        && (parts[5] == "<" || parts[5] == "<=")
        && parts[6] == "?"
}

/// Match pattern: `column >= :name AND column <[=] :name`
fn matches_range_named_pattern(s: &str) -> bool {
    let upper = s.to_uppercase();
    let parts: Vec<&str> = upper.split_whitespace().collect();
    if parts.len() != 7 {
        return false;
    }
    let orig_parts: Vec<&str> = s.split_whitespace().collect();
    is_column_identifier(orig_parts[0])
        && parts[1] == ">="
        && orig_parts[2].starts_with(':')
        && parts[3] == "AND"
        && orig_parts[0] == orig_parts[4]
        && (parts[5] == "<" || parts[5] == "<=")
        && orig_parts[6].starts_with(':')
}

/// Check if the string looks like a simple SQL column identifier (with optional table prefix).
/// Matches: word, word.word (but not word.word.word or expressions with parens)
fn is_column_identifier(s: &str) -> bool {
    if s.is_empty() || s.contains('(') || s.contains(')') {
        return false;
    }
    let dot_count = s.chars().filter(|&c| c == '.').count();
    if dot_count > 1 {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(WhereRange, "cops/rails/where_range");

    #[test]
    fn does_not_flag_complex_sql() {
        use crate::testutil::assert_cop_no_offenses;
        assert_cop_no_offenses(
            &WhereRange,
            b"User.where('COALESCE(status_stats.reblogs_count, 0) < ?', min_reblogs)\n",
        );
    }

    #[test]
    fn does_not_flag_non_comparison() {
        use crate::testutil::assert_cop_no_offenses;
        assert_cop_no_offenses(&WhereRange, b"User.where('name = ?', name)\n");
    }
}
