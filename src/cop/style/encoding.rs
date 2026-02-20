use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct Encoding;

impl Cop for Encoding {
    fn name(&self) -> &'static str {
        "Style/Encoding"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig, diagnostics: &mut Vec<Diagnostic>, mut corrections: Option<&mut Vec<crate::correction::Correction>>) {
        let total_len = source.as_bytes().len();
        let mut byte_offset: usize = 0;

        // Only check the first 3 lines (line 1, optional shebang pushes encoding to line 2,
        // and possibly line 3 with multiple magic comments)
        for (i, line) in source.lines().enumerate() {
            let line_len = line.len() + 1; // +1 for newline
            if i >= 3 {
                break;
            }

            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s.trim(),
                Err(_) => {
                    byte_offset += line_len;
                    continue;
                }
            };

            // Skip non-comment lines (but allow shebang on first line)
            if !line_str.starts_with('#') {
                // If it's the first line and a shebang, or blank, continue
                if i == 0 && line_str.starts_with("#!") {
                    byte_offset += line_len;
                    continue;
                }
                // If it's a code line, stop checking (encoding must be at the top)
                if !line_str.is_empty() {
                    break;
                }
                // Blank line: encoding comment is only valid in the first 2 lines
                // (or first 3 if there's a shebang). A blank line means we've left
                // the magic comment area.
                break;
            }

            // Check for various encoding comment formats
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

/// Check if a comment line is a UTF-8 encoding magic comment.
fn is_utf8_encoding_comment(line: &str) -> bool {
    let lower = line.to_lowercase();

    // Standard magic comment formats:
    // # encoding: utf-8
    // # coding: utf-8
    // # -*- encoding: utf-8 -*-
    // # -*- coding: utf-8 -*-
    // # vim:fileencoding=utf-8
    // # vim: fileencoding=utf-8

    // Check for standard Ruby encoding/coding magic comment
    if let Some(rest) = lower.strip_prefix("# ").or_else(|| lower.strip_prefix("#")) {
        let rest = rest.trim();

        // Emacs style: -*- encoding: utf-8 -*- or -*- coding: utf-8 -*-
        if rest.starts_with("-*-") && rest.ends_with("-*-") {
            let inner = &rest[3..rest.len() - 3].trim();
            // Check if it contains encoding or coding with utf-8
            let inner_lower = inner.to_lowercase();
            if (inner_lower.contains("encoding") || inner_lower.contains("coding"))
                && contains_utf8(&inner_lower)
            {
                return true;
            }
            return false;
        }

        // vim style: vim:fileencoding=utf-8 or vim: fileencoding=utf-8
        if rest.starts_with("vim:") || rest.starts_with("vim :") {
            if contains_utf8(&lower) {
                return true;
            }
            return false;
        }

        // Standard format: encoding: utf-8, coding: utf-8
        // Also handle: Encoding: UTF-8
        if let Some(after) = strip_encoding_prefix(rest) {
            let value = after.trim();
            if value.eq_ignore_ascii_case("utf-8") {
                return true;
            }
        }
    }

    false
}

/// Strip the encoding/coding prefix and colon, returning the value part.
fn strip_encoding_prefix(s: &str) -> Option<&str> {
    let lower = s.to_lowercase();
    for prefix in &["encoding:", "coding:"] {
        if lower.starts_with(prefix) {
            return Some(&s[prefix.len()..]);
        }
    }
    None
}

/// Check if a string contains "utf-8" (case insensitive).
fn contains_utf8(s: &str) -> bool {
    s.contains("utf-8")
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_scenario_fixture_tests!(
        Encoding, "cops/style/encoding",
        standard = "standard.rb",
        mixed_case = "mixed_case.rb",
        after_shebang = "after_shebang.rb",
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
}
