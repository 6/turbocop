use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct LineEndStringConcatenationIndentation;

impl Cop for LineEndStringConcatenationIndentation {
    fn name(&self) -> &'static str {
        "Layout/LineEndStringConcatenationIndentation"
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyle", "aligned");
        let _indent_width = config.get_usize("IndentationWidth", 2);

        let lines: Vec<&[u8]> = source.lines().collect();
        let mut diagnostics = Vec::new();

        for i in 0..lines.len().saturating_sub(1) {
            let line = lines[i];

            // Strip trailing \r
            let trimmed_end = line
                .iter()
                .rposition(|&b| b != b'\r')
                .map(|p| &line[..=p])
                .unwrap_or(line);

            if !trimmed_end.ends_with(b"\\") {
                continue;
            }

            let before_backslash = &trimmed_end[..trimmed_end.len() - 1];
            let before_trimmed = before_backslash.iter()
                .rposition(|&b| b != b' ' && b != b'\t')
                .map(|p| &before_backslash[..=p])
                .unwrap_or(before_backslash);

            // Check if the line ends with a string (before backslash)
            let is_string = before_trimmed.ends_with(b"'") || before_trimmed.ends_with(b"\"");
            if !is_string {
                continue;
            }

            // Current line indentation
            let current_indent = line.iter().take_while(|&&b| b == b' ').count();

            // Next line indentation
            let next_line = lines[i + 1];
            let next_indent = next_line.iter().take_while(|&&b| b == b' ').count();

            match style {
                "aligned" => {
                    if next_indent != current_indent {
                        let line_num = i + 2; // 1-based, points to next line
                        diagnostics.push(self.diagnostic(
                            source,
                            line_num,
                            next_indent,
                            "Align parts of a string concatenation.".to_string(),
                        ));
                    }
                }
                "indented" => {
                    let width = config.get_usize("IndentationWidth", 2);
                    let expected = current_indent + width;
                    if next_indent != expected {
                        let line_num = i + 2;
                        diagnostics.push(self.diagnostic(
                            source,
                            line_num,
                            next_indent,
                            format!(
                                "Indent the first part of a string concatenation by {} spaces.",
                                width
                            ),
                        ));
                    }
                }
                _ => {}
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        LineEndStringConcatenationIndentation,
        "cops/layout/line_end_string_concatenation_indentation"
    );
}
