use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct CommentedKeyword;

/// Keywords that should not have comments on the same line.
const KEYWORDS: &[&str] = &["begin", "class", "def", "end", "module"];

impl Cop for CommentedKeyword {
    fn name(&self) -> &'static str {
        "Style/CommentedKeyword"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let bytes = source.as_bytes();

        // Iterate over parser-recognized comments only.
        // This avoids false positives from `#` inside heredocs, strings, etc.
        for comment in parse_result.comments() {
            let loc = comment.location();
            let comment_start = loc.start_offset();
            let comment_end = loc.end_offset();
            let comment_text = &bytes[comment_start..comment_end];

            // Must start with #
            if comment_text.is_empty() || comment_text[0] != b'#' {
                continue;
            }

            let after_hash = &comment_text[1..]; // skip the '#'
            let after_hash_str = match std::str::from_utf8(after_hash) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let after_hash_trimmed = after_hash_str.trim_start();

            // Allow :nodoc: and :yields: (RDoc annotations)
            if after_hash_trimmed.starts_with(":nodoc:")
                || after_hash_trimmed.starts_with(":yields:")
            {
                continue;
            }

            // Allow rubocop directives (rubocop:disable, rubocop:todo, etc.)
            if after_hash_trimmed.starts_with("rubocop:")
                || after_hash_trimmed.starts_with("rubocop :")
            {
                continue;
            }

            // Allow steep:ignore annotations
            if after_hash_trimmed.starts_with("steep:ignore ")
                || after_hash_trimmed == "steep:ignore"
            {
                continue;
            }

            // Get the source line containing this comment
            let (line_num, comment_col) = source.offset_to_line_col(comment_start);

            // Get the full source line text before the comment
            let line_start_offset = comment_start - comment_col;
            let before_comment = match std::str::from_utf8(&bytes[line_start_offset..comment_start])
            {
                Ok(s) => s.trim(),
                Err(_) => continue,
            };

            // Skip if the comment is the only thing on the line (full-line comment)
            if before_comment.is_empty() {
                continue;
            }

            // Allow RBS::Inline `#:` annotations on def and end lines
            if after_hash_str.starts_with(':')
                && after_hash_str.get(1..2).is_some_and(|c| c != "[")
                && (starts_with_keyword(before_comment, "def")
                    || starts_with_keyword(before_comment, "end"))
            {
                continue;
            }

            // Check for RBS::Inline generics annotation on class with superclass: `class X < Y #[String]`
            if after_hash_str.starts_with('[')
                && after_hash_str.ends_with(']')
                && before_comment.contains('<')
                && starts_with_keyword(before_comment, "class")
            {
                continue;
            }

            // Check if the code before the comment starts with a keyword
            for &keyword in KEYWORDS {
                if starts_with_keyword(before_comment, keyword) {
                    diagnostics.push(self.diagnostic(
                        source,
                        line_num,
                        comment_col,
                        format!(
                            "Do not place comments on the same line as the `{}` keyword.",
                            keyword
                        ),
                    ));
                    break;
                }
            }
        }
    }
}

/// Check if a trimmed line starts with the given keyword as a keyword token.
/// For example, `starts_with_keyword("def x", "def")` returns true,
/// but `starts_with_keyword("defined?(x)", "def")` returns false.
fn starts_with_keyword(trimmed: &str, keyword: &str) -> bool {
    if !trimmed.starts_with(keyword) {
        return false;
    }
    let after = &trimmed[keyword.len()..];
    // After keyword must be empty or whitespace.
    // RuboCop uses /^\s*keyword\s/ — only whitespace after the keyword counts.
    // `.` after `end` means method chain (e.g., `end.to ...`), not keyword usage.
    // `;` and `(` are handled transitively: `def x; end # comment` matches on `def`,
    // and `def x(a, b) # comment` also matches `def` followed by space.
    after.is_empty() || after.starts_with(' ')
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CommentedKeyword, "cops/style/commented_keyword");
}
