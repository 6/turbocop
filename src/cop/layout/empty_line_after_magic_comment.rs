use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyLineAfterMagicComment;

const MAGIC_COMMENT_PATTERNS: &[&str] = &[
    "frozen_string_literal:",
    "encoding:",
    "coding:",
    "warn_indent:",
    "shareable_constant_value:",
    "typed:",
];

fn is_magic_comment(line: &[u8]) -> bool {
    let trimmed = line.iter().position(|&b| b != b' ' && b != b'\t');
    let trimmed = match trimmed {
        Some(t) => &line[t..],
        None => return false,
    };
    if !trimmed.starts_with(b"#") {
        return false;
    }
    let after_hash = &trimmed[1..];
    let after_hash = if after_hash.starts_with(b" ") {
        &after_hash[1..]
    } else {
        after_hash
    };
    let line_str = std::str::from_utf8(after_hash).unwrap_or("");

    // Check direct magic comment patterns: `# frozen_string_literal: true`
    if MAGIC_COMMENT_PATTERNS.iter().any(|p| line_str.starts_with(p)) {
        return true;
    }

    // Check Emacs-style magic comments: `# -*- coding: utf-8 -*-`
    if line_str.starts_with("-*-") {
        let inner = line_str.trim_start_matches("-*-").trim();
        if MAGIC_COMMENT_PATTERNS.iter().any(|p| inner.starts_with(p)) {
            return true;
        }
        // Also check for patterns within the -*- ... -*- wrapper
        if let Some(end) = inner.find("-*-") {
            let content = inner[..end].trim();
            if MAGIC_COMMENT_PATTERNS.iter().any(|p| content.starts_with(p)) {
                return true;
            }
        }
    }

    false
}

impl Cop for EmptyLineAfterMagicComment {
    fn name(&self) -> &'static str {
        "Layout/EmptyLineAfterMagicComment"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig, diagnostics: &mut Vec<Diagnostic>, mut corrections: Option<&mut Vec<crate::correction::Correction>>) {
        let lines: Vec<&[u8]> = source.lines().collect();
        let mut last_magic_line = None;

        for (i, line) in lines.iter().enumerate() {
            if is_magic_comment(line) {
                last_magic_line = Some(i);
            } else {
                // Stop at first non-magic-comment, non-blank line
                let is_blank = line.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r');
                if !is_blank {
                    break;
                }
                // Blank lines between magic comments and code are fine
                if last_magic_line.is_some() {
                    break;
                }
            }
        }

        let last_magic_idx = match last_magic_line {
            Some(idx) => idx,
            None => return,
        };

        // Check if the line after the last magic comment is blank
        let next_idx = last_magic_idx + 1;
        if next_idx >= lines.len() {
            return;
        }

        let next_line = lines[next_idx];
        let is_blank = next_line.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r');

        if !is_blank {
            let mut diag = self.diagnostic(
                source,
                next_idx + 1, // 1-indexed
                0,
                "Add an empty line after magic comments.".to_string(),
            );
            if let Some(ref mut corr) = corrections {
                if let Some(offset) = source.line_col_to_offset(next_idx + 1, 0) {
                    corr.push(crate::correction::Correction {
                        start: offset,
                        end: offset,
                        replacement: "\n".to_string(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diag.corrected = true;
                }
            }
            diagnostics.push(diag);
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_scenario_fixture_tests!(
        EmptyLineAfterMagicComment,
        "cops/layout/empty_line_after_magic_comment",
        frozen_string = "frozen_string.rb",
        encoding = "encoding.rb",
        multiple_magic = "multiple_magic.rb",
        emacs_coding = "emacs_coding.rb",
    );

    #[test]
    fn autocorrect_insert_blank_after_frozen_string() {
        let input = b"# frozen_string_literal: true\nx = 1\n";
        let (_diags, corrections) = crate::testutil::run_cop_autocorrect(&EmptyLineAfterMagicComment, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"# frozen_string_literal: true\n\nx = 1\n");
    }

    #[test]
    fn autocorrect_insert_blank_after_multiple_magic() {
        let input = b"# frozen_string_literal: true\n# encoding: utf-8\nx = 1\n";
        let (_diags, corrections) = crate::testutil::run_cop_autocorrect(&EmptyLineAfterMagicComment, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"# frozen_string_literal: true\n# encoding: utf-8\n\nx = 1\n");
    }
}
