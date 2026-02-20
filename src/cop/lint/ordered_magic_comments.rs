use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct OrderedMagicComments;

impl Cop for OrderedMagicComments {
    fn name(&self) -> &'static str {
        "Lint/OrderedMagicComments"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig, diagnostics: &mut Vec<Diagnostic>, _corrections: Option<&mut Vec<crate::correction::Correction>>) {
        let mut encoding_line: Option<usize> = None;
        let mut frozen_string_line: Option<usize> = None;

        for (i, line) in source.lines().enumerate() {
            let line_num = i + 1; // 1-indexed

            let trimmed = line
                .iter()
                .position(|&b| b != b' ' && b != b'\t')
                .map(|start| &line[start..])
                .unwrap_or(&[]);

            if trimmed.is_empty() {
                continue;
            }

            // Skip shebang
            if trimmed.starts_with(b"#!") {
                continue;
            }

            // Stop at first non-comment line
            if !trimmed.starts_with(b"#") {
                break;
            }

            let comment = &trimmed[1..]; // skip #
            let comment_trimmed = comment
                .iter()
                .position(|&b| b != b' ' && b != b'\t')
                .map(|start| &comment[start..])
                .unwrap_or(&[]);

            // Handle emacs-style: -*- coding: utf-8 -*-
            let comment_lower: Vec<u8> = comment_trimmed.iter().map(|b| b.to_ascii_lowercase()).collect();

            if is_encoding_comment(&comment_lower) && encoding_line.is_none() {
                encoding_line = Some(line_num);
            } else if is_frozen_string_comment(&comment_lower) && frozen_string_line.is_none() {
                frozen_string_line = Some(line_num);
            }

            if encoding_line.is_some() && frozen_string_line.is_some() {
                break;
            }
        }

        if let (Some(enc_line), Some(fsl_line)) = (encoding_line, frozen_string_line) {
            if enc_line > fsl_line {
                // Encoding comment appears after frozen_string_literal
                diagnostics.push(self.diagnostic(
                    source,
                    enc_line,
                    0,
                    "The encoding magic comment should precede all other magic comments."
                        .to_string(),
                ));
            }
        }

    }
}

fn is_encoding_comment(lower: &[u8]) -> bool {
    // Match "encoding:" or "coding:" patterns (also emacs-style)
    lower.windows(9).any(|w| w == b"encoding:")
        || lower.windows(7).any(|w| w == b"coding:")
}

fn is_frozen_string_comment(lower: &[u8]) -> bool {
    lower.windows(22).any(|w| w == b"frozen_string_literal:")
        || lower.windows(22).any(|w| w == b"frozen-string-literal:")
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_scenario_fixture_tests!(
        OrderedMagicComments, "cops/lint/ordered_magic_comments",
        basic = "basic.rb",
        with_coding = "with_coding.rb",
        with_shebang = "with_shebang.rb",
    );
}
