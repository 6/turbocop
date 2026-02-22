use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct LineEndConcatenation;

impl Cop for LineEndConcatenation {
    fn name(&self) -> &'static str {
        "Style/LineEndConcatenation"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let lines: Vec<&str> = source
            .lines()
            .filter_map(|l| std::str::from_utf8(l).ok())
            .collect();

        // Compute byte offsets where each line starts
        let mut line_offsets = Vec::with_capacity(lines.len());
        let mut offset = 0usize;
        for line in &lines {
            line_offsets.push(offset);
            offset += line.len() + 1; // +1 for the newline
        }

        for (i, line) in lines.iter().enumerate() {
            if i + 1 >= lines.len() {
                continue;
            }

            // Skip lines that are inside a heredoc body
            if code_map.is_heredoc(line_offsets[i]) {
                continue;
            }

            let trimmed = line.trim_end();

            // Check for string concatenation at end of line: "str" + or "str" <<
            let (op, op_len) = if trimmed.ends_with(" +") || trimmed.ends_with("\t+") {
                ("+", 1)
            } else if trimmed.ends_with(" <<") || trimmed.ends_with("\t<<") {
                ("<<", 2)
            } else {
                continue;
            };

            // Check that the operator is preceded by a string
            let before_op = &trimmed[..trimmed.len() - op_len].trim_end();

            // Skip if there's a comment after the operator
            if before_op.contains('#') && !before_op.ends_with('"') && !before_op.ends_with('\'') {
                continue;
            }

            // The part before the operator should end with a string literal
            let ends_with_string = before_op.ends_with('"') || before_op.ends_with('\'');

            if !ends_with_string {
                continue;
            }

            // Check that the next line starts with a string literal and is purely
            // a string (not a string followed by a method call like `" " * 3`
            // or `'gniht'.reverse`).
            let next_line = lines[i + 1].trim_start();
            let next_starts_with_string = next_line.starts_with('"') || next_line.starts_with('\'');

            if !next_starts_with_string {
                continue;
            }

            // Find the end of the string on the next line and check what follows
            let next_trimmed = next_line.trim_end();
            if let Some(after_string) = Self::after_string_literal(next_trimmed) {
                let rest = after_string.trim();
                // OK if rest is empty, or starts with `+` (continued concat) or `\` or `<<`
                if !rest.is_empty()
                    && !rest.starts_with('+')
                    && !rest.starts_with("<<")
                    && !rest.starts_with('\\')
                    && !rest.starts_with('#')
                // inline comment
                {
                    continue;
                }
            }

            // If the next line is a comment line, skip
            if i + 2 <= lines.len() {
                let next_trimmed_check = lines[i + 1].trim_start();
                if next_trimmed_check.starts_with('#') {
                    continue;
                }
            }

            // Check there's no comment on the line
            let has_comment = Self::has_inline_comment(trimmed);
            if has_comment {
                continue;
            }

            // Check it's not a % literal
            if before_op.contains("%(") || before_op.contains("%q(") || before_op.contains("%Q(") {
                continue;
            }

            let col = trimmed.len() - op_len;
            let line_num = i + 1;

            diagnostics.push(self.diagnostic(
                source,
                line_num,
                col,
                format!(
                    "Use `\\` instead of `{}` to concatenate multiline strings.",
                    op
                ),
            ));
        }
    }
}

impl LineEndConcatenation {
    /// Given a line starting with a string literal, return the rest of the line after the string.
    fn after_string_literal(line: &str) -> Option<&str> {
        let bytes = line.as_bytes();
        if bytes.is_empty() {
            return None;
        }
        let quote = bytes[0];
        if quote != b'\'' && quote != b'"' {
            return None;
        }
        let mut i = 1;
        while i < bytes.len() {
            if bytes[i] == b'\\' {
                i += 2; // skip escaped character
                continue;
            }
            if bytes[i] == quote {
                return Some(&line[i + 1..]);
            }
            i += 1;
        }
        None // unterminated string
    }

    fn has_inline_comment(line: &str) -> bool {
        let bytes = line.as_bytes();
        let mut in_single = false;
        let mut in_double = false;
        let mut i = 0;

        while i < bytes.len() {
            match bytes[i] {
                b'\\' if in_double || in_single => {
                    i += 2;
                    continue;
                }
                b'\'' if !in_double => {
                    in_single = !in_single;
                }
                b'"' if !in_single => {
                    in_double = !in_double;
                }
                b'#' if !in_single && !in_double => {
                    return true;
                }
                _ => {}
            }
            i += 1;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(LineEndConcatenation, "cops/style/line_end_concatenation");
}
