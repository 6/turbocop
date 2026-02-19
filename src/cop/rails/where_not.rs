use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, STRING_NODE};

pub struct WhereNot;

/// Check if the SQL template string matches a simple negation pattern
/// that can be replaced with `where.not(...)`.
fn is_simple_negation(sql: &str) -> bool {
    let trimmed = sql.trim();

    // column != ? or column <> ?
    // Pattern: \A[\w.]+\s+(?:!=|<>)\s+\?\z
    if is_not_eq_anonymous(trimmed) {
        return true;
    }

    // column != :name or column <> :name
    // Pattern: \A[\w.]+\s+(?:!=|<>)\s+:\w+\z
    if is_not_eq_named(trimmed) {
        return true;
    }

    // column NOT IN (?)
    // Pattern: \A[\w.]+\s+NOT\s+IN\s+\(\?\)\z  (case insensitive)
    if is_not_in_anonymous(trimmed) {
        return true;
    }

    // column NOT IN (:name)
    // Pattern: \A[\w.]+\s+NOT\s+IN\s+\(:\w+\)\z  (case insensitive)
    if is_not_in_named(trimmed) {
        return true;
    }

    // column IS NOT NULL
    // Pattern: \A[\w.]+\s+IS\s+NOT\s+NULL\z  (case insensitive)
    if is_not_null(trimmed) {
        return true;
    }

    false
}

fn is_word_dot_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'.'
}

/// Match: word_or_dot+ whitespace+ (!=|<>) whitespace+ ?
fn is_not_eq_anonymous(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0;
    // Must start with word/dot chars
    if i >= bytes.len() || !is_word_dot_char(bytes[i]) {
        return false;
    }
    while i < bytes.len() && is_word_dot_char(bytes[i]) {
        i += 1;
    }
    // Check column qualifier doesn't have more than one dot
    let col = &s[..i];
    if col.chars().filter(|&c| c == '.').count() > 1 {
        return false;
    }
    // whitespace
    if i >= bytes.len() || bytes[i] != b' ' {
        return false;
    }
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    // != or <>
    if i + 1 >= bytes.len() {
        return false;
    }
    if !((bytes[i] == b'!' && bytes[i + 1] == b'=') || (bytes[i] == b'<' && bytes[i + 1] == b'>'))
    {
        return false;
    }
    i += 2;
    // whitespace
    if i >= bytes.len() || bytes[i] != b' ' {
        return false;
    }
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    // ?
    i < bytes.len() && bytes[i] == b'?' && i + 1 == bytes.len()
}

/// Match: word_or_dot+ whitespace+ (!=|<>) whitespace+ :word+
fn is_not_eq_named(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0;
    if i >= bytes.len() || !is_word_dot_char(bytes[i]) {
        return false;
    }
    while i < bytes.len() && is_word_dot_char(bytes[i]) {
        i += 1;
    }
    let col = &s[..i];
    if col.chars().filter(|&c| c == '.').count() > 1 {
        return false;
    }
    if i >= bytes.len() || bytes[i] != b' ' {
        return false;
    }
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    if i + 1 >= bytes.len() {
        return false;
    }
    if !((bytes[i] == b'!' && bytes[i + 1] == b'=') || (bytes[i] == b'<' && bytes[i + 1] == b'>'))
    {
        return false;
    }
    i += 2;
    if i >= bytes.len() || bytes[i] != b' ' {
        return false;
    }
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    // :word+
    if i >= bytes.len() || bytes[i] != b':' {
        return false;
    }
    i += 1;
    let start = i;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        i += 1;
    }
    i > start && i == bytes.len()
}

fn eq_ignore_case(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.to_ascii_lowercase() == y.to_ascii_lowercase())
}

/// Match: word_or_dot+ whitespace+ NOT whitespace+ IN whitespace+ (?) (case insensitive)
fn is_not_in_anonymous(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0;
    if i >= bytes.len() || !is_word_dot_char(bytes[i]) {
        return false;
    }
    while i < bytes.len() && is_word_dot_char(bytes[i]) {
        i += 1;
    }
    let col = &s[..i];
    if col.chars().filter(|&c| c == '.').count() > 1 {
        return false;
    }
    // whitespace
    if i >= bytes.len() || bytes[i] != b' ' {
        return false;
    }
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    // NOT
    if i + 3 > bytes.len() || !eq_ignore_case(&bytes[i..i + 3], b"NOT") {
        return false;
    }
    i += 3;
    if i >= bytes.len() || bytes[i] != b' ' {
        return false;
    }
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    // IN
    if i + 2 > bytes.len() || !eq_ignore_case(&bytes[i..i + 2], b"IN") {
        return false;
    }
    i += 2;
    if i >= bytes.len() || bytes[i] != b' ' {
        return false;
    }
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    // (?)
    i + 3 == bytes.len() && bytes[i] == b'(' && bytes[i + 1] == b'?' && bytes[i + 2] == b')'
}

/// Match: word_or_dot+ whitespace+ NOT whitespace+ IN whitespace+ (:word+) (case insensitive)
fn is_not_in_named(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0;
    if i >= bytes.len() || !is_word_dot_char(bytes[i]) {
        return false;
    }
    while i < bytes.len() && is_word_dot_char(bytes[i]) {
        i += 1;
    }
    let col = &s[..i];
    if col.chars().filter(|&c| c == '.').count() > 1 {
        return false;
    }
    if i >= bytes.len() || bytes[i] != b' ' {
        return false;
    }
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    if i + 3 > bytes.len() || !eq_ignore_case(&bytes[i..i + 3], b"NOT") {
        return false;
    }
    i += 3;
    if i >= bytes.len() || bytes[i] != b' ' {
        return false;
    }
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    if i + 2 > bytes.len() || !eq_ignore_case(&bytes[i..i + 2], b"IN") {
        return false;
    }
    i += 2;
    if i >= bytes.len() || bytes[i] != b' ' {
        return false;
    }
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    // (:word+)
    if i >= bytes.len() || bytes[i] != b'(' {
        return false;
    }
    i += 1;
    if i >= bytes.len() || bytes[i] != b':' {
        return false;
    }
    i += 1;
    let start = i;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        i += 1;
    }
    if i <= start {
        return false;
    }
    i < bytes.len() && bytes[i] == b')' && i + 1 == bytes.len()
}

/// Match: word_or_dot+ whitespace+ IS whitespace+ NOT whitespace+ NULL (case insensitive)
fn is_not_null(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0;
    if i >= bytes.len() || !is_word_dot_char(bytes[i]) {
        return false;
    }
    while i < bytes.len() && is_word_dot_char(bytes[i]) {
        i += 1;
    }
    let col = &s[..i];
    if col.chars().filter(|&c| c == '.').count() > 1 {
        return false;
    }
    if i >= bytes.len() || bytes[i] != b' ' {
        return false;
    }
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    if i + 2 > bytes.len() || !eq_ignore_case(&bytes[i..i + 2], b"IS") {
        return false;
    }
    i += 2;
    if i >= bytes.len() || bytes[i] != b' ' {
        return false;
    }
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    if i + 3 > bytes.len() || !eq_ignore_case(&bytes[i..i + 3], b"NOT") {
        return false;
    }
    i += 3;
    if i >= bytes.len() || bytes[i] != b' ' {
        return false;
    }
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    if i + 4 > bytes.len() || !eq_ignore_case(&bytes[i..i + 4], b"NULL") {
        return false;
    }
    i += 4;
    i == bytes.len()
}

impl Cop for WhereNot {
    fn name(&self) -> &'static str {
        "Rails/WhereNot"
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
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"where" {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        // First argument must be a string literal
        let first_arg = &arg_list[0];
        let sql_content = if let Some(str_node) = first_arg.as_string_node() {
            String::from_utf8_lossy(str_node.unescaped()).to_string()
        } else {
            return Vec::new();
        };

        if !is_simple_negation(&sql_content) {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `where.not(...)` instead of manually constructing negated SQL.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(WhereNot, "cops/rails/where_not");
}
