use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use regex::Regex;
use std::sync::OnceLock;

/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=3, FN=234.
///
/// FP=3 came from two bugs in the top-of-file scan:
/// - The cop stopped at the first non-magic comment, so generated-file headers
///   like `# typed: strict` ... `# frozen_string_literal: true` were treated as
///   if the earlier comment were the last magic comment.
/// - The magic-comment recognizer was too loose in some places (`coding:utf-8`)
///   and too narrow in others (`frozen-string-literal`, `rbs_inline`,
///   Emacs-style `encoding : utf-8`).
///
/// FN=234 were dominated by Emacs-style encoding comments and `rbs_inline`
/// comments that RuboCop recognizes before the first line of code.
///
/// This implementation now mirrors RuboCop's selection rule more closely:
/// inspect all comment lines before the first code line, take the last comment
/// that matches RuboCop-compatible magic-comment patterns, and require a blank
/// line only after that final magic comment.
///
/// Acceptance gate after the fix: expected 6,890, actual 7,313, CI baseline
/// 6,659, raw delta +654, file-drop noise 1,006, missing 0. The rerun passed
/// because the delta stayed within the existing `jruby` parser-crash noise.
///
/// Follow-up investigation on 2026-03-10 found remaining corpus FN on files
/// whose first line starts with a UTF-8 BOM before a valid magic comment. The
/// top-of-file scan treated the BOM as code, so the cop missed
/// `\xEF\xBB\xBF# frozen_string_literal: true` and
/// `\xEF\xBB\xBF# coding: utf-8` headers.
///
/// Follow-up investigation on 2026-03-13 found 1 remaining FP on
/// jruby/jruby `spec/ruby/language/fixtures/case_magic_comment.rb`:
/// `# CoDiNg:   bIg5` (triple space after colon). The encoding regex used
/// `:\s+` which matched multiple spaces, but RuboCop's SimpleComment#encoding
/// uses a literal `": "` (colon-space) so multi-space comments don't match.
/// Fixed by changing `:\s+` to `: ` in encoding_re().
///
/// Follow-up investigation on 2026-03-31 found 6 FP in files whose first
/// non-comment line is an exact `__END__` marker. RuboCop with Prism treats
/// those files as having no code before the data section, so it does not
/// require a blank line after a leading magic comment. This implementation now
/// stops the top-of-file scan at an exact `__END__` line; prefixed, indented,
/// or trailing-space variants still count as code because RuboCop flags them.
pub struct EmptyLineAfterMagicComment;

impl Cop for EmptyLineAfterMagicComment {
    fn name(&self) -> &'static str {
        "Layout/EmptyLineAfterMagicComment"
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
        let lines: Vec<&[u8]> = source.lines().collect();
        let last_magic_idx = match last_magic_comment_line(&lines) {
            Some(idx) => idx,
            None => return,
        };

        // Check if the line after the last magic comment is blank
        let next_idx = last_magic_idx + 1;
        if next_idx >= lines.len() {
            return;
        }

        let next_line = lines[next_idx];
        let is_blank = next_line
            .iter()
            .all(|&b| b == b' ' || b == b'\t' || b == b'\r');

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

fn last_magic_comment_line(lines: &[&[u8]]) -> Option<usize> {
    let limit = magic_comment_scan_limit(lines)?;

    let mut last_magic = None;
    for (idx, line) in lines.iter().take(limit).enumerate() {
        let line = if idx == 0 { strip_utf8_bom(line) } else { line };
        if is_magic_comment(line) {
            last_magic = Some(idx);
        }
    }

    last_magic
}

fn magic_comment_scan_limit(lines: &[&[u8]]) -> Option<usize> {
    for (idx, line) in lines.iter().enumerate() {
        let line = if idx == 0 { strip_utf8_bom(line) } else { line };
        let trimmed = trim_leading_space(line);
        if trimmed.is_empty() || trimmed.starts_with(b"#") {
            continue;
        }
        if is_data_section_marker(line) {
            return None;
        }
        return Some(idx);
    }

    Some(lines.len())
}

fn is_magic_comment(line: &[u8]) -> bool {
    let Ok(line_str) = std::str::from_utf8(line) else {
        return false;
    };

    is_simple_magic_comment(line_str)
        || is_emacs_magic_comment(line_str)
        || is_vim_magic_comment(line_str)
}

fn is_simple_magic_comment(comment: &str) -> bool {
    if frozen_string_re().is_match(comment)
        || shareable_constant_value_re().is_match(comment)
        || typed_re().is_match(comment)
    {
        return true;
    }

    if let Some(caps) = rbs_inline_re().captures(comment) {
        return matches!(
            &caps["token"].to_ascii_lowercase()[..],
            "enabled" | "disabled"
        );
    }

    encoding_re().is_match(comment)
}

fn is_emacs_magic_comment(comment: &str) -> bool {
    let Some(caps) = emacs_re().captures(comment) else {
        return false;
    };
    caps["token"].split(';').map(str::trim).any(|token| {
        emacs_encoding_re().is_match(token)
            || emacs_frozen_string_re().is_match(token)
            || emacs_shareable_constant_value_re().is_match(token)
    })
}

fn is_vim_magic_comment(comment: &str) -> bool {
    let Some(caps) = vim_re().captures(comment) else {
        return false;
    };
    let tokens: Vec<_> = caps["token"].split(", ").map(str::trim).collect();
    tokens.len() > 1 && tokens.iter().any(|token| vim_encoding_re().is_match(token))
}

fn trim_leading_space(line: &[u8]) -> &[u8] {
    let start = line
        .iter()
        .position(|&b| b != b' ' && b != b'\t' && b != b'\r')
        .unwrap_or(line.len());
    &line[start..]
}

fn is_data_section_marker(line: &[u8]) -> bool {
    line.strip_suffix(b"\r").unwrap_or(line) == b"__END__"
}

fn strip_utf8_bom(line: &[u8]) -> &[u8] {
    line.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(line)
}

fn frozen_string_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^\s*#\s*frozen[_-]string[_-]literal:\s*(?P<token>[[:alnum:]_-]+)\s*$")
            .unwrap()
    })
}

fn shareable_constant_value_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^\s*#\s*shareable[_-]constant[_-]value:\s*(?P<token>[[:alnum:]_-]+)\s*$")
            .unwrap()
    })
}

fn typed_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^\s*#\s*typed:\s*(?P<token>[[:alnum:]_-]+)\s*$").unwrap())
}

fn rbs_inline_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^\s*#\s*rbs_inline:\s*(?P<token>[[:alnum:]_-]+)\s*$").unwrap()
    })
}

fn encoding_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)^\s*#\s*(?:frozen[_-]string[_-]literal:\s*(?:true|false))?\s*(?:en)?coding: (?P<token>[[:alnum:]_-]+(?:-[[:alnum:]_-]+)*)",
        )
        .unwrap()
    })
}

fn emacs_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)-\*-(?P<token>.+)-\*-").unwrap())
}

fn emacs_encoding_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^(?:en)?coding\s*:\s*(?P<token>[[:alnum:]_-]+(?:-[[:alnum:]_-]+)*)$")
            .unwrap()
    })
}

fn emacs_frozen_string_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^frozen[_-]string[_-]literal\s*:\s*(?P<token>[[:alnum:]_-]+)$").unwrap()
    })
}

fn emacs_shareable_constant_value_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^shareable[_-]constant[_-]value\s*:\s*(?P<token>[[:alnum:]_-]+)$").unwrap()
    })
}

fn vim_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)#\s*vim:\s*(?P<token>.+)$").unwrap())
}

fn vim_encoding_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^fileencoding=(?P<token>[[:alnum:]_-]+(?:-[[:alnum:]_-]+)*)$").unwrap()
    })
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
        rbs_inline_enabled = "rbs_inline_enabled.rb",
    );

    #[test]
    fn autocorrect_insert_blank_after_frozen_string() {
        let input = b"# frozen_string_literal: true\nx = 1\n";
        let (_diags, corrections) =
            crate::testutil::run_cop_autocorrect(&EmptyLineAfterMagicComment, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"# frozen_string_literal: true\n\nx = 1\n");
    }

    #[test]
    fn autocorrect_insert_blank_after_multiple_magic() {
        let input = b"# frozen_string_literal: true\n# encoding: utf-8\nx = 1\n";
        let (_diags, corrections) =
            crate::testutil::run_cop_autocorrect(&EmptyLineAfterMagicComment, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(
            corrected,
            b"# frozen_string_literal: true\n# encoding: utf-8\n\nx = 1\n"
        );
    }

    #[test]
    fn no_offense_when_non_magic_comments_precede_later_magic_comment() {
        let source = b"# typed: strict\n# generated file\n# do not edit\n# frozen_string_literal: true\n\nrequire \"set\"\n";
        let diags = crate::testutil::run_cop_full(&EmptyLineAfterMagicComment, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn no_offense_for_invalid_coding_without_space() {
        let source = b"# coding:utf-8\nrequire_relative \"helper\"\n";
        let diags = crate::testutil::run_cop_full(&EmptyLineAfterMagicComment, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn no_offense_when_kebab_magic_comment_follows_emacs_encoding() {
        let source =
            b"# -*- coding: us-ascii -*-\n# frozen-string-literal: false\n\n# regular comment\nrequire \"logger\"\n";
        let diags = crate::testutil::run_cop_full(&EmptyLineAfterMagicComment, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn no_offense_for_multi_space_encoding_comment() {
        // RuboCop uses a literal space after "coding:" so multi-space doesn't match
        let source = b"# CoDiNg:   bIg5\n$magic_comment_result = __ENCODING__.name\n";
        let diags = crate::testutil::run_cop_full(&EmptyLineAfterMagicComment, source);
        assert!(
            diags.is_empty(),
            "expected no offense for multi-space encoding comment, got {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_when_magic_comment_precedes_exact_end_marker() {
        let source = b"# -*- encoding : utf-8 -*-\n__END__\nrequire_relative \"helper\"\n";
        let diags = crate::testutil::run_cop_full(&EmptyLineAfterMagicComment, source);
        assert!(
            diags.is_empty(),
            "expected no offense when magic comment is followed by exact __END__, got {:?}",
            diags
        );
    }

    #[test]
    fn offense_when_magic_comment_precedes_end_prefix_identifier() {
        let source = b"# frozen_string_literal: true\n__END__foo = 1\nclass Foo; end\n";
        let diags = crate::testutil::run_cop_full(&EmptyLineAfterMagicComment, source);
        assert_eq!(
            diags.len(),
            1,
            "expected offense for __END__ prefix identifier after magic comment"
        );
        assert_eq!(diags[0].location.line, 2);
        assert_eq!(diags[0].message, "Add an empty line after magic comments.");
    }

    #[test]
    fn offense_when_utf8_bom_precedes_frozen_string_comment() {
        let source = b"\xEF\xBB\xBF# frozen_string_literal: true\nclass Foo; end\n";
        let diags = crate::testutil::run_cop_full(&EmptyLineAfterMagicComment, source);
        assert_eq!(
            diags.len(),
            1,
            "expected offense for BOM-prefixed magic comment"
        );
        assert_eq!(diags[0].location.line, 2);
        assert_eq!(diags[0].message, "Add an empty line after magic comments.");
    }

    #[test]
    fn offense_when_utf8_bom_precedes_coding_comment() {
        let source = b"\xEF\xBB\xBF# coding: utf-8\nrequire_relative \"helper\"\n";
        let diags = crate::testutil::run_cop_full(&EmptyLineAfterMagicComment, source);
        assert_eq!(
            diags.len(),
            1,
            "expected offense for BOM-prefixed coding comment"
        );
        assert_eq!(diags[0].location.line, 2);
        assert_eq!(diags[0].message, "Add an empty line after magic comments.");
    }
}
