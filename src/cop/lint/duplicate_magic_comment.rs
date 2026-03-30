use std::collections::HashSet;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for duplicated magic comments at the top of a file.
///
/// Fixed: Emacs-style encoding comments (`# -*- encoding : utf-8 -*-`) were not
/// recognized because leading whitespace in the inner content was not stripped,
/// causing key extraction to fail. Also, `coding` and `encoding` were tracked
/// as separate keys in the duplicate-detection set, so `# -*- encoding : utf-8 -*-`
/// followed by `# coding: utf-8` was not flagged. Both keys are now normalized to
/// `encoding` (along with hyphen-variants of other keys).
///
/// Additionally, RuboCop's `SimpleComment#encoding` regex requires a space after
/// the colon for encoding/coding keys (e.g. `# coding: utf-8` matches but
/// `#coding:utf-8` does not). This is now matched to avoid false positives.
pub struct DuplicateMagicComment;

impl Cop for DuplicateMagicComment {
    fn name(&self) -> &'static str {
        "Lint/DuplicateMagicComment"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut seen_keys = HashSet::new();
        let total_len = source.as_bytes().len();
        let mut byte_offset: usize = 0;

        for (i, line) in source.lines().enumerate() {
            let line_len = line.len() + 1; // +1 for newline
            let trimmed = line
                .iter()
                .position(|&b| b != b' ' && b != b'\t')
                .map(|start| &line[start..])
                .unwrap_or(&[]);

            // Only check leading comments (magic comments must be at top of file)
            if trimmed.is_empty() {
                byte_offset += line_len;
                continue;
            }

            // Shebang line
            if trimmed.starts_with(b"#!") {
                byte_offset += line_len;
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
            let is_emacs = comment.starts_with(b"-*-");
            let comment = if is_emacs {
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
                // Trim leading whitespace (needed for Emacs-style inner content)
                let key = key
                    .iter()
                    .position(|&b| b != b' ' && b != b'\t')
                    .map(|start| &key[start..])
                    .unwrap_or(key);
                // Trim trailing whitespace
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

                // RuboCop's SimpleComment#encoding regex requires ": " (colon
                // then space) for encoding/coding keys. Skip if no space follows
                // the colon in non-Emacs comments so we don't flag e.g. `#coding:utf-8`.
                if !is_emacs
                    && matches!(key_lower.as_slice(), b"encoding" | b"coding")
                    && colon_pos + 1 < comment.len()
                    && comment[colon_pos + 1] != b' '
                    && comment[colon_pos + 1] != b'\t'
                {
                    byte_offset += line_len;
                    continue;
                }

                // Normalize aliases so duplicates across variant names are detected.
                // E.g. `# encoding: utf-8` followed by `# coding: utf-8`.
                let canonical = if is_magic {
                    match key_lower.as_slice() {
                        b"coding" => b"encoding".to_vec(),
                        b"frozen-string-literal" => b"frozen_string_literal".to_vec(),
                        b"warn-indent" => b"warn_indent".to_vec(),
                        b"shareable-constant-value" => b"shareable_constant_value".to_vec(),
                        _ => key_lower,
                    }
                } else {
                    key_lower
                };

                if is_magic && !seen_keys.insert(canonical) {
                    let mut diag = self.diagnostic(
                        source,
                        i + 1,
                        0,
                        "Duplicate magic comment detected.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        let end = std::cmp::min(byte_offset + line_len, total_len);
                        corr.push(crate::correction::Correction {
                            start: byte_offset,
                            end,
                            replacement: String::new(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
            }

            byte_offset += line_len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateMagicComment, "cops/lint/duplicate_magic_comment");
    crate::cop_autocorrect_fixture_tests!(
        DuplicateMagicComment,
        "cops/lint/duplicate_magic_comment"
    );

    #[test]
    fn autocorrect_remove_duplicate() {
        let input = b"# frozen_string_literal: true\n# frozen_string_literal: true\nx = 1\n";
        let (_diags, corrections) =
            crate::testutil::run_cop_autocorrect(&DuplicateMagicComment, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"# frozen_string_literal: true\nx = 1\n");
    }
}
