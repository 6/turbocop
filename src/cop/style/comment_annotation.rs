use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct CommentAnnotation;

const DEFAULT_KEYWORDS: &[&str] = &["TODO", "FIXME", "OPTIMIZE", "HACK", "REVIEW", "NOTE"];

impl Cop for CommentAnnotation {
    fn name(&self) -> &'static str {
        "Style/CommentAnnotation"
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig) -> Vec<Diagnostic> {
        let require_colon = config.get_bool("RequireColon", true);
        let keywords_opt = config.get_string_array("Keywords");
        let keywords: Vec<String> = keywords_opt.unwrap_or_else(|| {
            DEFAULT_KEYWORDS.iter().map(|s| s.to_string()).collect()
        });

        let mut diagnostics = Vec::new();

        for (i, line) in source.lines().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Find # comment start
            let comment_start = match line_str.find('#') {
                Some(pos) => pos,
                None => continue,
            };

            let after_hash = &line_str[comment_start + 1..];
            // Strip leading whitespace after #
            let trimmed = after_hash.trim_start();
            if trimmed.is_empty() {
                continue;
            }

            // Check if any keyword matches (case-insensitive)
            for keyword in &keywords {
                let kw_upper = keyword.to_uppercase();

                // Check if the comment starts with this keyword (case-insensitive)
                if !trimmed.get(..keyword.len()).is_some_and(|s| s.eq_ignore_ascii_case(&kw_upper)) {
                    continue;
                }

                let after_kw = &trimmed[keyword.len()..];

                // If already correctly formatted, skip
                if require_colon {
                    // Correct: "KEYWORD: note"
                    if after_kw.starts_with(": ") && trimmed.starts_with(&kw_upper[..]) {
                        continue;
                    }
                } else {
                    // Correct: "KEYWORD note" (no colon)
                    if after_kw.starts_with(' ') && !after_kw.starts_with(": ") && trimmed.starts_with(&kw_upper[..]) {
                        continue;
                    }
                }

                // Check if this is actually an annotation keyword (followed by colon, space, or end-of-line)
                if after_kw.is_empty()
                    || after_kw.starts_with(':')
                    || after_kw.starts_with(' ')
                {
                    // It's an annotation, but not correctly formatted
                    let msg = if after_kw.is_empty() {
                        format!("Annotation comment, with keyword `{}`, is missing a note.", kw_upper)
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

                    // Find the byte offset of the keyword in the line
                    let kw_start = comment_start + 1 + (after_hash.len() - trimmed.len());
                    diagnostics.push(self.diagnostic(
                        source,
                        i + 1,
                        kw_start,
                        msg,
                    ));
                    break;
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CommentAnnotation, "cops/style/comment_annotation");
}
