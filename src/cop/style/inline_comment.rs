use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct InlineComment;

impl Cop for InlineComment {
    fn name(&self) -> &'static str {
        "Style/InlineComment"
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let lines = source.lines();

        for (i, line_bytes) in lines.enumerate() {
            let line = match std::str::from_utf8(line_bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let trimmed = line.trim_start();

            // Skip standalone comment lines (line starts with #)
            if trimmed.starts_with('#') {
                continue;
            }

            // Skip empty lines
            if trimmed.is_empty() {
                continue;
            }

            // Look for trailing inline comments
            // Must find # that's not inside a string
            if let Some(comment_pos) = Self::find_inline_comment(line) {
                let comment = &line[comment_pos..];

                // Skip rubocop directives
                let after_hash = comment.trim_start_matches('#').trim_start();
                if after_hash.starts_with("rubocop:") {
                    continue;
                }

                // Skip turbocop directives
                if after_hash.starts_with("turbocop-") {
                    continue;
                }

                let line_num = i + 1;
                let col = comment_pos;
                diagnostics.push(self.diagnostic(
                    source,
                    line_num,
                    col,
                    "Avoid trailing inline comments.".to_string(),
                ));
            }
        }

    }
}

impl InlineComment {
    fn find_inline_comment(line: &str) -> Option<usize> {
        let bytes = line.as_bytes();
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut i = 0;

        while i < bytes.len() {
            match bytes[i] {
                b'\\' if in_double_quote || in_single_quote => {
                    i += 2;
                    continue;
                }
                b'\'' if !in_double_quote => {
                    in_single_quote = !in_single_quote;
                }
                b'"' if !in_single_quote => {
                    in_double_quote = !in_double_quote;
                }
                b'#' if !in_single_quote && !in_double_quote => {
                    // Check that there's code before this
                    let before = &line[..i];
                    if !before.trim().is_empty() {
                        return Some(i);
                    }
                }
                _ => {}
            }
            i += 1;
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InlineComment, "cops/style/inline_comment");
}
