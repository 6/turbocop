use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

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

        let mut idx = 0;

        // Skip shebang
        if idx < lines.len() && lines[idx].starts_with(b"#!") {
            idx += 1;
        }

        // Skip encoding comment
        if idx < lines.len() && is_encoding_comment(lines[idx]) {
            idx += 1;
        }

        // Remember where to insert the comment (after shebang/encoding)
        let insert_after_line = idx; // 0-indexed line number

        // Scan leading comment lines for the frozen_string_literal magic comment.
        while idx < lines.len() && is_comment_line(lines[idx]) {
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

fn is_comment_line(line: &[u8]) -> bool {
    let trimmed = line.iter().skip_while(|&&b| b == b' ' || b == b'\t');
    matches!(trimmed.clone().next(), Some(b'#'))
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

fn is_frozen_string_literal_comment(line: &[u8]) -> bool {
    let s = match std::str::from_utf8(line) {
        Ok(s) => s,
        Err(_) => return false,
    };
    // Allow leading whitespace, then `#`, then optional space, then `frozen_string_literal:`
    let s = s.trim_start();
    let trimmed = s.strip_prefix('#').unwrap_or("");
    let trimmed = trimmed.trim_start();
    trimmed.starts_with("frozen_string_literal:")
}

fn is_frozen_string_literal_true(line: &[u8]) -> bool {
    let s = match std::str::from_utf8(line) {
        Ok(s) => s,
        Err(_) => return false,
    };
    // Allow leading whitespace
    let s = s.trim_start();
    let trimmed = s.strip_prefix('#').unwrap_or("");
    let trimmed = trimmed.trim_start();
    trimmed
        .strip_prefix("frozen_string_literal:")
        .map(|rest| rest.trim() == "true")
        .unwrap_or(false)
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
