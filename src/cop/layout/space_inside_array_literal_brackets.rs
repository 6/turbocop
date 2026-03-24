use crate::cop::node_type::ARRAY_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SpaceInsideArrayLiteralBrackets;

impl Cop for SpaceInsideArrayLiteralBrackets {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsideArrayLiteralBrackets"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let array = match node.as_array_node() {
            Some(a) => a,
            None => return,
        };

        let opening = match array.opening_loc() {
            Some(loc) => loc,
            None => return, // Implicit array (no brackets)
        };
        let closing = match array.closing_loc() {
            Some(loc) => loc,
            None => return,
        };

        // Only check [ ] arrays
        if opening.as_slice() != b"[" || closing.as_slice() != b"]" {
            return;
        }

        let bytes = source.as_bytes();
        let open_end = opening.end_offset();
        let close_start = closing.start_offset();

        let empty_style = config.get_str("EnforcedStyleForEmptyBrackets", "no_space");

        // Handle empty arrays []
        if close_start == open_end {
            match empty_style {
                "space" => {
                    let (line, column) = source.offset_to_line_col(opening.start_offset());
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside empty array literal brackets missing.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: open_end,
                            end: open_end,
                            replacement: " ".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
                _ => return,
            }
        }
        // Check for [ ] (empty with space)
        if close_start == open_end + 1 && bytes.get(open_end) == Some(&b' ') {
            match empty_style {
                "no_space" => {
                    let (line, column) = source.offset_to_line_col(open_end);
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside empty array literal brackets detected.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: open_end,
                            end: open_end + 1,
                            replacement: String::new(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
                _ => return,
            }
        }

        let enforced = config.get_str("EnforcedStyle", "no_space");

        // For multiline arrays, determine which bracket sides to skip.
        // RuboCop uses start_ok/end_ok:
        // - For no_space: start_ok if next token after [ is a comment
        // - For space: start_ok if next non-whitespace after [ is on a different line
        // - end_ok: if ] begins its own line (only whitespace before it)
        let (open_line, _) = source.offset_to_line_col(opening.start_offset());
        let (close_line, _) = source.offset_to_line_col(closing.start_offset());
        let is_multiline = open_line != close_line;

        let start_ok = if is_multiline {
            match enforced {
                "no_space" => next_to_comment(bytes, open_end),
                _ => next_to_newline(bytes, open_end),
            }
        } else {
            false
        };

        let end_ok = if is_multiline {
            // ] begins its line: only whitespace before it on the same line
            begins_its_line_raw(bytes, close_start)
        } else {
            false
        };

        let space_after_open = bytes.get(open_end) == Some(&b' ');
        let space_before_close = close_start > 0 && bytes.get(close_start - 1) == Some(&b' ');

        match enforced {
            "no_space" => {
                if !start_ok && space_after_open {
                    let (line, column) = source.offset_to_line_col(opening.start_offset());
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside array literal brackets detected.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: open_end,
                            end: open_end + 1,
                            replacement: String::new(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
                if !end_ok && space_before_close {
                    let (line, column) = source.offset_to_line_col(closing.start_offset());
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside array literal brackets detected.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: close_start - 1,
                            end: close_start,
                            replacement: String::new(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
            }
            "space" => {
                if !start_ok && !space_after_open {
                    let (line, column) = source.offset_to_line_col(opening.start_offset());
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside array literal brackets missing.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: open_end,
                            end: open_end,
                            replacement: " ".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
                if !end_ok && !space_before_close {
                    let (line, column) = source.offset_to_line_col(closing.start_offset());
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside array literal brackets missing.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: close_start,
                            end: close_start,
                            replacement: " ".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
            }
            _ => {}
        }
    }
}

/// Check if the next non-whitespace character after `pos` is on a different line.
/// Used for `space` style to skip the opening bracket check when elements
/// start on the next line.
fn next_to_newline(bytes: &[u8], pos: usize) -> bool {
    let mut i = pos;
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' => i += 1,
            b'\n' | b'\r' => return true,
            _ => return false,
        }
    }
    true // end of file
}

/// Check if the next non-whitespace character after `pos` is a `#` comment.
/// Used for `no_space` style to allow `[ # comment\n  ...]`.
fn next_to_comment(bytes: &[u8], pos: usize) -> bool {
    let mut i = pos;
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' => i += 1,
            b'#' => return true,
            _ => return false,
        }
    }
    false
}

/// Check if the position is the first non-whitespace on its line (raw byte scan).
/// Equivalent to `util::begins_its_line` but works on raw bytes without SourceFile.
fn begins_its_line_raw(bytes: &[u8], pos: usize) -> bool {
    if pos == 0 {
        return true;
    }
    let mut i = pos - 1;
    loop {
        match bytes[i] {
            b' ' | b'\t' => {
                if i == 0 {
                    return true;
                }
                i -= 1;
            }
            b'\n' => return true,
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        SpaceInsideArrayLiteralBrackets,
        "cops/layout/space_inside_array_literal_brackets"
    );
    crate::cop_autocorrect_fixture_tests!(
        SpaceInsideArrayLiteralBrackets,
        "cops/layout/space_inside_array_literal_brackets"
    );

    #[test]
    fn empty_brackets_space_style_flags_no_space() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyleForEmptyBrackets".into(),
                serde_yml::Value::String("space".into()),
            )]),
            ..CopConfig::default()
        };
        let src = b"x = []\n";
        let diags = run_cop_full_with_config(&SpaceInsideArrayLiteralBrackets, src, config);
        assert_eq!(
            diags.len(),
            1,
            "space style should flag empty [] without space"
        );
    }

    #[test]
    fn empty_brackets_no_space_is_default() {
        use crate::testutil::run_cop_full;

        let src = b"x = []\n";
        let diags = run_cop_full(&SpaceInsideArrayLiteralBrackets, src);
        assert!(diags.is_empty(), "Default no_space should accept []");
    }
}
