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

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for (i, line) in source.lines().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Find a comment `#` on this line
            // We need to find the # that starts a comment, not one inside a string
            // Simple heuristic: find last `#` that has a space or is at start of comment portion
            let comment_pos = match find_comment_start(line_str) {
                Some(pos) => pos,
                None => continue,
            };

            let comment_text = &line_str[comment_pos..];

            // Check if this is an allowed comment type
            let after_hash = &comment_text[1..]; // skip the '#'
            let after_hash_trimmed = after_hash.trim_start();

            // Allow :nodoc: and :yields: (RDoc annotations)
            if after_hash_trimmed.starts_with(":nodoc:") || after_hash_trimmed.starts_with(":yields:") {
                continue;
            }

            // Allow rubocop directives (rubocop:disable, rubocop:todo, etc.)
            // Match: rubocop:, rubocop :
            if after_hash_trimmed.starts_with("rubocop:") || after_hash_trimmed.starts_with("rubocop :") {
                continue;
            }

            // Allow RBS::Inline `#:` annotations on def and end lines
            if after_hash.starts_with(':') && after_hash.get(1..2).is_some_and(|c| c != "[") {
                // #: is an RBS type annotation, allowed on def and end
                let before_comment = line_str[..comment_pos].trim();
                if starts_with_keyword(before_comment, "def") || starts_with_keyword(before_comment, "end") {
                    continue;
                }
            }

            // Allow steep:ignore annotations
            if after_hash_trimmed.starts_with("steep:ignore ") || after_hash_trimmed == "steep:ignore" {
                continue;
            }

            // Check for RBS::Inline generics annotation on class with superclass: `class X < Y #[String]`
            if after_hash.starts_with('[') && after_hash.ends_with(']') {
                let before_comment = line_str[..comment_pos].trim();
                if before_comment.contains('<') && starts_with_keyword(before_comment, "class") {
                    continue;
                }
            }

            // Check if the code before the comment starts with a keyword
            let before_comment = line_str[..comment_pos].trim();
            if before_comment.is_empty() {
                continue;
            }

            for &keyword in KEYWORDS {
                if starts_with_keyword(before_comment, keyword) {
                    let (line_num, _) = (i + 1, 0);
                    let col = comment_pos;
                    diagnostics.push(self.diagnostic(
                        source,
                        line_num,
                        col,
                        format!(
                            "Do not place comments on the same line as the `{}` keyword.",
                            keyword
                        ),
                    ));
                    break;
                }
            }
        }

        diagnostics
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

/// Find the byte offset of the comment `#` in a line, skipping `#` inside strings.
/// This is a simple heuristic: we look for `#` that is preceded by whitespace or
/// is at the start of the line, and not inside a string literal.
fn find_comment_start(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'\\' if in_double_quote || in_single_quote => {
                i += 2; // skip escaped char
                continue;
            }
            b'\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            b'"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            b'#' if !in_single_quote && !in_double_quote => {
                return Some(i);
            }
            _ => {}
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CommentedKeyword, "cops/style/commented_keyword");
}
