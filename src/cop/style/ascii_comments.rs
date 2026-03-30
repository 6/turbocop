use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/AsciiComments: Use only ASCII symbols in comments.
///
/// Root cause of prior FPs (~1,549): The old `check_lines` approach used
/// `line_str.find('#')` to detect comment starts, which matched `#` inside
/// string literals (interpolation `"#{var}"`, HTML entities `"&#83;"`, etc.).
///
/// A pure Prism approach was tried (commit fc9eb19) but reverted because it
/// produced ~1,090 different excess offenses — likely from Prism including
/// shebang lines, `__END__` sections, or encoding differences vs RuboCop's
/// `processed_source.comments`.
///
/// Current fix (2026-03-08): Uses `check_source` with Prism's `parse_result.comments()`
/// to get accurate comment byte ranges. For each Prism comment, scans only
/// within that byte range for non-ASCII characters. This avoids both the
/// string-literal FPs (old approach) and the shebang/encoding issues (reverted
/// approach) because we now correctly scope scanning to real comment content
/// only, using the same AllowedChars config as before.
///
/// FN fixes (2026-03-30):
/// - Comments beginning with `#!!` were missed because we skipped every comment
///   that started with `#!`, even though RuboCop only stays quiet on ordinary
///   ASCII shebangs because they contain no non-ASCII characters.
/// - Comments in legacy-encoded files (for example `# coding: ISO-8859-15`)
///   were missed because the cop required each comment slice to be valid UTF-8
///   before scanning it. RuboCop still checks those comments after honoring the
///   file encoding, so we now fall back to byte-wise scanning for non-UTF-8
///   comment text while still honoring single-byte allowed characters.
pub struct AsciiComments;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MagicEncoding {
    Iso8859_15,
    Other,
}

fn first_non_ascii_utf8_offset(comment_text: &str, allowed_chars: &[String]) -> Option<usize> {
    let after_hash = comment_text.strip_prefix('#').unwrap_or(comment_text);

    for (char_idx, ch) in after_hash.char_indices() {
        if ch.is_ascii() {
            continue;
        }

        let ch_str = ch.to_string();
        if allowed_chars.iter().any(|allowed| allowed == &ch_str) {
            continue;
        }

        return Some(1 + char_idx);
    }

    None
}

fn first_non_ascii_byte_offset(
    comment_bytes: &[u8],
    allowed_chars: &[String],
    encoding: Option<MagicEncoding>,
) -> Option<usize> {
    let mut idx = usize::from(comment_bytes.first() == Some(&b'#'));
    while idx < comment_bytes.len() {
        let byte = comment_bytes[idx];
        if byte.is_ascii() {
            idx += 1;
            continue;
        }

        if is_allowed_non_utf8_byte(byte, allowed_chars, encoding) {
            idx += 1;
            continue;
        }

        return Some(idx);
    }

    None
}

fn is_allowed_non_utf8_byte(
    byte: u8,
    allowed_chars: &[String],
    encoding: Option<MagicEncoding>,
) -> bool {
    allowed_chars.iter().any(|allowed| {
        let mut chars = allowed.chars();
        let ch = match (chars.next(), chars.next()) {
            (Some(ch), None) => ch,
            _ => return false,
        };

        allowed_char_byte(ch, encoding) == Some(byte)
    })
}

fn allowed_char_byte(ch: char, encoding: Option<MagicEncoding>) -> Option<u8> {
    let codepoint = ch as u32;
    if codepoint <= u8::MAX as u32 {
        return Some(codepoint as u8);
    }

    if encoding == Some(MagicEncoding::Iso8859_15) {
        return match ch {
            '€' => Some(0xA4),
            'Š' => Some(0xA6),
            'š' => Some(0xA8),
            'Ž' => Some(0xB4),
            'ž' => Some(0xB8),
            'Œ' => Some(0xBC),
            'œ' => Some(0xBD),
            'Ÿ' => Some(0xBE),
            _ => None,
        };
    }

    None
}

fn detect_magic_encoding(source: &[u8]) -> Option<MagicEncoding> {
    let mut start = 0;
    for _ in 0..3 {
        if start >= source.len() {
            break;
        }

        let end = source[start..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|idx| start + idx)
            .unwrap_or(source.len());
        let line = &source[start..end];
        let lower = String::from_utf8_lossy(line).to_ascii_lowercase();

        if lower.contains("iso-8859-15")
            || lower.contains("iso8859-15")
            || lower.contains("latin-9")
            || lower.contains("latin9")
        {
            return Some(MagicEncoding::Iso8859_15);
        }

        if lower.contains("encoding") || lower.contains("coding") {
            return Some(MagicEncoding::Other);
        }

        start = end + 1;
    }

    None
}

impl Cop for AsciiComments {
    fn name(&self) -> &'static str {
        "Style/AsciiComments"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allowed_chars = config.get_string_array("AllowedChars").unwrap_or_default();
        let bytes = source.as_bytes();
        let encoding = detect_magic_encoding(bytes);

        for comment in parse_result.comments() {
            let loc = comment.location();
            let start = loc.start_offset();
            let end = loc.end_offset();

            // Get the comment text (everything from # to end of comment)
            let comment_bytes = &bytes[start..end];
            let relative_offset = match std::str::from_utf8(comment_bytes) {
                Ok(comment_text) => first_non_ascii_utf8_offset(comment_text, &allowed_chars),
                Err(_) => first_non_ascii_byte_offset(comment_bytes, &allowed_chars, encoding),
            };

            if let Some(relative_offset) = relative_offset {
                let byte_offset = start + relative_offset;
                let (line, col) = source.offset_to_line_col(byte_offset);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    col,
                    "Use only ascii symbols in comments.".to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AsciiComments, "cops/style/ascii_comments");

    #[test]
    fn flags_non_utf8_comment_with_magic_encoding() {
        let diagnostics = crate::testutil::run_cop_full(
            &AsciiComments,
            b"# coding: ISO-8859-15\n# We use \xA4 for default currency symbol\n",
        );

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].location.line, 2);
        assert_eq!(diagnostics[0].location.column, 9);
        assert_eq!(diagnostics[0].cop_name, "Style/AsciiComments");
    }

    #[test]
    fn allows_single_byte_allowed_chars_in_non_utf8_comment() {
        let config = CopConfig {
            options: std::collections::HashMap::from([(
                "AllowedChars".to_string(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("©".to_string())]),
            )]),
            ..CopConfig::default()
        };

        let diagnostics = crate::testutil::run_cop_full_with_config(
            &AsciiComments,
            b"# coding: ISO-8859-15\n# copyright \xA9\n",
            config,
        );

        assert!(
            diagnostics.is_empty(),
            "expected no diagnostics for allowed single-byte chars, got: {diagnostics:?}"
        );
    }
}
