use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::BLOCK_NODE;

pub struct SpaceInsideBlockBraces;

impl Cop for SpaceInsideBlockBraces {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsideBlockBraces"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let block = match node.as_block_node() {
            Some(b) => b,
            None => return,
        };

        let opening = block.opening_loc();
        let closing = block.closing_loc();

        // Only check { } blocks, not do...end
        if opening.as_slice() != b"{" {
            return;
        }

        let bytes = source.as_bytes();
        let open_end = opening.end_offset();
        let close_start = closing.start_offset();

        let empty_style = config.get_str("EnforcedStyleForEmptyBraces", "no_space");
        let space_before_params = config.get_bool("SpaceBeforeBlockParameters", true);

        // Handle empty blocks {}
        if close_start == open_end {
            // Empty block with no space: {}
            match empty_style {
                "space" => {
                    let (line, column) = source.offset_to_line_col(opening.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside empty braces missing.".to_string(),
                    ));
                    return;
                }
                _ => return, // no_space is fine for {}
            }
        }
        // Check for { } (empty with space)
        if close_start == open_end + 1 && bytes.get(open_end) == Some(&b' ') {
            match empty_style {
                "no_space" => {
                    let (line, column) = source.offset_to_line_col(open_end);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside empty braces detected.".to_string(),
                    ));
                    return;
                }
                _ => return, // space is fine for { }
            }
        }

        // SpaceBeforeBlockParameters: check space between { and |
        if !space_before_params {
            if let Some(params) = block.parameters() {
                let pipe_start = params.location().start_offset();
                if pipe_start == open_end + 1 && bytes.get(open_end) == Some(&b' ') {
                    let (line, column) = source.offset_to_line_col(open_end);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Space between { and | detected.".to_string(),
                    ));
                    return;
                }
            }
        }

        // Skip multiline blocks
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
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Space missing inside {.".to_string(),
                    ));
                }
                if !space_before_close {
                    let (line, column) = source.offset_to_line_col(closing.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Space missing inside }.".to_string(),
                    ));
                }
            }
            "no_space" => {
                if space_after_open {
                    let (line, column) = source.offset_to_line_col(open_end);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside { detected.".to_string(),
                    ));
                }
                if space_before_close {
                    let (line, column) = source.offset_to_line_col(close_start - 1);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside } detected.".to_string(),
                    ));
                }
            }
            _ => {}
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceInsideBlockBraces, "cops/layout/space_inside_block_braces");

    #[test]
    fn empty_braces_space_style_flags_no_space() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyleForEmptyBraces".into(), serde_yml::Value::String("space".into())),
            ]),
            ..CopConfig::default()
        };
        let src = b"items.each {}\n";
        let diags = run_cop_full_with_config(&SpaceInsideBlockBraces, src, config);
        assert_eq!(diags.len(), 1, "space style for empty braces should flag braces");
        assert!(diags[0].message.contains("missing"));
    }

    #[test]
    fn space_before_block_params_false_flags_space() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("SpaceBeforeBlockParameters".into(), serde_yml::Value::Bool(false)),
            ]),
            ..CopConfig::default()
        };
        let src = b"items.each { |x| puts x }\n";
        let diags = run_cop_full_with_config(&SpaceInsideBlockBraces, src, config);
        assert!(diags.iter().any(|d| d.message.contains("{ and |")),
            "SpaceBeforeBlockParameters:false should flag space between {{ and |");
    }
}
