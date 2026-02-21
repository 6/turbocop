use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::ARRAY_NODE;

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
                        source, line, column,
                        "Space inside empty array literal brackets missing.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: open_end, end: open_end, replacement: " ".to_string(),
                            cop_name: self.name(), cop_index: 0,
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
                        source, line, column,
                        "Space inside empty array literal brackets detected.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: open_end, end: open_end + 1, replacement: String::new(),
                            cop_name: self.name(), cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
                _ => return,
            }
        }

        // Skip multiline arrays
        let (open_line, _) = source.offset_to_line_col(opening.start_offset());
        let (close_line, _) = source.offset_to_line_col(closing.start_offset());
        if open_line != close_line {
            return;
        }

        let enforced = config.get_str("EnforcedStyle", "no_space");


        let space_after_open = bytes.get(open_end) == Some(&b' ');
        let space_before_close = close_start > 0 && bytes.get(close_start - 1) == Some(&b' ');

        match enforced {
            "no_space" => {
                if space_after_open {
                    let (line, column) = source.offset_to_line_col(opening.start_offset());
                    let mut diag = self.diagnostic(
                        source, line, column,
                        "Space inside array literal brackets detected.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: open_end, end: open_end + 1, replacement: String::new(),
                            cop_name: self.name(), cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
                if space_before_close {
                    let (line, column) = source.offset_to_line_col(closing.start_offset());
                    let mut diag = self.diagnostic(
                        source, line, column,
                        "Space inside array literal brackets detected.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: close_start - 1, end: close_start, replacement: String::new(),
                            cop_name: self.name(), cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
            }
            "space" => {
                if !space_after_open {
                    let (line, column) = source.offset_to_line_col(opening.start_offset());
                    let mut diag = self.diagnostic(
                        source, line, column,
                        "Space inside array literal brackets missing.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: open_end, end: open_end, replacement: " ".to_string(),
                            cop_name: self.name(), cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
                if !space_before_close {
                    let (line, column) = source.offset_to_line_col(closing.start_offset());
                    let mut diag = self.diagnostic(
                        source, line, column,
                        "Space inside array literal brackets missing.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: close_start, end: close_start, replacement: " ".to_string(),
                            cop_name: self.name(), cop_index: 0,
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
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyleForEmptyBrackets".into(), serde_yml::Value::String("space".into())),
            ]),
            ..CopConfig::default()
        };
        let src = b"x = []\n";
        let diags = run_cop_full_with_config(&SpaceInsideArrayLiteralBrackets, src, config);
        assert_eq!(diags.len(), 1, "space style should flag empty [] without space");
    }

    #[test]
    fn empty_brackets_no_space_is_default() {
        use crate::testutil::run_cop_full;

        let src = b"x = []\n";
        let diags = run_cop_full(&SpaceInsideArrayLiteralBrackets, src);
        assert!(diags.is_empty(), "Default no_space should accept []");
    }
}
