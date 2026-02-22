use crate::cop::node_type::{CALL_NODE, PARENTHESES_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SpaceInsideParens;

impl Cop for SpaceInsideParens {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsideParens"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, PARENTHESES_NODE]
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
        let style = config.get_str("EnforcedStyle", "no_space");

        // Extract open/close offsets from either ParenthesesNode or CallNode
        let (open_end, close_start) = if let Some(parens) = node.as_parentheses_node() {
            (
                parens.opening_loc().end_offset(),
                parens.closing_loc().start_offset(),
            )
        } else if let Some(call) = node.as_call_node() {
            // CallNode: opening_loc and closing_loc are for argument parens
            let open = match call.opening_loc() {
                Some(loc) if loc.as_slice() == b"(" => loc,
                _ => return,
            };
            let close = match call.closing_loc() {
                Some(loc) if loc.as_slice() == b")" => loc,
                _ => return,
            };
            (open.end_offset(), close.start_offset())
        } else {
            return;
        };

        let bytes = source.as_bytes();

        // Skip empty parens ()
        if close_start == open_end {
            return;
        }

        let space_after_open = bytes.get(open_end) == Some(&b' ');
        let space_before_close = close_start > 0 && bytes.get(close_start - 1) == Some(&b' ');

        // Skip multiline parens for space checks — when the opening and closing
        // are on different lines, spaces adjacent to parens are just indentation.
        let (open_line, _) = source.offset_to_line_col(open_end.saturating_sub(1));
        let (close_line, _) = source.offset_to_line_col(close_start);
        let is_multiline = open_line != close_line;
        let is_multiline_after = is_multiline;
        let is_multiline_before = is_multiline;

        match style {
            "space" => {
                if !space_after_open && !is_multiline_after {
                    let (line, column) = source.offset_to_line_col(open_end);
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside parentheses missing.".to_string(),
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
                if !space_before_close && !is_multiline_before {
                    let (line, column) = source.offset_to_line_col(close_start);
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside parentheses missing.".to_string(),
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
            "compact" | _ => {
                // "no_space" (default) and "compact"
                if space_after_open && !is_multiline_after {
                    let (line, column) = source.offset_to_line_col(open_end);
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside parentheses detected.".to_string(),
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
                if space_before_close && !is_multiline_before {
                    let (line, column) = source.offset_to_line_col(close_start - 1);
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside parentheses detected.".to_string(),
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceInsideParens, "cops/layout/space_inside_parens");
    crate::cop_autocorrect_fixture_tests!(SpaceInsideParens, "cops/layout/space_inside_parens");

    #[test]
    fn space_style_flags_missing_spaces() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("space".into()),
            )]),
            ..CopConfig::default()
        };
        let src = b"x = (1 + 2)\n";
        let diags = run_cop_full_with_config(&SpaceInsideParens, src, config);
        assert_eq!(
            diags.len(),
            2,
            "space style should flag missing spaces inside parens"
        );
        assert!(diags[0].message.contains("missing"));
    }

    #[test]
    fn space_style_accepts_spaces() {
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("space".into()),
            )]),
            ..CopConfig::default()
        };
        let src = b"x = ( 1 + 2 )\n";
        assert_cop_no_offenses_full_with_config(&SpaceInsideParens, src, config);
    }
}
