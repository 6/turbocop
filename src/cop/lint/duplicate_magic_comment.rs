use std::collections::HashSet;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DuplicateMagicComment;

impl Cop for DuplicateMagicComment {
    fn name(&self) -> &'static str {
        "Lint/DuplicateMagicComment"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig, diagnostics: &mut Vec<Diagnostic>) {
        let mut seen_keys = HashSet::new();

        for (i, line) in source.lines().enumerate() {
            let trimmed = line
                .iter()
                .position(|&b| b != b' ' && b != b'\t')
                .map(|start| &line[start..])
                .unwrap_or(&[]);

            // Only check leading comments (magic comments must be at top of file)
            if trimmed.is_empty() {
                continue;
            }

            // Shebang line
            if trimmed.starts_with(b"#!") {
                continue;
            }

            if !trimmed.starts_with(b"#") {
                break; // Non-comment line reached, stop scanning
            }

            // Check for magic comment pattern: # key: value or # -*- key: value -*-
            let comment = &trimmed[1..]; // skip #
            let comment = comment
                .iter()
                .position(|&b| b != b' ' && b != b'\t')
                .map(|start| &comment[start..])
                .unwrap_or(&[]);

            // Emacs-style: -*- coding: utf-8 -*-
            let comment = if comment.starts_with(b"-*-") {
                let inner = &comment[3..];
                if let Some(end) = inner.windows(3).position(|w| w == b"-*-") {
                    &inner[..end]
                } else {
                    inner
                }
            } else {
                comment
            };

            // Extract key from key: value pattern
            if let Some(colon_pos) = comment.iter().position(|&b| b == b':') {
                let key = &comment[..colon_pos];
                let key = key
                    .iter()
                    .rev()
                    .position(|&b| b != b' ' && b != b'\t')
                    .map(|end| &key[..key.len() - end])
                    .unwrap_or(key);

                // Valid magic comment keys
                let key_lower: Vec<u8> = key.iter().map(|b| b.to_ascii_lowercase()).collect();
                let is_magic = matches!(
                    key_lower.as_slice(),
                    b"frozen_string_literal"
                        | b"frozen-string-literal"
                        | b"encoding"
                        | b"coding"
                        | b"warn_indent"
                        | b"warn-indent"
                        | b"shareable_constant_value"
                        | b"shareable-constant-value"
                        | b"typed"
                );

                if is_magic && !seen_keys.insert(key_lower) {
                    diagnostics.push(self.diagnostic(
                        source,
                        i + 1,
                        0,
                        "Duplicate magic comment detected.".to_string(),
                    ));
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateMagicComment, "cops/lint/duplicate_magic_comment");
}
