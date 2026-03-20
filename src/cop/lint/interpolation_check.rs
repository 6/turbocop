use crate::cop::node_type::STRING_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for interpolation in a single quoted string.
///
/// Root cause analysis (corpus: 67 FP, 275 FN at 53.7%, then 19 FP, 19 FN at 94.3%):
///
/// Previous FP causes (fixed):
/// - Missing backslash-escaped `#` check: `'\#{foo}'` has `\` before `#` in the
///   source text. RuboCop's `(?<!\\)#\{.*\}` regex skips these.
/// - Missing `valid_syntax?` check: patterns like `'#{%<expression>s}'` are not
///   valid Ruby interpolation.
///
/// Previous FN causes (fixed):
/// - Overly aggressive double-quote filter: `content_bytes.contains(&b'"')` skipped
///   ALL strings containing `"`, but RuboCop only skips when converting to
///   double-quoted produces invalid syntax.
///
/// Round 2 (19 FP, 19 FN at 94.3%):
///
/// FP causes:
/// - Multiline `#{}` matching: `has_unescaped_interpolation` searched for `}` across
///   newlines, but RuboCop's regex `.*` doesn't cross lines. Single-quoted strings
///   spanning multiple lines with `#{` and `}` on different lines were falsely flagged.
///
/// FN causes:
/// - `%q` validity check: RuboCop's `gsub(/\A'|'\z/, '"')` doesn't modify `%q{...}`
///   strings (no leading/trailing `'`), so it parses the original `%q{...}` which is
///   always valid Ruby. nitrocop was converting `%q{content}` to `"content"` which
///   could fail when content contained inner double quotes or format directives.
/// - Prism error filtering: `"BEGIN is permitted only at toplevel"` error wasn't
///   filtered, but Parser gem accepts this as valid syntax.
///
/// Fix: Restrict `}` search to same line (matching `.*` behavior), always return
/// true for `%q` validity (matching RuboCop's gsub behavior), filter additional
/// Prism-specific context errors.
///
/// Round 3 (56 FP, 0 FN at 92.1%):
///
/// FP causes — Prism/Parser gem divergences in `valid_syntax?`:
/// - `BEGIN` in interpolation: nitrocop was filtering "BEGIN is permitted" Prism
///   errors, treating `'#{BEGIN { ... }}'` as valid. But the Parser gem rejects
///   BEGIN inside interpolation (returns nil AST), so RuboCop's `valid_syntax?`
///   returns false. Fix: stop filtering "BEGIN is permitted" errors.
/// - `\U` escape: In single-quoted strings, `\U` is literal backslash + U. When
///   converted to double-quoted, Prism accepts `\U` as an unknown escape (treated
///   as literal), but the Parser gem throws a fatal `SyntaxError`. Fix: pre-check
///   for `\U` in content and reject it before Prism parsing.
///
/// The "Invalid " error filter (for yield, retry, break, next, redo) remains
/// correct — the Parser gem accepts these as valid syntax while Prism rejects them.
///
/// Round 4 (56 FP → 0 FP, 0 FN target):
///
/// Previous (incorrect) analysis: claimed the Parser gem rejects ALL non-standard
/// uppercase escape sequences. Actually, only `\U` is fatally rejected (it looks
/// like an incomplete unicode escape). Other uppercase escapes like `\A`, `\B`, `\D`,
/// `\Z` are accepted by the Parser gem as non-standard escapes (with deprecation
/// warning, but `valid_syntax?` returns true). The blanket rejection of all uppercase
/// escapes caused FN=2 in corpus (strings with `\A`/`\z` + interpolation were
/// incorrectly skipped).
///
/// Fix (round 4 correction): narrowed `has_parser_rejected_escape` to only reject
/// `\U`. Other uppercase escapes pass through to Prism parsing as before.
///
/// Round 5 (56 FP from `%q{...}` strings — reverted):
///
/// Previous (incorrect) analysis claimed v1.85+ `dstr_type?` check causes `%q`
/// to be skipped. Actually, RuboCop 1.84.2 DOES flag single-line `%q` strings.
/// The gsub doesn't modify `%q{...}` (no leading `'`), so parsing the original
/// `%q{...}` always succeeds, and `valid_syntax?` returns true.
///
/// The previous blanket `%q` skip was wrong — removed in round 6.
///
/// Round 6 (1 FP multiline, 56 FN `%q` strings):
///
/// FP root cause: The Parser gem represents multiline single-quoted strings as
/// `dstr` nodes with `str` children. The child `str` nodes have no `loc.begin`
/// or `loc.end`, so RuboCop's `return unless node.loc.begin && node.loc.end`
/// skips them. Prism keeps these as a single `StringNode` with opening/closing
/// quotes, so nitrocop previously flagged them. Fix: skip strings whose content
/// (between quotes) contains newlines, matching the Parser gem's dstr split.
///
/// FN root cause: The blanket `%q` skip from round 5 was incorrect. RuboCop
/// 1.84.2 flags single-line `%q` strings. The `gsub(/\A'|'\z/, '"')` doesn't
/// modify `%q{...}` (no leading/trailing `'`), and parsing the unchanged `%q`
/// is always valid. Multiline `%q` strings are naturally skipped by the new
/// multiline check. Fix: remove the `%q` blanket skip, handle `%q` in
/// `valid_syntax_as_double_quoted` by returning true (since gsub is a no-op).
pub struct InterpolationCheck;

impl Cop for InterpolationCheck {
    fn name(&self) -> &'static str {
        "Lint/InterpolationCheck"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[STRING_NODE]
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
        let string_node = match node.as_string_node() {
            Some(s) => s,
            None => return,
        };

        // Only check single-quoted strings.
        // opening_loc gives us the quote character (', ", %q{, etc.)
        let opening = match string_node.opening_loc() {
            Some(loc) => loc,
            None => return, // bare string (heredoc body, %w element, etc.)
        };

        let open_slice = opening.as_slice();

        // Only check single-quoted strings: either ' or %q delimiters
        let is_pctq = open_slice.starts_with(b"%q");
        if !is_pctq && open_slice != b"'" {
            return;
        }

        // Get the full source span of the string node (including quotes)
        let node_start = opening.start_offset();
        let closing = match string_node.closing_loc() {
            Some(loc) => loc,
            None => return,
        };
        let node_end = closing.end_offset();
        let node_source = &source.as_bytes()[node_start..node_end];

        // Skip multiline strings. The Parser gem represents multiline single-quoted
        // strings as dstr nodes with str children that have no loc.begin/loc.end,
        // causing RuboCop to skip them. We match this by checking if the content
        // between the opening and closing delimiters contains newlines.
        let content_start = opening.end_offset();
        let content_end = closing.start_offset();
        let content_bytes = &source.as_bytes()[content_start..content_end];
        if content_bytes.contains(&b'\n') {
            return;
        }

        // Match RuboCop's regex: /(?<!\\)#\{.*\}/
        // Look for #{ not preceded by backslash in the source text
        if !has_unescaped_interpolation(node_source) {
            return;
        }

        // valid_syntax? check: convert to double-quoted and see if it parses
        if !valid_syntax_as_double_quoted(node_source) {
            return;
        }

        // Report at the string node's opening quote (matching RuboCop)
        let (line, column) = source.offset_to_line_col(node_start);
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Interpolation in single quoted string detected. Use double quoted strings if you need interpolation.".to_string(),
        ));
    }
}

/// Check if the source bytes contain `#{...}` not preceded by `\`.
/// Matches RuboCop's `/(?<!\\)#\{.*\}/` regex behavior.
///
/// Important: RuboCop's regex uses `.*` which does NOT match newlines by default.
/// So `#{` and `}` must be on the same line for the regex to match.
fn has_unescaped_interpolation(source: &[u8]) -> bool {
    let mut i = 0;
    while i + 1 < source.len() {
        if source[i] == b'#' && source[i + 1] == b'{' {
            // Check if preceded by backslash
            if i == 0 || source[i - 1] != b'\\' {
                // Check there's a closing } on the SAME LINE (matching Ruby's `.*` behavior)
                let rest = &source[i + 2..];
                for &b in rest {
                    if b == b'}' {
                        return true;
                    }
                    if b == b'\n' {
                        // Newline before closing } — Ruby's .* doesn't cross lines
                        break;
                    }
                }
            }
        }
        i += 1;
    }
    false
}

/// Convert the single-quoted string source to double-quoted and check if it
/// parses as valid Ruby. Matches RuboCop's `valid_syntax?` method.
///
/// RuboCop uses `ProcessedSource#valid_syntax?` which considers the source valid
/// if parsing doesn't produce fatal errors. Prism is stricter than the Parser gem --
/// it reports semantic errors like "Invalid yield" (yield outside method) as errors,
/// while the Parser gem treats these as valid syntax. We filter out known semantic
/// errors to match RuboCop behavior.
///
/// For `%q` strings, RuboCop's `gsub(/\A'|'\z/, '"')` doesn't modify the source
/// (no leading/trailing `'`), so parsing the original `%q{...}` always succeeds.
/// We match this by always returning true for `%q` strings.
fn valid_syntax_as_double_quoted(source: &[u8]) -> bool {
    // source is the full string including quotes, e.g. b"'foo #{bar}'"
    let source_str = match std::str::from_utf8(source) {
        Ok(s) => s,
        Err(_) => return false,
    };

    // For %q strings, RuboCop's gsub(/\A'|'\z/, '"') doesn't modify the source
    // (no leading/trailing '), so it parses the original %q{...} which is always
    // valid Ruby. Return true immediately.
    if source_str.starts_with("%q") {
        return true;
    }

    let double_quoted = if source_str.starts_with('\'') && source_str.ends_with('\'') {
        // Simple single-quoted: 'content' -> "content"
        format!("\"{}\"", &source_str[1..source_str.len() - 1])
    } else {
        return false;
    };

    // Pre-check: reject backslash sequences that the Parser gem rejects but Prism
    // accepts. In single-quoted strings these are literal text, but when converted
    // to double-quoted they become escape sequences with different Parser/Prism
    // behavior.
    // - \U: Prism accepts as unknown escape (literal), Parser throws fatal error.
    let content = &source_str[1..source_str.len() - 1];
    if has_parser_rejected_escape(content) {
        return false;
    }

    // Parse with Prism and check for syntax errors.
    // Filter out semantic errors (e.g., "Invalid yield", "Invalid retry") that
    // the Parser gem accepts but Prism rejects. These start with "Invalid" and
    // represent runtime-checked conditions, not true syntax problems.
    // Note: "BEGIN is permitted only at toplevel" is NOT filtered — the Parser gem
    // rejects BEGIN inside interpolation (returns ast=nil), so we must reject it too.
    let result = ruby_prism::parse(double_quoted.as_bytes());
    let has_syntax_error = result.errors().any(|e| {
        let msg = e.message();
        let msg_bytes = msg.as_bytes();
        // Filter semantic errors that Parser gem accepts:
        // - "Invalid yield", "Invalid retry", "Invalid break", etc.
        !msg_bytes.starts_with(b"Invalid ")
    });
    !has_syntax_error
}

/// Check if the content (between quotes) contains backslash escape sequences
/// that the Parser gem rejects but Prism accepts in double-quoted strings.
///
/// In single-quoted strings, `\X` is literal backslash + X. When converted to
/// double-quoted, these become escape sequences.
///
/// The Parser gem only fatally rejects `\U` (it looks like an incomplete unicode
/// escape `\u`). Other uppercase escapes like `\A`, `\B`, `\D`, `\Z` are treated
/// as non-standard/unknown escapes — the Parser gem accepts them (possibly with a
/// deprecation warning, but `valid_syntax?` returns true). Only `\U` causes a
/// fatal SyntaxError.
fn has_parser_rejected_escape(content: &str) -> bool {
    let bytes = content.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'\\' {
            let next = bytes[i + 1];
            // Only \U is fatally rejected by the Parser gem
            if next == b'U' {
                return true;
            }
            // Skip past the escaped character to avoid double-processing
            i += 2;
            continue;
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InterpolationCheck, "cops/lint/interpolation_check");

    #[test]
    fn test_has_unescaped_interpolation() {
        assert!(has_unescaped_interpolation(b"'hello #{name}'"));
        assert!(!has_unescaped_interpolation(b"'hello \\#{name}'"));
        assert!(!has_unescaped_interpolation(b"'hello world'"));
        assert!(has_unescaped_interpolation(b"'#{bar}'"));
    }

    #[test]
    fn test_valid_syntax_as_double_quoted() {
        assert!(valid_syntax_as_double_quoted(b"'hello #{name}'"));
        assert!(valid_syntax_as_double_quoted(b"'#{bar}'"));
        assert!(valid_syntax_as_double_quoted(b"'foo \"#{bar}\"'"));
        assert!(!valid_syntax_as_double_quoted(b"'#{%<expression>s}'"));
    }

    #[test]
    fn test_valid_syntax_yield() {
        // yield.upcase is valid Ruby syntax (yield outside method is a semantic
        // error that Prism flags but Parser gem accepts)
        assert!(valid_syntax_as_double_quoted(
            b"'THIS. IS. #{yield.upcase}!'"
        ));
    }

    #[test]
    fn test_pctq_valid_syntax() {
        // For %q strings, RuboCop's gsub doesn't modify them (no leading/trailing '),
        // so parsing the original %q{...} always succeeds. valid_syntax should return true.
        assert!(valid_syntax_as_double_quoted(b"%q{text \"#{name}\"}"));
        assert!(valid_syntax_as_double_quoted(b"%q(#{foo})"));
        assert!(valid_syntax_as_double_quoted(b"%q[#{bar}]"));
        assert!(valid_syntax_as_double_quoted(b"%q|#{baz}|"));
    }

    #[test]
    fn test_multiline_interpolation_not_matched() {
        // RuboCop's regex .* doesn't cross newlines, so #{...} split across
        // lines should NOT be matched
        assert!(!has_unescaped_interpolation(b"'text #{\n  foo\n}'"));
        assert!(!has_unescaped_interpolation(b"'#{\nbar\n}'"));
        // But single-line should still match
        assert!(has_unescaped_interpolation(b"'text #{foo}'"));
    }

    #[test]
    fn test_double_backslash_interpolation() {
        // '\\#{foo}' - source bytes: ' \ \ # { f o o } '
        // In Ruby source, \\ in single-quoted string is escaped backslash
        // RuboCop regex (?<!\\) checks char before # which is \, so no match
        // nitrocop should also NOT match (char before # is \)
        assert!(!has_unescaped_interpolation(b"'\\\\#{foo}'"));
        // '\\\\#{foo}' - four backslashes then #{foo}
        assert!(!has_unescaped_interpolation(b"'\\\\\\\\#{foo}'"));
    }

    #[test]
    fn test_begin_in_interpolation_invalid() {
        // BEGIN inside interpolation: Parser gem rejects (ast=nil), so
        // RuboCop's valid_syntax? returns false. We must match this.
        assert!(!valid_syntax_as_double_quoted(b"'#{BEGIN {}}'"));
        assert!(!valid_syntax_as_double_quoted(b"'test #{BEGIN { x = 1 }}'"));
    }

    #[test]
    fn test_backslash_u_uppercase_invalid() {
        // \U in single-quoted string is literal. When converted to double-quoted,
        // Parser gem throws fatal SyntaxError, but Prism accepts it.
        // We must reject it to match RuboCop.
        assert!(!valid_syntax_as_double_quoted(b"'\\U+0041 #{foo}'"));
        assert!(!valid_syntax_as_double_quoted(b"'\\U #{bar}'"));
        // Lowercase \u with valid hex is fine
        assert!(valid_syntax_as_double_quoted(b"'#{foo}'"));
    }

    #[test]
    fn test_only_backslash_u_uppercase_rejected() {
        // Only \U is fatally rejected by the Parser gem
        assert!(has_parser_rejected_escape("\\U+0041 #{foo}"));
        assert!(has_parser_rejected_escape("\\U #{foo}"));
        // Other uppercase escapes are NOT rejected — Parser gem accepts them
        for ch in b"ABCDEFGHIJKLMNOPQRSTVWXYZ" {
            let content = format!("\\{} #{{foo}}", *ch as char);
            assert!(
                !has_parser_rejected_escape(&content),
                "Expected \\{} to NOT be rejected",
                *ch as char
            );
        }
    }
}
