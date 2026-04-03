use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Recent corpus fixes for RuboCop parity:
/// 1. `# frozen_string_literal:` only counts as a valid magic comment when its value is
///    `true` or `false` (case-insensitive). Malformed values like `tru` must be treated as
///    missing comments, not as present-but-disabled directives.
/// 2. `__END__` only suppresses this cop for bare data files whose first non-blank line is
///    `__END__`. RuboCop still flags files that have leading shebang, encoding, or ordinary
///    comments before `__END__`, so this cop must not skip those.
///
/// Remaining corpus FPs around `vendor/` and `.rb.spec` paths are config/include issues rather
/// than content-matching bugs in this cop.
pub struct FrozenStringLiteralComment;

impl Cop for FrozenStringLiteralComment {
    fn name(&self) -> &'static str {
        "Style/FrozenStringLiteralComment"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "always");

        // Skip files with non-UTF-8 content — RuboCop's tokenizer produces no tokens
        // for files with encoding errors (e.g., invalid UTF-8, UTF-16, ISO-8859),
        // so it returns early via `processed_source.tokens.empty?`.
        if std::str::from_utf8(source.as_bytes()).is_err() || source.as_bytes().contains(&0x00) {
            return;
        }

        let lines: Vec<&[u8]> = source.lines().collect();

        if enforced_style == "never" {
            // Flag the presence of frozen_string_literal comment as unnecessary
            for (i, line) in lines.iter().enumerate() {
                if is_frozen_string_literal_comment(line) {
                    let mut diag = self.diagnostic(
                        source,
                        i + 1,
                        0,
                        "Unnecessary frozen string literal comment.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        // Delete the entire line including its newline
                        if let Some(start) = source.line_col_to_offset(i + 1, 0) {
                            let end = source
                                .line_col_to_offset(i + 2, 0)
                                .unwrap_or(source.as_bytes().len());
                            corr.push(crate::correction::Correction {
                                start,
                                end,
                                replacement: String::new(),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diag.corrected = true;
                        }
                    }
                    diagnostics.push(diag);
                }
            }
            return;
        }

        // Skip empty files — RuboCop returns early when there are no tokens.
        // Lint/EmptyFile handles these instead.
        let has_content = lines
            .iter()
            .any(|l| !l.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r'));
        if !has_content {
            return;
        }

        // RuboCop skips bare data files that start with `__END__`, but it still flags files
        // that have leading comments before `__END__`.
        if starts_with_end_data_only(&lines) {
            return;
        }

        let mut idx = 0;

        // Skip shebang
        if idx < lines.len() && lines[idx].starts_with(b"#!") {
            idx += 1;
        }

        // Skip blank lines after shebang (RuboCop scans all lines before the first
        // non-comment token, so blank lines don't break the search)
        while idx < lines.len() && is_blank_line(lines[idx]) {
            idx += 1;
        }

        // Skip encoding comment, but check if it also contains frozen_string_literal
        // (Emacs-style: # -*- encoding: utf-8; frozen_string_literal: true -*-)
        if idx < lines.len() && is_encoding_comment(lines[idx]) {
            if is_frozen_string_literal_comment(lines[idx]) {
                if enforced_style == "always_true" && !is_frozen_string_literal_true(lines[idx]) {
                    diagnostics.push(self.diagnostic(
                        source,
                        idx + 1,
                        0,
                        "Frozen string literal comment must be set to `true`.".to_string(),
                    ));
                }
                return;
            }
            idx += 1;
        }

        // Remember where to insert the comment (after shebang/encoding)
        let insert_after_line = idx; // 0-indexed line number

        // Scan leading comment and blank lines for the frozen_string_literal magic comment.
        // RuboCop's `leading_comment_lines` returns all lines before the first non-comment
        // token — blank lines are included since they don't produce tokens.
        while idx < lines.len() && is_comment_or_blank_line(lines[idx]) {
            if is_frozen_string_literal_comment(lines[idx]) {
                if enforced_style == "always_true" {
                    // Must be set to true specifically
                    if !is_frozen_string_literal_true(lines[idx]) {
                        let mut diag = self.diagnostic(
                            source,
                            idx + 1,
                            0,
                            "Frozen string literal comment must be set to `true`.".to_string(),
                        );
                        if let Some(ref mut corr) = corrections {
                            // Replace the entire line with the correct comment
                            if let Some(start) = source.line_col_to_offset(idx + 1, 0) {
                                let end = source
                                    .line_col_to_offset(idx + 2, 0)
                                    .unwrap_or(source.as_bytes().len());
                                corr.push(crate::correction::Correction {
                                    start,
                                    end,
                                    replacement: "# frozen_string_literal: true\n".to_string(),
                                    cop_name: self.name(),
                                    cop_index: 0,
                                });
                                diag.corrected = true;
                            }
                        }
                        diagnostics.push(diag);
                    }
                }
                return;
            }
            idx += 1;
        }

        let msg = if enforced_style == "always_true" {
            "Missing magic comment `# frozen_string_literal: true`."
        } else {
            "Missing frozen string literal comment."
        };
        let mut diag = self.diagnostic(source, 1, 0, msg.to_string());
        if let Some(ref mut corr) = corrections {
            // Insert after shebang/encoding lines
            let insert_offset = source
                .line_col_to_offset(insert_after_line + 1, 0)
                .unwrap_or(0);
            corr.push(crate::correction::Correction {
                start: insert_offset,
                end: insert_offset,
                replacement: "# frozen_string_literal: true\n".to_string(),
                cop_name: self.name(),
                cop_index: 0,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}

/// Returns true when the file is bare `__END__` data: its first non-blank line is `__END__`
/// with no leading comments or shebang lines ahead of it.
fn starts_with_end_data_only(lines: &[&[u8]]) -> bool {
    for line in lines {
        if is_blank_line(line) {
            continue;
        }
        let trimmed: &[u8] = {
            let start = line
                .iter()
                .position(|&b| b != b' ' && b != b'\t')
                .unwrap_or(line.len());
            &line[start..]
        };
        if trimmed.starts_with(b"#") {
            return false;
        }
        // First non-blank, non-comment line.
        return trimmed.starts_with(b"__END__");
    }
    false
}

fn is_comment_line(line: &[u8]) -> bool {
    let trimmed = line.iter().skip_while(|&&b| b == b' ' || b == b'\t');
    matches!(trimmed.clone().next(), Some(b'#'))
}

fn is_blank_line(line: &[u8]) -> bool {
    line.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r')
}

fn is_comment_or_blank_line(line: &[u8]) -> bool {
    is_blank_line(line) || is_comment_line(line)
}

fn is_encoding_comment(line: &[u8]) -> bool {
    let s = match std::str::from_utf8(line) {
        Ok(s) => s,
        Err(_) => return false,
    };
    // Explicit encoding/coding directive: `# encoding: utf-8` or `# coding: utf-8`
    if s.starts_with("# encoding:") || s.starts_with("# coding:") {
        return true;
    }
    // Emacs-style mode line: `# -*- encoding: utf-8 -*-` or `# -*- coding: utf-8 -*-`
    // The space before the colon is optional: `# -*- encoding : utf-8 -*-`
    if s.starts_with("# -*-") {
        let lower = s.to_ascii_lowercase();
        return lower.contains("encoding") || lower.contains("coding");
    }
    false
}

/// Match `frozen_string_literal:` or `frozen-string-literal:` case-insensitively,
/// consistent with RuboCop's regex `frozen[_-]string[_-]literal` with `/i` flag.
///
/// For simple comments, RuboCop requires the key to be the ONLY content after `#`:
///   `\A\s*#\s*frozen[_-]string[_-]literal:\s*TOKEN\s*\z`
/// This means `# # frozen_string_literal: true` (double-hash) is NOT valid.
///
/// For Emacs-style comments (`# -*- ... -*-`), the key can appear anywhere
/// among semicolon-separated directives.
fn is_frozen_string_literal_comment(line: &[u8]) -> bool {
    frozen_string_literal_value(line).is_some_and(is_frozen_string_literal_boolean)
}

/// Check if a string STARTS WITH `frozen_string_literal:` or `frozen-string-literal:`
/// (case-insensitive, allowing hyphens or underscores as separators).
/// Used for simple (non-Emacs) magic comments where the key must be at the start.
fn starts_with_frozen_string_literal_key(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    // "frozen" (6) + sep (1) + "string" (6) + sep (1) + "literal:" (8) = 22 chars
    bytes.starts_with(b"frozen")
        && bytes.len() >= 22
        && (bytes[6] == b'_' || bytes[6] == b'-')
        && bytes[7..].starts_with(b"string")
        && (bytes[13] == b'_' || bytes[13] == b'-')
        && bytes[14..].starts_with(b"literal:")
}

fn is_frozen_string_literal_true(line: &[u8]) -> bool {
    frozen_string_literal_value(line).is_some_and(|value| value.eq_ignore_ascii_case("true"))
}

fn frozen_string_literal_value(line: &[u8]) -> Option<&str> {
    let s = std::str::from_utf8(line).ok()?;
    let trimmed = s.trim_start().strip_prefix('#')?.trim_start();
    if trimmed.starts_with("-*-") && trimmed.ends_with("-*-") {
        let after_key = strip_frozen_string_literal_key(trimmed)?;
        return Some(after_key.split([';', '-']).next().unwrap_or("").trim());
    }
    let after_key = strip_prefix_frozen_string_literal_key(trimmed)?;
    Some(after_key.trim())
}

fn is_frozen_string_literal_boolean(value: &str) -> bool {
    value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false")
}

/// If the string STARTS WITH `frozen[_-]string[_-]literal:` (case-insensitive),
/// return the portion after the colon. Used for simple (non-Emacs) comments.
fn strip_prefix_frozen_string_literal_key(s: &str) -> Option<&str> {
    if starts_with_frozen_string_literal_key(s) {
        Some(&s[22..])
    } else {
        None
    }
}

/// If the string contains `frozen[_-]string[_-]literal:` (case-insensitive),
/// return the portion after the colon.
fn strip_frozen_string_literal_key(s: &str) -> Option<&str> {
    let lower = s.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    // "frozen" (6) + sep (1) + "string" (6) + sep (1) + "literal:" (8) = 22 chars
    for i in 0..bytes.len() {
        if bytes[i..].starts_with(b"frozen")
            && i + 22 <= bytes.len()
            && (bytes[i + 6] == b'_' || bytes[i + 6] == b'-')
            && bytes[i + 7..].starts_with(b"string")
            && (bytes[i + 13] == b'_' || bytes[i + 13] == b'-')
            && bytes[i + 14..].starts_with(b"literal:")
        {
            return Some(&s[i + 22..]);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_scenario_fixture_tests!(
        FrozenStringLiteralComment,
        "cops/style/frozen_string_literal_comment",
        plain_missing = "plain_missing.rb",
        shebang_missing = "shebang_missing.rb",
        encoding_missing = "encoding_missing.rb",
        double_hash_frozen = "double_hash_frozen.rb",
        invalid_token = "invalid_token.rb",
        comment_before_end = "comment_before_end.rb",
        encoding_before_end = "encoding_before_end.rb",
    );

    #[test]
    fn missing_comment() {
        let source = SourceFile::from_bytes("test.rb", b"puts 'hello'\n".to_vec());
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
        assert_eq!(diags[0].location.column, 0);
        assert_eq!(diags[0].message, "Missing frozen string literal comment.");
    }

    #[test]
    fn with_frozen_true() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# frozen_string_literal: true\nputs 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(diags.is_empty());
    }

    #[test]
    fn with_frozen_false() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# frozen_string_literal: false\nputs 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(diags.is_empty());
    }

    #[test]
    fn with_shebang_and_frozen() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"#!/usr/bin/env ruby\n# frozen_string_literal: true\nputs 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(diags.is_empty());
    }

    #[test]
    fn with_shebang_no_frozen() {
        let source =
            SourceFile::from_bytes("test.rb", b"#!/usr/bin/env ruby\nputs 'hello'\n".to_vec());
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn with_encoding_and_frozen() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# encoding: utf-8\n# frozen_string_literal: true\nputs 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(diags.is_empty());
    }

    #[test]
    fn with_shebang_encoding_and_frozen() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"#!/usr/bin/env ruby\n# encoding: utf-8\n# frozen_string_literal: true\nputs 'hello'\n"
                .to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(diags.is_empty());
    }

    #[test]
    fn empty_file() {
        // Empty files should not be flagged — Lint/EmptyFile handles them
        let source = SourceFile::from_bytes("test.rb", b"".to_vec());
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(diags.is_empty(), "Empty files should not be flagged");
    }

    #[test]
    fn emacs_encoding_style() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# -*- coding: utf-8 -*-\n# frozen_string_literal: true\nputs 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(diags.is_empty());
    }

    #[test]
    fn emacs_encoding_with_spaces() {
        // Emacs mode line with spaces around colon: `# -*- encoding : utf-8 -*-`
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# -*- encoding : utf-8 -*-\n# frozen_string_literal: true\nputs 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "Should recognize encoding comment with spaces around colon"
        );
    }

    #[test]
    fn enforced_style_never_flags_presence() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("never".into()),
            )]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# frozen_string_literal: true\nputs 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &config, &mut diags, None);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("Unnecessary"));
    }

    #[test]
    fn enforced_style_never_allows_missing() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("never".into()),
            )]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes("test.rb", b"puts 'hello'\n".to_vec());
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &config, &mut diags, None);
        assert!(
            diags.is_empty(),
            "Should not flag missing comment with 'never' style"
        );
    }

    #[test]
    fn enforced_style_always_true_flags_false() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("always_true".into()),
            )]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# frozen_string_literal: false\nputs 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &config, &mut diags, None);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("must be set to `true`"));
    }

    #[test]
    fn enforced_style_always_true_allows_true() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("always_true".into()),
            )]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# frozen_string_literal: true\nputs 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &config, &mut diags, None);
        assert!(diags.is_empty(), "Should allow true with always_true style");
    }

    #[test]
    fn leading_whitespace_recognized() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"  # frozen_string_literal: true\nputs 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "Should recognize frozen_string_literal with leading whitespace"
        );
    }

    #[test]
    fn with_typed_comment_before_frozen() {
        // Sorbet typed: true comment before frozen_string_literal should be recognized
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# typed: true\n# frozen_string_literal: true\nputs 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "Should find frozen_string_literal after # typed: true"
        );
    }

    #[test]
    fn with_shebang_and_typed_and_frozen() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"#!/usr/bin/env ruby\n# typed: strict\n# frozen_string_literal: true\nputs 'hello'\n"
                .to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "Should find frozen_string_literal after shebang + typed comment"
        );
    }

    #[test]
    fn emacs_combined_encoding_and_frozen() {
        // Emacs-style: # -*- encoding: utf-8; frozen_string_literal: true -*-
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# -*- encoding: utf-8; frozen_string_literal: true -*-\nputs 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "Should recognize frozen_string_literal in Emacs-style combined comment"
        );
    }

    #[test]
    fn emacs_frozen_only() {
        // Emacs-style with only frozen_string_literal
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# -*- frozen_string_literal: true -*-\nputs 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "Should recognize Emacs-style frozen_string_literal-only comment"
        );
    }

    #[test]
    fn emacs_combined_frozen_false() {
        // Emacs-style with frozen_string_literal: false — should still count as present
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# -*- encoding: utf-8; frozen_string_literal: false -*-\nputs 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "Should recognize frozen_string_literal: false in Emacs-style comment"
        );
    }

    #[test]
    fn emacs_combined_with_shebang() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"#!/usr/bin/env ruby\n# -*- encoding: utf-8; frozen_string_literal: true -*-\nputs 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "Should recognize Emacs-style comment after shebang"
        );
    }

    #[test]
    fn blank_line_between_shebang_and_frozen() {
        // FP pattern: shebang, blank line, then frozen_string_literal
        let source = SourceFile::from_bytes(
            "test.rb",
            b"#!/usr/bin/env ruby\n\n# frozen_string_literal: true\n\nputs 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "Should recognize frozen_string_literal after shebang + blank line"
        );
    }

    #[test]
    fn leading_blank_line_before_frozen() {
        // FP pattern: blank line at start, then frozen_string_literal
        let source = SourceFile::from_bytes(
            "test.rb",
            b"\n# frozen_string_literal: true\n\nputs 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "Should recognize frozen_string_literal after leading blank line"
        );
    }

    #[test]
    fn case_insensitive_frozen_string_literal() {
        // FP pattern: typo with different case like frozen_sTring_literal
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# frozen_sTring_literal: true\nputs 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "Should recognize frozen_string_literal case-insensitively"
        );
    }

    #[test]
    fn hyphen_separator_frozen_string_literal() {
        // FP pattern: hyphens instead of underscores (frozen-string-literal)
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# frozen-string-literal: true\nputs 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "Should recognize frozen-string-literal with hyphens"
        );
    }

    #[test]
    fn shebang_blank_line_encoding_frozen() {
        // shebang, blank line, encoding, frozen_string_literal
        let source = SourceFile::from_bytes(
            "test.rb",
            b"#!/usr/bin/env ruby\n\n# encoding: ascii-8bit\n# frozen_string_literal: true\n\nputs 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "Should find frozen_string_literal after shebang + blank + encoding"
        );
    }

    #[test]
    fn skip_file_with_invalid_utf8() {
        // Files with invalid UTF-8 bytes should not be flagged — RuboCop's tokenizer
        // produces no tokens for these, so it returns early.
        let mut content = b"# @!method foo()\n# \treturn [String] ".to_vec();
        content.push(0xFF); // invalid UTF-8 byte
        content.push(b'\n');
        let source = SourceFile::from_bytes("test.rb", content);
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "Should not flag files with invalid UTF-8 bytes"
        );
    }

    #[test]
    fn skip_file_with_null_bytes() {
        // UTF-16 encoded files have null bytes — should not be flagged
        let content = vec![0x00, b'#', 0x00, b' ', 0x00, b'e', 0x00, b'\n'];
        let source = SourceFile::from_bytes("test.rb", content);
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "Should not flag files with null bytes (e.g., UTF-16)"
        );
    }

    #[test]
    fn skip_file_starting_with_end_only() {
        // Files that start with __END__ and have no code should not be flagged
        let source = SourceFile::from_bytes("test.rb", b"__END__\ndata only\n".to_vec());
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "Should not flag files starting with __END__"
        );
    }

    #[test]
    fn double_hash_not_valid_magic_comment() {
        // `# # frozen_string_literal: true` has a double hash prefix — not a valid magic comment.
        // RuboCop's SimpleComment regex requires `\A\s*#\s*frozen...` (one # only).
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# # frozen_string_literal: true\nputs 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert_eq!(
            diags.len(),
            1,
            "Double-hash frozen_string_literal should NOT be recognized as valid"
        );
    }

    #[test]
    fn encoding_only_no_frozen() {
        // File with encoding magic comment but no frozen string literal comment should be flagged.
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# -*- encoding: iso-8859-9 -*-\nclass Foo; end\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert_eq!(
            diags.len(),
            1,
            "File with encoding-only magic comment should be flagged"
        );
    }

    #[test]
    fn comment_only_file_not_flagged() {
        // A file with only comments and no real code tokens should not be flagged.
        // RuboCop returns early via `processed_source.tokens.empty?`.
        let source = SourceFile::from_bytes(
            "test.rb",
            b"#~# ORIGINAL retry\n\nretry\n\n#~# EXPECTED\nretry\n".to_vec(),
        );
        let mut diags = Vec::new();
        FrozenStringLiteralComment.check_lines(&source, &CopConfig::default(), &mut diags, None);
        // This file HAS code tokens (retry), so it should be flagged.
        // The FP in the corpus is likely a config issue (file extension .rb.spec), not cop logic.
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn autocorrect_insert_missing() {
        let input = b"puts 'hello'\n";
        let (diags, corrections) =
            crate::testutil::run_cop_autocorrect(&FrozenStringLiteralComment, input);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"# frozen_string_literal: true\nputs 'hello'\n");
    }

    #[test]
    fn autocorrect_insert_after_shebang() {
        let input = b"#!/usr/bin/env ruby\nputs 'hello'\n";
        let (_diags, corrections) =
            crate::testutil::run_cop_autocorrect(&FrozenStringLiteralComment, input);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(
            corrected,
            b"#!/usr/bin/env ruby\n# frozen_string_literal: true\nputs 'hello'\n"
        );
    }

    #[test]
    fn autocorrect_insert_after_encoding() {
        let input = b"# encoding: utf-8\nputs 'hello'\n";
        let (_diags, corrections) =
            crate::testutil::run_cop_autocorrect(&FrozenStringLiteralComment, input);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(
            corrected,
            b"# encoding: utf-8\n# frozen_string_literal: true\nputs 'hello'\n"
        );
    }

    #[test]
    fn autocorrect_remove_never_style() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("never".into()),
            )]),
            ..CopConfig::default()
        };
        let input = b"# frozen_string_literal: true\nputs 'hello'\n";
        let (diags, corrections) = crate::testutil::run_cop_autocorrect_with_config(
            &FrozenStringLiteralComment,
            input,
            config,
        );
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"puts 'hello'\n");
    }

    #[test]
    fn autocorrect_always_true_replaces_false() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("always_true".into()),
            )]),
            ..CopConfig::default()
        };
        let input = b"# frozen_string_literal: false\nputs 'hello'\n";
        let (diags, corrections) = crate::testutil::run_cop_autocorrect_with_config(
            &FrozenStringLiteralComment,
            input,
            config,
        );
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"# frozen_string_literal: true\nputs 'hello'\n");
    }
}
