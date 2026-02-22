use crate::cop::node_type::BLOCK_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SpaceBeforeBlockBraces;

impl Cop for SpaceBeforeBlockBraces {
    fn name(&self) -> &'static str {
        "Layout/SpaceBeforeBlockBraces"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE]
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
        let style = config.get_str("EnforcedStyle", "space");
        let empty_style = config.get_str("EnforcedStyleForEmptyBraces", "space");
        let block = match node.as_block_node() {
            Some(b) => b,
            None => return,
        };

        let opening = block.opening_loc();
        let closing = block.closing_loc();

        // Only check { blocks, not do...end
        if opening.as_slice() != b"{" {
            return;
        }

        let bytes = source.as_bytes();
        let before = opening.start_offset();

        // Check if this is an empty block {}
        let is_empty = closing.start_offset() == opening.end_offset();

        // Use empty_style for empty braces, style for non-empty
        let effective_style = if is_empty { empty_style } else { style };

        match effective_style {
            "no_space" => {
                if before > 0 && bytes[before - 1] == b' ' {
                    let (line, column) = source.offset_to_line_col(before - 1);
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        column,
                        "Space detected to the left of {.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: before - 1,
                            end: before,
                            replacement: String::new(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                    return;
                }
            }
            _ => {
                // "space" (default)
                if before > 0 && bytes[before - 1] != b' ' {
                    let (line, column) = source.offset_to_line_col(before);
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        column,
                        "Space missing to the left of {.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: before,
                            end: before,
                            replacement: " ".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        SpaceBeforeBlockBraces,
        "cops/layout/space_before_block_braces"
    );
    crate::cop_autocorrect_fixture_tests!(
        SpaceBeforeBlockBraces,
        "cops/layout/space_before_block_braces"
    );

    #[test]
    fn no_space_style_flags_space() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("no_space".into()),
            )]),
            ..CopConfig::default()
        };
        let src = b"items.each { |x| puts x }\n";
        let diags = run_cop_full_with_config(&SpaceBeforeBlockBraces, src, config);
        assert_eq!(
            diags.len(),
            1,
            "no_space style should flag space before brace"
        );
        assert!(diags[0].message.contains("detected"));
    }

    #[test]
    fn no_space_style_accepts_no_space() {
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("no_space".into()),
            )]),
            ..CopConfig::default()
        };
        let src = b"items.each{ |x| puts x }\n";
        assert_cop_no_offenses_full_with_config(&SpaceBeforeBlockBraces, src, config);
    }

    #[test]
    fn empty_braces_no_space_style() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyleForEmptyBraces".into(),
                serde_yml::Value::String("no_space".into()),
            )]),
            ..CopConfig::default()
        };
        let src = b"items.each {}\n";
        let diags = run_cop_full_with_config(&SpaceBeforeBlockBraces, src, config);
        assert_eq!(
            diags.len(),
            1,
            "no_space for empty braces should flag space before brace"
        );
    }
}
