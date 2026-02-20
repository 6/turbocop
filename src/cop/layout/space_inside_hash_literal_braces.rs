use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::HASH_NODE;

pub struct SpaceInsideHashLiteralBraces;

impl Cop for SpaceInsideHashLiteralBraces {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsideHashLiteralBraces"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[HASH_NODE]
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
        // Note: keyword_hash_node (keyword args like `foo(a: 1)`) intentionally not
        // handled — this cop only applies to hash literals with `{ }` braces.
        let hash = match node.as_hash_node() {
            Some(h) => h,
            None => return,
        };

        let opening = hash.opening_loc();
        let closing = hash.closing_loc();

        // Only check hash literals with { }
        if opening.as_slice() != b"{" || closing.as_slice() != b"}" {
            return;
        }

        let bytes = source.as_bytes();
        let open_end = opening.end_offset();
        let close_start = closing.start_offset();

        let empty_style = config.get_str("EnforcedStyleForEmptyBraces", "no_space");

        // Handle empty hashes {}
        if close_start == open_end {
            match empty_style {
                "space" => {
                    let (line, column) = source.offset_to_line_col(opening.start_offset());
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside empty hash literal braces missing.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: open_end, end: open_end, replacement: " ".to_string(),
                            cop_name: self.name(), cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                    return;
                }
                _ => return,
            }
        }
        // Check for { } (empty with space)
        if close_start == open_end + 1 && bytes.get(open_end) == Some(&b' ') {
            match empty_style {
                "no_space" => {
                    let (line, column) = source.offset_to_line_col(open_end);
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside empty hash literal braces detected.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: open_end, end: open_end + 1, replacement: String::new(),
                            cop_name: self.name(), cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                    return;
                }
                _ => return,
            }
        }

        // Skip multiline hashes (opening and closing on different lines)
        let (open_line, _) = source.offset_to_line_col(opening.start_offset());
        let (close_line, _) = source.offset_to_line_col(closing.start_offset());
        if open_line != close_line {
            return;
        }

        let enforced = config.get_str("EnforcedStyle", "space");


        let space_after_open = bytes.get(open_end) == Some(&b' ');
        let space_before_close = close_start > 0 && bytes.get(close_start - 1) == Some(&b' ');

        match enforced {
            "space" => {
                if !space_after_open {
                    let (line, column) = source.offset_to_line_col(opening.start_offset());
                    let mut diag = self.diagnostic(
                        source, line, column,
                        "Space inside { missing.".to_string(),
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
                        "Space inside } missing.".to_string(),
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
            "no_space" => {
                if space_after_open {
                    let (line, column) = source.offset_to_line_col(open_end);
                    let mut diag = self.diagnostic(
                        source, line, column,
                        "Space inside { detected.".to_string(),
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
                    let (line, column) = source.offset_to_line_col(close_start - 1);
                    let mut diag = self.diagnostic(
                        source, line, column,
                        "Space inside } detected.".to_string(),
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
            _ => {}
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{run_cop_full_with_config, assert_cop_no_offenses_full_with_config};

    crate::cop_fixture_tests!(
        SpaceInsideHashLiteralBraces,
        "cops/layout/space_inside_hash_literal_braces"
    );
    crate::cop_autocorrect_fixture_tests!(
        SpaceInsideHashLiteralBraces,
        "cops/layout/space_inside_hash_literal_braces"
    );

    #[test]
    fn empty_braces_space_style_flags_no_space() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyleForEmptyBraces".into(), serde_yml::Value::String("space".into())),
            ]),
            ..CopConfig::default()
        };
        let source = b"x = {}\n";
        let diags = run_cop_full_with_config(&SpaceInsideHashLiteralBraces, source, config);
        assert_eq!(diags.len(), 1, "space style should flag empty hash without space");
        assert!(diags[0].message.contains("missing"));
    }

    #[test]
    fn config_no_space() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("no_space".into())),
            ]),
            ..CopConfig::default()
        };
        // Hash with spaces should trigger with no_space style
        let source = b"x = { a: 1 }\n";
        let diags = run_cop_full_with_config(&SpaceInsideHashLiteralBraces, source, config.clone());
        assert!(!diags.is_empty(), "Should fire with EnforcedStyle:no_space on spaced hash");

        // Hash without spaces should be clean with no_space style
        let source2 = b"x = {a: 1}\n";
        assert_cop_no_offenses_full_with_config(&SpaceInsideHashLiteralBraces, source2, config);
    }
}
