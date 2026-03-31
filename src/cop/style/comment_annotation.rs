use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/CommentAnnotation cop.
///
/// ## Investigation findings (2026-03-10)
///
/// Root cause of 235 FPs: bare keyword comments like `# TODO`, `# FIXME`, `# NOTE`
/// (keyword alone at end of line, no colon or space after) were being flagged as
/// "missing a note". RuboCop's `keyword_appearance?` method requires either a colon
/// or space after the keyword for it to count as an annotation. A bare keyword with
/// nothing after it is not considered an annotation and is silently accepted.
///
/// Fix: Changed the `keyword_appearance?` check from
/// `!has_colon && !has_space && !after_kw.is_empty()` to `!has_colon && !has_space`,
/// so bare keywords (empty after_kw) are also skipped. Also removed the redundant
/// gate at the offense registration site that checked `after_kw.starts_with(':')`
/// or `after_kw.starts_with(' ')`, since keyword_appearance is already validated
/// earlier. Additionally fixed the "missing a note" message condition to check
/// `has_note` instead of `after_kw.is_empty()`, matching RuboCop's behavior.
///
/// ## Investigation findings (2026-03-31)
///
/// Root cause of 1 FP: CRLF line endings. Prism includes the trailing `\r` in
/// the comment text for files with `\r\n` line endings. The `\r` is ASCII
/// whitespace, so for a bare keyword like `# HACK\r`, the `\r` caused
/// `has_space = true` while `has_note = false`, producing a spurious "missing
/// a note" offense. RuboCop's parser normalizes line endings so it is unaffected.
///
/// Fix: Strip trailing `\r` from comment text before analysis.
pub struct CommentAnnotation;

const DEFAULT_KEYWORDS: &[&str] = &["TODO", "FIXME", "OPTIMIZE", "HACK", "REVIEW", "NOTE"];

/// RuboCop's `just_keyword_of_sentence?`: if the keyword is exactly Capitalized
/// (e.g., `Note`, `Fixme`), has no colon, and has a space+note, it's just a word
/// in a sentence, not an annotation.
fn just_keyword_of_sentence(
    keyword_text: &str,
    has_colon: bool,
    has_space: bool,
    has_note: bool,
) -> bool {
    if has_colon || !has_space || !has_note {
        return false;
    }
    // Check if keyword is exactly capitalized: first char upper, rest lower
    let mut chars = keyword_text.chars();
    match chars.next() {
        Some(c) if c.is_ascii_uppercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase())
}

impl Cop for CommentAnnotation {
    fn name(&self) -> &'static str {
        "Style/CommentAnnotation"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let require_colon = config.get_bool("RequireColon", true);
        let keywords_opt = config.get_string_array("Keywords");
        let keywords: Vec<String> = keywords_opt
            .unwrap_or_else(|| DEFAULT_KEYWORDS.iter().map(|s| s.to_string()).collect());

        let bytes = source.as_bytes();
        let comments: Vec<_> = parse_result.comments().collect();

        for (idx, comment) in comments.iter().enumerate() {
            let loc = comment.location();
            let comment_start_offset = loc.start_offset();
            let comment_end_offset = loc.end_offset();
            let comment_text =
                match std::str::from_utf8(&bytes[comment_start_offset..comment_end_offset]) {
                    Ok(s) => s.trim_end_matches('\r'),
                    Err(_) => continue,
                };

            // RuboCop only checks the first line of a contiguous comment block,
            // or inline comments (comments on lines with code).
            let comment_line = source.offset_to_line_col(comment_start_offset).0;

            let is_inline = {
                // An inline comment has code before it on the same line.
                // The column tells us how many bytes precede the # on this line.
                let (_, col) = source.offset_to_line_col(comment_start_offset);
                if col == 0 {
                    false
                } else {
                    // Check if there's non-whitespace before the comment
                    let line_start = comment_start_offset - col;
                    let before = &bytes[line_start..comment_start_offset];
                    before.iter().any(|&b| !b.is_ascii_whitespace())
                }
            };

            if !is_inline {
                // Check if previous comment is on the immediately preceding line
                // (contiguous comment block). If so, skip — only flag first line.
                if idx > 0 {
                    let prev_loc = comments[idx - 1].location();
                    let prev_line = source.offset_to_line_col(prev_loc.start_offset()).0;
                    if prev_line == comment_line - 1 {
                        continue;
                    }
                }
            }

            // Must start with #
            if !comment_text.starts_with('#') {
                continue;
            }

            let after_hash = &comment_text[1..];

            // RuboCop only matches "# KEYWORD" (one space) or "#KEYWORD" (no space).
            // The regex margin group is `(# ?)`, so at most one space after `#`.
            let (margin_len, trimmed) = if let Some(stripped) = after_hash.strip_prefix(' ') {
                // One space after # — check if keyword starts at position 1
                if !stripped.is_empty() && stripped.as_bytes()[0] != b' ' {
                    (1, stripped)
                } else {
                    // Two or more spaces, or just "# " — not a valid annotation position
                    continue;
                }
            } else {
                // No space after # — keyword immediately follows
                (0, after_hash)
            };

            if trimmed.is_empty() {
                continue;
            }

            // Check if any keyword matches (case-insensitive)
            for keyword in &keywords {
                let kw_upper = keyword.to_uppercase();

                // Check if the comment starts with this keyword (case-insensitive)
                if !trimmed
                    .get(..keyword.len())
                    .is_some_and(|s| s.eq_ignore_ascii_case(&kw_upper))
                {
                    continue;
                }

                // Ensure the keyword is at a word boundary (next char is not alphanumeric/underscore)
                if let Some(next_byte) = trimmed.as_bytes().get(keyword.len()) {
                    if next_byte.is_ascii_alphanumeric() || *next_byte == b'_' {
                        continue;
                    }
                }

                let keyword_text = &trimmed[..keyword.len()];
                let after_kw = &trimmed[keyword.len()..];

                // Parse the annotation parts matching RuboCop's AnnotationComment regex:
                // /^(# ?)(\bKEYWORD\b)(\s*:)?(\s+)?(\S+)?/i
                //
                // after_kw is everything after the keyword match.
                // Parse colon, space, note groups from after_kw.
                let mut rest = after_kw;

                // (\s*:)? — optional whitespace then colon
                let has_colon = {
                    let trimmed_rest = rest.trim_start();
                    if let Some(after_colon) = trimmed_rest.strip_prefix(':') {
                        rest = after_colon; // consume the colon
                        true
                    } else {
                        false
                    }
                };

                // (\s+)? — optional whitespace after keyword/colon
                let has_space = if !rest.is_empty() && rest.as_bytes()[0].is_ascii_whitespace() {
                    let trimmed = rest.trim_start();
                    rest = trimmed;
                    true
                } else {
                    false
                };

                // (\S+)? — first non-space word (note)
                let has_note = !rest.is_empty();

                // RuboCop's annotation? = keyword_appearance? && !just_keyword_of_sentence?
                // keyword_appearance? = keyword && (colon || space)
                // A bare keyword alone (e.g. `# TODO`) with nothing after it is NOT
                // considered an annotation — it needs at least a colon or space.
                if !has_colon && !has_space {
                    continue;
                }

                // Skip if this is just a keyword used in a sentence (e.g., "# Note that...")
                if just_keyword_of_sentence(keyword_text, has_colon, has_space, has_note) {
                    continue;
                }

                // RuboCop's correct? = keyword && space && note && keyword==UPPER &&
                //   (colon.nil? == !require_colon)
                let is_keyword_upper = keyword_text == kw_upper;
                let is_correct =
                    is_keyword_upper && has_space && has_note && (has_colon == require_colon);
                if is_correct {
                    continue;
                }

                // At this point, keyword_appearance is true (has_colon or has_space),
                // it's not just a keyword in a sentence, and it's not correctly formatted.
                let msg = if !has_note {
                    format!(
                        "Annotation comment, with keyword `{}`, is missing a note.",
                        kw_upper
                    )
                } else if require_colon {
                    format!(
                        "Annotation keywords like `{}` should be all upper case, followed by a colon, and a space, then a note describing the problem.",
                        kw_upper,
                    )
                } else {
                    format!(
                        "Annotation keywords like `{}` should be all upper case, followed by a space, then a note describing the problem.",
                        kw_upper,
                    )
                };

                // Column of the keyword within the comment
                let (_, comment_col) = source.offset_to_line_col(comment_start_offset);
                let kw_col = comment_col + 1 + margin_len;
                diagnostics.push(self.diagnostic(source, comment_line, kw_col, msg));
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CommentAnnotation, "cops/style/comment_annotation");

    #[test]
    fn crlf_bare_keyword_no_offense() {
        // CRLF line endings: Prism includes trailing \r in the comment text,
        // which must not be mistaken for meaningful whitespace after a bare keyword.
        crate::testutil::assert_cop_no_offenses_full(
            &CommentAnnotation,
            b"require 'foo' # HACK\r\n",
        );
    }
}
