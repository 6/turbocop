use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct LineEndConcatenation;

impl Cop for LineEndConcatenation {
    fn name(&self) -> &'static str {
        "Style/LineEndConcatenation"
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let lines = source.lines();

        for (i, line) in lines.iter().enumerate() {
            if i + 1 >= lines.len() {
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

            // Check that the next line starts with a string literal
            let next_line = lines[i + 1].trim_start();
            let next_starts_with_string = next_line.starts_with('"')
                || next_line.starts_with('\'');

            if !next_starts_with_string {
                continue;
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
                format!("Use `\\` instead of `{}` to concatenate multiline strings.", op),
            ));
        }

        diagnostics
    }
}

impl LineEndConcatenation {
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
