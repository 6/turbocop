use crate::cop::shared::node_type::{CALL_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/WhereNot - detects manually constructed negated SQL in `where` calls.
///
/// ## Corpus investigation findings
///
/// FN root cause (35 FN): RuboCop's `where_method_call?` pattern matches two forms:
///
/// - `(call _ :where $str_type? $_ ?)` -- bare string arg: `where('name != ?', val)`
/// - `(call _ :where (array $str_type? $_ ?))` -- array-wrapped: `where(['name != ?', val])`
///
/// Nitrocop originally only handled the bare string form; the array-wrapped form was missed.
///
/// FP root cause (27 FP): RuboCop's `offense_range` starts at `node.loc.selector`
/// (the `where` method name), not the full node including receiver. Nitrocop used
/// `node.location()` which includes the receiver (e.g., `User.where(...)` vs `where(...)`).
/// On multiline chains this causes line-number mismatches: nitrocop reports on the receiver
/// line while RuboCop reports on the `where` line, creating paired FP+FN on adjacent lines.
///
/// Fix applied: Added array-unwrapping for first arg, and changed offense location
/// to start at `call.message_loc()` (the `where` keyword) instead of `node.location()`.
///
/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=1, FN=0.
///
/// FP=1: `builder.where("id NOT IN (:selected_tag_ids)")` — named parameter
/// form without a hash second argument. RuboCop's `extract_column_and_value`
/// requires a hash argument for named patterns (`:name`) and a positional
/// argument for anonymous patterns (`?`). Fixed by classifying negation
/// patterns into Anonymous/Named/IsNotNull and validating that the required
/// value argument exists.
///
/// ## Corpus investigation (2026-03-24)
///
/// Extended corpus reported FP=2, FN=0.
///
/// FP=1: `where(["state not in (?) ", ...])` — trailing space in SQL template.
/// `negation_type()` called `sql.trim()` before matching, but RuboCop uses
/// `\A...\z` anchored regex without trimming. Removed `trim()` call.
///
/// FP=2: `where("repositories.private <> ?", true, user.repository_ids)` — 3
/// call arguments. RuboCop's pattern `(call _ :where $str_type? $_ ?)` matches
/// at most 2 args. Added early return when bare string form has >2 args.
pub struct WhereNot;

/// Type of negation pattern found in SQL.
enum NegationType {
    /// `?` placeholder — requires a positional value argument
    Anonymous,
    /// `:name` placeholder — requires a hash value argument
    Named,
    /// `IS NOT NULL` — no value argument needed
    IsNotNull,
}

/// Check if the SQL template string matches a simple negation pattern
/// that can be replaced with `where.not(...)`.
fn negation_type(sql: &str) -> Option<NegationType> {
    // Do NOT trim whitespace: RuboCop uses `\A...\z` anchored regex without
    // trimming, so trailing/leading spaces cause a non-match.
    if is_not_eq_anonymous(sql) || is_not_in_anonymous(sql) {
        return Some(NegationType::Anonymous);
    }

    if is_not_eq_named(sql) || is_not_in_named(sql) {
        return Some(NegationType::Named);
    }

    if is_not_null(sql) {
        return Some(NegationType::IsNotNull);
    }

    None
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
    if !((bytes[i] == b'!' && bytes[i + 1] == b'=') || (bytes[i] == b'<' && bytes[i + 1] == b'>')) {
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
    if !((bytes[i] == b'!' && bytes[i + 1] == b'=') || (bytes[i] == b'<' && bytes[i + 1] == b'>')) {
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
    a.len() == b.len()
        && a.iter()
            .zip(b.iter())
            .all(|(x, y)| x.eq_ignore_ascii_case(y))
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
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"where" {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // RuboCop matches two forms:
        // 1. where('col != ?', val) — bare string first arg
        // 2. where(['col != ?', val]) — array-wrapped first arg
        let first_arg = &arg_list[0];
        let (sql_content, has_value_arg, has_hash_arg) =
            if let Some(str_node) = first_arg.as_string_node() {
                // Form 1: bare string — value args are remaining call arguments.
                // RuboCop's pattern `(call _ :where $str_type? $_ ?)` matches at most
                // 2 call args (template + optional value). Reject 3+ args.
                if arg_list.len() > 2 {
                    return;
                }
                let has_val = arg_list.len() == 2;
                let has_hash = arg_list.len() == 2
                    && (arg_list[1].as_hash_node().is_some()
                        || arg_list[1].as_keyword_hash_node().is_some());
                (
                    String::from_utf8_lossy(str_node.unescaped()).to_string(),
                    has_val,
                    has_hash,
                )
            } else if let Some(array_node) = first_arg.as_array_node() {
                // Form 2: array-wrapped — value args are remaining array elements
                let elements: Vec<_> = array_node.elements().iter().collect();
                if elements.is_empty() {
                    return;
                }
                let str_node = match elements[0].as_string_node() {
                    Some(s) => s,
                    None => return,
                };
                let has_val = elements.len() > 1;
                let has_hash = elements.len() > 1 && elements[1].as_hash_node().is_some();
                (
                    String::from_utf8_lossy(str_node.unescaped()).to_string(),
                    has_val,
                    has_hash,
                )
            } else {
                return;
            };

        let neg_type = match negation_type(&sql_content) {
            Some(t) => t,
            None => return,
        };

        // RuboCop's extract_column_and_value requires matching value arguments:
        // - Anonymous (?) patterns need a positional value argument
        // - Named (:name) patterns need a hash value argument
        // - IS NOT NULL needs no value argument
        match neg_type {
            NegationType::Anonymous => {
                if !has_value_arg {
                    return;
                }
            }
            NegationType::Named => {
                if !has_hash_arg {
                    return;
                }
            }
            NegationType::IsNotNull => {}
        }

        // Use message_loc to start offense at `where` keyword (matching RuboCop's
        // offense_range which uses node.loc.selector, not the full node with receiver)
        let loc = call.message_loc().unwrap_or(node.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `where.not(...)` instead of manually constructing negated SQL.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(WhereNot, "cops/rails/where_not");
}
