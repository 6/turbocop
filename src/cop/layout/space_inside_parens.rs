use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SpaceInsideParens;

impl Cop for SpaceInsideParens {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsideParens"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyle", "no_space");
        let parens = match node.as_parentheses_node() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let bytes = source.as_bytes();
        let open_end = parens.opening_loc().end_offset();
        let close_start = parens.closing_loc().start_offset();

        // Skip empty parens ()
        if close_start == open_end {
            return Vec::new();
        }

        let mut diagnostics = Vec::new();

        let space_after_open = bytes.get(open_end) == Some(&b' ');
        let space_before_close = close_start > 0 && bytes.get(close_start - 1) == Some(&b' ');

        // Skip multiline parens for space checks
        let is_multiline_after = bytes.get(open_end) == Some(&b'\n') || bytes.get(open_end) == Some(&b'\r');
        let is_multiline_before = close_start >= 2
            && (bytes.get(close_start - 2) == Some(&b'\n') || bytes.get(close_start - 2) == Some(&b'\r'));

        match style {
            "space" => {
                if !space_after_open && !is_multiline_after {
                    let (line, column) = source.offset_to_line_col(open_end);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside parentheses missing.".to_string(),
                    ));
                }
                if !space_before_close && !is_multiline_before {
                    let (line, column) = source.offset_to_line_col(close_start);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside parentheses missing.".to_string(),
                    ));
                }
            }
            "compact" => {
                // "compact" is like no_space but allows spaces in certain positions.
                // For simplicity, behave like no_space.
                if space_after_open && !is_multiline_after {
                    let (line, column) = source.offset_to_line_col(open_end);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside parentheses detected.".to_string(),
                    ));
                }
                if space_before_close && !is_multiline_before {
                    let (line, column) = source.offset_to_line_col(close_start - 1);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside parentheses detected.".to_string(),
                    ));
                }
            }
            _ => {
                // "no_space" (default)
                if space_after_open && !is_multiline_after {
                    let (line, column) = source.offset_to_line_col(open_end);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside parentheses detected.".to_string(),
                    ));
                }
                if space_before_close && !is_multiline_before {
                    let (line, column) = source.offset_to_line_col(close_start - 1);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside parentheses detected.".to_string(),
                    ));
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceInsideParens, "cops/layout/space_inside_parens");

    #[test]
    fn space_style_flags_missing_spaces() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("space".into())),
            ]),
            ..CopConfig::default()
        };
        let src = b"x = (1 + 2)\n";
        let diags = run_cop_full_with_config(&SpaceInsideParens, src, config);
        assert_eq!(diags.len(), 2, "space style should flag missing spaces inside parens");
        assert!(diags[0].message.contains("missing"));
    }

    #[test]
    fn space_style_accepts_spaces() {
        use std::collections::HashMap;
        use crate::testutil::assert_cop_no_offenses_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("space".into())),
            ]),
            ..CopConfig::default()
        };
        let src = b"x = ( 1 + 2 )\n";
        assert_cop_no_offenses_full_with_config(&SpaceInsideParens, src, config);
    }
}
