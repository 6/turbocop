use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/Encoding checks for unnecessary utf-8 encoding comments.
///
/// ## Investigation findings (2026-03-18)
///
/// ### FP root cause fixed (round 1):
/// - Non-magic comment lines (e.g., `# This is a description`) on line 1 or 2
///   should stop magic comment processing. Previously we checked the first 3 lines
///   regardless; now we match RuboCop's behavior of only processing contiguous
///   magic comment lines (frozen_string_literal, encoding, shareable_constant_value,
///   typed, rbs_inline). Regular comments terminate the search.
///
/// ### FN root cause fixed (round 1):
/// - `# coding: utf-8` format was already handled but lacked fixture coverage.
///   Added coding_format.rb scenario.
///
/// ## Investigation findings (2026-03-23)
///
/// ### FP root causes fixed (58 FP):
/// 1. `# vim:fileencoding=utf-8` (44 FP, resque-scheduler): Vim comments with only
///    one token should NOT detect encoding. RuboCop's VimComment#encoding returns nil
///    when tokens.size <= 1, making valid? false and stopping magic comment processing.
/// 2. `# encoding:utf-8` / `#coding:utf-8` (12 FP, catarse/gollum/padrino/piotrmurach):
///    RuboCop's SimpleComment encoding regex requires `: ` (colon + space) after the
///    keyword. `encoding:utf-8` (no space) is NOT a valid magic comment.
/// 3. `# -*- Mode: Ruby; tab-width: 2 -*-` (1 FP, juvia) and `# -*- rspec -*-` (1 FP, ffi):
///    Emacs comments with unrecognized keywords (Mode, tab-width, rspec) are NOT valid
///    magic comments. RuboCop checks `any?` which requires encoding/frozen_string_literal/
///    shareable_constant_value/typed/rbs_inline to be specified. Previously we treated
///    ALL emacs-style comments as valid.
///
/// ### FN root cause fixed (494 FN):
/// - `# -*- coding: utf-8 -*- #` (all 494 from rouge-ruby/rouge): Emacs comments
///   with trailing content after `-*-` were not matched. RuboCop's Emacs regex
///   `-*-(?<token>.+)-*-` greedily matches between first and last `-*-` occurrences,
///   ignoring trailing text. Fixed by using `rfind("-*-")` instead of `ends_with("-*-")`.
///
/// ## Investigation findings (2026-03-26)
///
/// ### FP root causes fixed (3 FP):
/// - Indented first-line comments like `  # encoding: utf-8`, `\t#encoding: utf-8`,
///   and `  # -*- coding: utf-8 -*-` are ordinary comments, not magic comments.
///   RuboCop's `MagicComment#valid?` requires the raw line to start with `#`, but
///   we previously called `trim()` before checking, which incorrectly promoted
///   indented comments into top-of-file magic comments.
///
/// ### FN root cause fixed (1 FN):
/// - `# coding: utf-8 -*-` is still an offense in RuboCop because
///   `SimpleComment#encoding` captures the first token after `coding: ` and does
///   not anchor the rest of the line. We previously required the entire remainder
///   to equal `utf-8`, which missed this malformed-but-detected case.
pub struct Encoding;

impl Cop for Encoding {
    fn name(&self) -> &'static str {
        "Style/Encoding"
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
        let total_len = source.as_bytes().len();
        let mut byte_offset: usize = 0;

        // Process contiguous magic comment lines at the top of the file,
        // matching RuboCop's behavior: skip shebangs, then process each line
        // as a magic comment. Stop as soon as a non-magic-comment line is found.
        for (i, line) in source.lines().enumerate() {
            let line_len = line.len() + 1; // +1 for newline

            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s.trim_end(),
                Err(_) => {
                    break;
                }
            };

            // Skip shebang lines
            if line_str.starts_with("#!") {
                byte_offset += line_len;
                continue;
            }

            // Non-comment or blank line: stop processing
            if !line_str.starts_with('#') {
                break;
            }

            // Must be a valid magic comment to continue processing.
            // RuboCop uses MagicComment.parse(line).valid? which checks for
            // encoding, frozen_string_literal, shareable_constant_value, typed, rbs_inline.
            if !is_magic_comment(line_str) {
                break;
            }

            // Check if it's specifically a UTF-8 encoding comment
            if is_utf8_encoding_comment(line_str) {
                let mut diag = self.diagnostic(
                    source,
                    i + 1,
                    0,
                    "Unnecessary utf-8 encoding comment.".to_string(),
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

            byte_offset += line_len;
        }
    }
}

/// Extract the inner content from an emacs-style comment: `-*- ... -*-`.
/// Returns the content between the first `-*-` and the last `-*-`, or None
/// if the line doesn't contain an emacs-style magic comment.
/// RuboCop's regex: `/-\*-(?<token>.+)-\*-/` matches greedily between first
/// and last occurrence, so `# -*- coding: utf-8 -*- #` is valid (trailing `#`).
fn extract_emacs_inner(line: &str) -> Option<&str> {
    let start = line.find("-*-")?;
    let after_start = start + 3;
    let rest = line.get(after_start..)?;
    let end = rest.rfind("-*-")?;
    if end == 0 {
        return None; // No content between markers
    }
    Some(rest[..end].trim())
}

/// Extract the token content from a vim-style comment: `# vim: ...` or `# vim:...`.
/// RuboCop's regex: `/#\s*vim:\s*(?<token>.+)/`
fn extract_vim_tokens(line: &str) -> Option<&str> {
    let lower = line.to_lowercase();
    // Find "vim:" after "#"
    let hash_pos = lower.find('#')?;
    let after_hash = lower[hash_pos + 1..].trim_start();
    if let Some(rest) = after_hash.strip_prefix("vim:") {
        let content = rest.trim_start();
        if content.is_empty() {
            return None;
        }
        // Return from original line at the corresponding position
        // We need to work with the original line for the tokens
        // Find "vim:" in the original line (case-insensitive)
        let orig_after_hash = line[hash_pos + 1..].trim_start();
        // Skip "vim:" (4 chars) case-insensitively
        let orig_rest = &orig_after_hash[4..];
        let orig_content = orig_rest.trim_start();
        if orig_content.is_empty() {
            return None;
        }
        return Some(orig_content);
    }
    None
}

/// Count tokens in a vim comment (comma-separated).
fn vim_token_count(tokens_str: &str) -> usize {
    tokens_str
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .count()
}

/// Check if a comment line is a valid Ruby magic comment.
/// Matches RuboCop's MagicComment.parse(line).valid? behavior:
/// valid? = starts_with('#') && any?
/// any? checks encoding_specified? || frozen_string_literal_specified? ||
///   rbs_inline_specified? || shareable_constant_value_specified? || typed_specified?
///
/// Key differences from previous implementation:
/// - Emacs comments: only valid if they contain a recognized magic keyword
///   (encoding/coding, frozen_string_literal, shareable_constant_value, typed, rbs_inline)
/// - Vim comments: encoding (fileencoding) only detected with 2+ tokens;
///   vim comments can't specify frozen_string_literal/shareable_constant_value/typed/rbs_inline
/// - Simple comments: require ": " (colon + space) after keyword
fn is_magic_comment(line: &str) -> bool {
    // Must start with #
    if !line.starts_with('#') {
        return false;
    }

    let lower = line.to_lowercase();

    // Emacs style: try to extract -*- ... -*- content
    if let Some(inner) = extract_emacs_inner(&lower) {
        // Split by ';' to get tokens, check if any is a recognized magic keyword
        let emacs_keywords = &[
            "encoding",
            "coding", // encoding/coding
            "frozen_string_literal",
            "frozen-string-literal",
            "shareable_constant_value",
            "shareable-constant-value",
            "typed",
            "rbs_inline",
        ];
        for token in inner.split(';') {
            let token = token.trim();
            for kw in emacs_keywords {
                // Token format is "keyword: value" or "keyword : value"
                if let Some(rest) = token.strip_prefix(kw) {
                    if rest.trim_start().starts_with(':') {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    // Vim style: # vim: ...
    if let Some(tokens_str) = extract_vim_tokens(line) {
        // Vim comments can only specify fileencoding.
        // fileencoding is only detected when there are 2+ tokens.
        // So a vim comment is only "valid" (has any recognized keyword) if:
        // - it has 2+ tokens AND one of them starts with "fileencoding="
        let count = vim_token_count(tokens_str);
        if count >= 2 {
            let lower_tokens = tokens_str.to_lowercase();
            for token in lower_tokens.split(',') {
                let token = token.trim();
                if let Some(rest) = token.strip_prefix("fileencoding") {
                    if rest.trim_start().starts_with('=') {
                        return true;
                    }
                }
            }
        }
        // Vim comments with single token or without fileencoding: not valid
        return false;
    }

    // Simple magic comment keywords (case-insensitive)
    // RuboCop's SimpleComment requires "# keyword: value" with ": " (colon + space)
    // The regex patterns are like: /\A\s*#\s*(?:en)?coding:\s*TOKEN/io
    // Note: the colon is part of the keyword regex, and \s* allows space after colon.
    // But the key point: the keyword MUST be followed by ": " (colon then optional space
    // is fine for matching, but the keyword itself uses ":").
    // Actually re-reading: /\A\s*#\s*(?:en)?coding: (TOKEN)/io - that ": " is
    // "colon space" literally. So "# encoding:utf-8" (no space) does NOT match.
    // But /\A\s*#\s*frozen[_-]string[_-]literal:\s*TOKEN\s*\z/io uses ":\s*" so
    // it DOES allow no space after colon for frozen_string_literal.
    //
    // For encoding specifically (SimpleComment#encoding):
    //   /\A\s*\#\s*(frozen_string_literal:\s*(true|false))?\s*(?:en)?coding: (TOKEN)/io
    // The ": " after coding requires a space. So "# encoding:utf-8" is NOT valid.
    //
    // For other keywords, the pattern is :\s* which allows no space.
    // But for is_magic_comment, we only care about valid? = any? which checks
    // if any keyword is specified. So we need to match what RuboCop actually accepts.

    // Extract content after # (with optional space)
    let Some(content) = simple_comment_content(&lower) else {
        return false;
    };

    // encoding/coding: requires ": " (colon + space) for SimpleComment
    // Actually let me re-check: SimpleComment#encoding regex is:
    //   /\A\s*\#\s*(frozen_string_literal:\s*(true|false))?\s*(?:en)?coding: (TOKEN)/io
    // The " " after "coding:" is a literal space in the regex. So "coding:utf-8" won't match.
    //
    // But frozen_string_literal regex is:
    //   /\A\s*#\s*frozen[_-]string[_-]literal:\s*TOKEN\s*\z/io
    // Here ":\s*" allows no space. So "frozen_string_literal:true" WOULD match.
    //
    // For is_magic_comment, we need: does any keyword match => valid? = true
    let simple_magic_prefixes_with_colon_space = &[
        "encoding: ", // requires space after colon
        "coding: ",   // requires space after colon
    ];

    let simple_magic_prefixes_colon_optional_space = &[
        "frozen_string_literal:",
        "frozen-string-literal:",
        "shareable_constant_value:",
        "shareable-constant-value:",
        "typed:",
        "rbs_inline:",
    ];

    for prefix in simple_magic_prefixes_with_colon_space {
        if content.starts_with(prefix) {
            return true;
        }
    }

    for prefix in simple_magic_prefixes_colon_optional_space {
        if content.starts_with(prefix) {
            return true;
        }
    }

    false
}

/// Check if a comment line is a UTF-8 encoding magic comment.
/// Must match RuboCop's behavior precisely:
/// - SimpleComment: `# encoding: utf-8` or `# coding: utf-8` (requires space after colon)
/// - EmacsComment: `# -*- coding: utf-8 -*-` (may have trailing content after -\*-)
/// - VimComment: `# vim: filetype=ruby, fileencoding=utf-8` (requires 2+ tokens)
fn is_utf8_encoding_comment(line: &str) -> bool {
    if !line.starts_with('#') {
        return false;
    }

    let lower = line.to_lowercase();

    // Emacs style: extract content between first and last -*-
    if let Some(inner) = extract_emacs_inner(&lower) {
        // Check each semicolon-separated token for encoding/coding with utf-8 value
        for token in inner.split(';') {
            let token = token.trim();
            // Match (en)?coding : TOKEN pattern
            let after_kw = token
                .strip_prefix("encoding")
                .or_else(|| token.strip_prefix("coding"));
            if let Some(rest) = after_kw {
                let rest = rest.trim_start();
                if let Some(value) = rest.strip_prefix(':') {
                    let value = value.trim();
                    if value.eq_ignore_ascii_case("utf-8") {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    // Vim style: fileencoding=utf-8 with 2+ tokens
    if let Some(tokens_str) = extract_vim_tokens(line) {
        let count = vim_token_count(tokens_str);
        if count >= 2 {
            let lower_tokens = tokens_str.to_lowercase();
            for token in lower_tokens.split(',') {
                let token = token.trim();
                if let Some(rest) = token.strip_prefix("fileencoding") {
                    let rest = rest.trim_start();
                    if let Some(value) = rest.strip_prefix('=') {
                        let value = value.trim();
                        if value == "utf-8" {
                            return true;
                        }
                    }
                }
            }
        }
        return false;
    }

    // Simple format: # encoding: utf-8 or # coding: utf-8
    // Requires space after colon per RuboCop's SimpleComment regex.
    // Extract content after # (with optional whitespace)
    if let Some(value) = extract_simple_encoding_value(&lower) {
        if value.eq_ignore_ascii_case("utf-8") {
            return true;
        }
    }

    false
}

/// Extract the content after a leading '#', trimming only the whitespace that Ruby
/// allows between '#' and the magic comment keyword.
fn simple_comment_content(line: &str) -> Option<&str> {
    Some(line.strip_prefix('#')?.trim_start())
}

/// Consume a leading magic-comment token value. RuboCop uses `[[:alnum:]\\-_]+`
/// for the token, so parsing stops at the first non-token character.
fn take_magic_token(value: &str) -> Option<&str> {
    let end = value
        .char_indices()
        .find_map(|(idx, ch)| {
            (!ch.is_ascii_alphanumeric() && ch != '-' && ch != '_').then_some(idx)
        })
        .unwrap_or(value.len());

    (end > 0).then_some(&value[..end])
}

/// Skip the optional `frozen_string_literal: true|false` prefix that RuboCop's
/// SimpleComment encoding regex accepts ahead of `coding: ...`.
fn skip_simple_frozen_string_literal_prefix(content: &str) -> &str {
    for prefix in ["frozen_string_literal:", "frozen-string-literal:"] {
        if let Some(after_prefix) = content.strip_prefix(prefix) {
            let after_prefix = after_prefix.trim_start();
            if let Some(value) = take_magic_token(after_prefix) {
                if matches!(value, "true" | "false") {
                    return after_prefix[value.len()..].trim_start();
                }
            }
        }
    }

    content
}

/// Extract the encoding token for a simple magic comment.
/// Matches RuboCop's `SimpleComment#encoding` behavior:
/// - the raw line must start with '#'
/// - optional spaces after '#'
/// - optional `frozen_string_literal: true|false` prefix
/// - `encoding: ` or `coding: ` with exactly one space after the colon
/// - capture only the leading token characters, ignoring any trailing junk
fn extract_simple_encoding_value(line: &str) -> Option<&str> {
    let content = simple_comment_content(line)?;
    let content = skip_simple_frozen_string_literal_prefix(content);

    for prefix in ["encoding: ", "coding: "] {
        if let Some(after_prefix) = content.strip_prefix(prefix) {
            return take_magic_token(after_prefix);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_scenario_fixture_tests!(
        Encoding,
        "cops/style/encoding",
        standard = "standard.rb",
        mixed_case = "mixed_case.rb",
        after_shebang = "after_shebang.rb",
        coding_format = "coding_format.rb",
        malformed_trailing_marker = "malformed_trailing_marker.rb",
        emacs_with_trailing = "emacs_with_trailing.rb",
        emacs_encoding = "emacs_encoding.rb",
    );

    #[test]
    fn autocorrect_remove_encoding() {
        let input = b"# encoding: utf-8\nx = 1\n";
        let (_diags, corrections) = crate::testutil::run_cop_autocorrect(&Encoding, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1\n");
    }

    #[test]
    fn no_fp_regular_comment_before_encoding() {
        // A regular comment (not a magic comment) on line 1 should stop processing.
        // RuboCop only processes contiguous magic comment lines.
        let input = b"# This is a regular comment\n# encoding: utf-8\ndef foo; end\n";
        let diags = crate::testutil::run_cop_full(&Encoding, input);
        assert!(
            diags.is_empty(),
            "Should NOT flag encoding comment after a regular (non-magic) comment: {:?}",
            diags
        );
    }

    #[test]
    fn flags_encoding_after_frozen_string_literal() {
        // frozen_string_literal IS a valid magic comment, so processing continues
        let input = b"# frozen_string_literal: true\n# encoding: utf-8\ndef foo; end\n";
        let diags = crate::testutil::run_cop_full(&Encoding, input);
        assert_eq!(
            diags.len(),
            1,
            "Should flag encoding comment after frozen_string_literal"
        );
    }

    #[test]
    fn flags_coding_format() {
        // # coding: utf-8 is a valid encoding magic comment
        let input = b"# coding: utf-8\ndef foo; end\n";
        let diags = crate::testutil::run_cop_full(&Encoding, input);
        assert_eq!(diags.len(), 1, "Should flag # coding: utf-8");
    }

    #[test]
    fn no_fp_encoding_after_non_magic_comment_line2() {
        // Shebang is OK, but a regular comment on line 2 should stop processing
        let input =
            b"#!/usr/bin/env ruby\n# A description comment\n# encoding: utf-8\ndef foo; end\n";
        let diags = crate::testutil::run_cop_full(&Encoding, input);
        assert!(
            diags.is_empty(),
            "Should NOT flag encoding after non-magic comment: {:?}",
            diags
        );
    }

    #[test]
    fn no_fp_vim_single_token_fileencoding() {
        // # vim:fileencoding=utf-8 with only one token is NOT detected by RuboCop.
        // VimComment.encoding returns nil when tokens.size == 1,
        // so encoding_specified? is false, valid? is false, processing stops.
        let input = b"# vim:fileencoding=utf-8\ndef foo; end\n";
        let diags = crate::testutil::run_cop_full(&Encoding, input);
        assert!(
            diags.is_empty(),
            "Should NOT flag vim:fileencoding=utf-8 with single token: {:?}",
            diags
        );
    }

    #[test]
    fn no_fp_encoding_no_space_after_colon() {
        // # encoding:utf-8 (no space after colon) is NOT a valid magic comment in RuboCop.
        // SimpleComment encoding regex requires ": " (colon + space).
        let input = b"# encoding:utf-8\ndef foo; end\n";
        let diags = crate::testutil::run_cop_full(&Encoding, input);
        assert!(
            diags.is_empty(),
            "Should NOT flag # encoding:utf-8 (no space): {:?}",
            diags
        );
    }

    #[test]
    fn no_fp_coding_no_space_after_colon() {
        // #coding:utf-8 (no space after colon) is NOT a valid magic comment in RuboCop.
        let input = b"#coding:utf-8\ndef foo; end\n";
        let diags = crate::testutil::run_cop_full(&Encoding, input);
        assert!(
            diags.is_empty(),
            "Should NOT flag #coding:utf-8 (no space): {:?}",
            diags
        );
    }

    #[test]
    fn no_fp_indented_simple_encoding_comment() {
        // RuboCop requires magic comments to start with '#'. Leading indentation
        // makes this an ordinary comment, so Style/Encoding should not fire.
        let input = b"  # encoding: utf-8\n  require 'foo'\n";
        let diags = crate::testutil::run_cop_full(&Encoding, input);
        assert!(
            diags.is_empty(),
            "Should NOT flag indented encoding comment: {:?}",
            diags
        );
    }

    #[test]
    fn no_fp_indented_emacs_encoding_comment() {
        // Indented editor comments are also ordinary comments, not magic comments.
        let input = b"  # -*- coding: utf-8 -*-\n##########################################################################\n";
        let diags = crate::testutil::run_cop_full(&Encoding, input);
        assert!(
            diags.is_empty(),
            "Should NOT flag indented emacs encoding comment: {:?}",
            diags
        );
    }

    #[test]
    fn no_fp_emacs_non_encoding_keywords() {
        // # -*- Mode: Ruby; tab-width: 2 -*- is an emacs comment with non-encoding keywords.
        // RuboCop's MagicComment checks encoding_specified?, frozen_string_literal_specified?, etc.
        // None of these match Mode/tab-width, so valid? returns false, processing stops.
        let input =
            b"# -*- Mode: Ruby; tab-width: 2 -*-\n# -*- encoding: utf-8 -*-\ndef foo; end\n";
        let diags = crate::testutil::run_cop_full(&Encoding, input);
        assert!(
            diags.is_empty(),
            "Should NOT flag encoding after non-magic emacs comment: {:?}",
            diags
        );
    }

    #[test]
    fn no_fp_emacs_rspec_comment() {
        // # -*- rspec -*- is an emacs comment with unrecognized keyword.
        // valid? returns false, processing stops, encoding on line 2 is not checked.
        let input = b"# -*- rspec -*-\n# encoding: utf-8\ndef foo; end\n";
        let diags = crate::testutil::run_cop_full(&Encoding, input);
        assert!(
            diags.is_empty(),
            "Should NOT flag encoding after non-magic emacs comment: {:?}",
            diags
        );
    }

    #[test]
    fn flags_emacs_encoding_with_trailing() {
        // # -*- coding: utf-8 -*- # has trailing content after -*-
        // RuboCop still matches this because the regex is -*-(.+)-*- which greedily captures.
        let input = b"# -*- coding: utf-8 -*- #\nmodule Foo; end\n";
        let diags = crate::testutil::run_cop_full(&Encoding, input);
        assert_eq!(
            diags.len(),
            1,
            "Should flag emacs encoding with trailing content"
        );
    }

    #[test]
    fn flags_vim_fileencoding_multi_token() {
        // # vim: filetype=ruby, fileencoding=utf-8 has 2+ tokens, so encoding is detected
        let input = b"# vim: filetype=ruby, fileencoding=utf-8\ndef foo; end\n";
        let diags = crate::testutil::run_cop_full(&Encoding, input);
        assert_eq!(
            diags.len(),
            1,
            "Should flag vim fileencoding with multiple tokens"
        );
    }

    #[test]
    fn flags_after_shebang_then_magic_comment() {
        // shebang → frozen_string_literal → encoding: all valid magic comments
        let input = b"#!/usr/bin/env ruby\n# frozen_string_literal: true\n# encoding: utf-8\ndef foo; end\n";
        let diags = crate::testutil::run_cop_full(&Encoding, input);
        assert_eq!(
            diags.len(),
            1,
            "Should flag encoding after shebang + frozen_string_literal"
        );
    }
}
