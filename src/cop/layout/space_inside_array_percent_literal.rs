use crate::cop::shared::node_type::ARRAY_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks for unnecessary additional spaces inside array percent literals (%w, %W, %i, %I).
///
/// Root cause of 79 FNs: original implementation skipped multiline percent literals entirely
/// (`open_line != close_line` early return). Fix: removed the multiline skip and scan each
/// line of the content individually for double spaces between elements, matching RuboCop's
/// `MULTIPLE_SPACES_BETWEEN_ITEMS_REGEX` behavior. Leading/trailing spaces on each line are
/// ignored (only mid-line double spaces between items are flagged).
///
/// Root cause of 61 FPs: tab-indented continuation lines inside `%w()` had their leading
/// whitespace (tabs + spaces) incorrectly flagged as double spaces between items. The
/// `space_start > 0` check was insufficient because it only verified a non-zero position,
/// not that the preceding character was a non-whitespace item. Fix: also check that the
/// character before the spaces is not a space or tab, matching RuboCop's `\S\s{2,}\S` regex
/// which requires non-space characters on both sides.
pub struct SpaceInsideArrayPercentLiteral;

impl Cop for SpaceInsideArrayPercentLiteral {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsideArrayPercentLiteral"
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
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let array = match node.as_array_node() {
            Some(a) => a,
            None => return,
        };

        let open_loc = match array.opening_loc() {
            Some(loc) => loc,
            None => return,
        };

        let close_loc = match array.closing_loc() {
            Some(loc) => loc,
            None => return,
        };

        let open_slice = open_loc.as_slice();
        // Only percent array literals
        if !open_slice.starts_with(b"%w")
            && !open_slice.starts_with(b"%W")
            && !open_slice.starts_with(b"%i")
            && !open_slice.starts_with(b"%I")
        {
            return;
        }

        let bytes = source.as_bytes();
        let open_end = open_loc.end_offset();
        let close_start = close_loc.start_offset();

        if close_start <= open_end {
            return;
        }

        let content = &bytes[open_end..close_start];

        // Scan each line of the content for multiple consecutive spaces
        // between non-space, non-backslash characters (matching RuboCop's
        // MULTIPLE_SPACES_BETWEEN_ITEMS_REGEX behavior).
        for line_bytes in content.split(|&b| b == b'\n') {
            let mut i = 0;
            while i < line_bytes.len() {
                if line_bytes[i] == b' ' {
                    let space_start = i;
                    while i < line_bytes.len() && line_bytes[i] == b' ' {
                        i += 1;
                    }
                    let space_len = i - space_start;
                    // Multiple spaces between items (not leading/trailing on the line)
                    if space_len >= 2 && space_start > 0 && i < line_bytes.len() {
                        // Check that character before spaces is not escaped and is not
                        // whitespace (leading indentation on continuation lines)
                        let prev_char = line_bytes[space_start - 1];
                        if prev_char != b'\\' && prev_char != b' ' && prev_char != b'\t' {
                            // Compute offset within content: find where this line starts
                            let line_start_in_content =
                                line_bytes.as_ptr() as usize - content.as_ptr() as usize;
                            let offset = open_end + line_start_in_content + space_start;
                            let (line, col) = source.offset_to_line_col(offset);
                            let mut diag = self.diagnostic(
                                source,
                                line,
                                col,
                                "Use only a single space inside array percent literal.".to_string(),
                            );
                            if let Some(ref mut corr) = corrections {
                                corr.push(crate::correction::Correction {
                                    start: offset,
                                    end: offset + space_len,
                                    replacement: " ".to_string(),
                                    cop_name: self.name(),
                                    cop_index: 0,
                                });
                                diag.corrected = true;
                            }
                            diagnostics.push(diag);
                        }
                    }
                } else {
                    i += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        SpaceInsideArrayPercentLiteral,
        "cops/layout/space_inside_array_percent_literal"
    );
    crate::cop_autocorrect_fixture_tests!(
        SpaceInsideArrayPercentLiteral,
        "cops/layout/space_inside_array_percent_literal"
    );

    #[test]
    fn multiline_large_percent_w() {
        let source = b"%w(
  aget alength all-ns alter and append-child apply array-map
  contains? count create-ns create-struct cycle dec  deref
  str string?  struct struct-map subs subvec symbol symbol?
)
";
        let diags = crate::testutil::run_cop_full(&SpaceInsideArrayPercentLiteral, source);
        // Should detect double space in "dec  deref" and "string?  struct"
        assert_eq!(diags.len(), 2, "Expected 2 offenses, got: {diags:?}");
    }

    #[test]
    fn multiline_with_leading_spaces() {
        // Reproduces corpus pattern: %w() with large leading whitespace
        let source = b"        words = %w(
          aget alength all-ns alter and append-child apply array-map
          bit-xor boolean branch?  butlast byte cast char children
          contains? count create-ns create-struct cycle dec  deref
        )
";
        let diags = crate::testutil::run_cop_full(&SpaceInsideArrayPercentLiteral, source);
        assert_eq!(
            diags.len(),
            2,
            "Expected 2 offenses for branch?__butlast and dec__deref, got: {diags:?}"
        );
    }
}
