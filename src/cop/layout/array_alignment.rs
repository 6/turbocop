use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::ARRAY_NODE;

pub struct ArrayAlignment;

/// Returns true if the byte at `offset` is the first non-whitespace character on its line.
fn begins_its_line(source: &SourceFile, offset: usize) -> bool {
    let (line, col) = source.offset_to_line_col(offset);
    if col == 0 {
        return true;
    }
    let line_bytes = source.lines().nth(line - 1).unwrap_or(b"");
    line_bytes[..col].iter().all(|&b| b == b' ' || b == b'\t')
}

impl Cop for ArrayAlignment {
    fn name(&self) -> &'static str {
        "Layout/ArrayAlignment"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE]
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
        let style = config.get_str("EnforcedStyle", "with_first_element");
        let indent_width = config.get_usize("IndentationWidth", 2);
        let array_node = match node.as_array_node() {
            Some(a) => a,
            None => return,
        };

        let elements = array_node.elements();
        if elements.len() < 2 {
            return;
        }

        let first = match elements.iter().next() {
            Some(e) => e,
            None => return,
        };
        let (first_line, first_col) = source.offset_to_line_col(first.location().start_offset());

        // For "with_fixed_indentation", expected column is array line indent + indent_width
        let expected_col = match style {
            "with_fixed_indentation" => {
                let open_loc = array_node.opening_loc().unwrap_or(first.location());
                let (open_line, _) = source.offset_to_line_col(open_loc.start_offset());
                let open_line_bytes = source.lines().nth(open_line - 1).unwrap_or(b"");
                crate::cop::util::indentation_of(open_line_bytes) + indent_width
            }
            _ => first_col, // "with_first_element" (default)
        };

        let mut last_checked_line = first_line;

        for elem in elements.iter().skip(1) {
            let start_offset = elem.location().start_offset();
            let (elem_line, elem_col) = source.offset_to_line_col(start_offset);
            // Only check the first element on each new line; subsequent elements
            // on the same line are just comma-separated and not alignment targets.
            if elem_line == last_checked_line {
                continue;
            }
            last_checked_line = elem_line;
            // Skip elements that are not the first non-whitespace token on their line.
            // E.g. in `}, {` the `{` follows a `}` and should not be checked.
            if !begins_its_line(source, start_offset) {
                continue;
            }
            if elem_col != expected_col {
                diagnostics.push(self.diagnostic(
                    source,
                    elem_line,
                    elem_col,
                    "Align the elements of an array literal if they span more than one line."
                        .to_string(),
                ));
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(ArrayAlignment, "cops/layout/array_alignment");

    #[test]
    fn single_line_array_no_offense() {
        let source = b"x = [1, 2, 3]\n";
        let diags = run_cop_full(&ArrayAlignment, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn with_fixed_indentation_style() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("with_fixed_indentation".into())),
            ]),
            ..CopConfig::default()
        };
        // Elements at fixed indentation (2 spaces) should be accepted
        let src = b"x = [\n  1,\n  2\n]\n";
        let diags = run_cop_full_with_config(&ArrayAlignment, src, config.clone());
        assert!(diags.is_empty(), "with_fixed_indentation should accept 2-space indent");

        // Elements aligned with first element at column 4 should be flagged
        let src2 = b"x = [1,\n     2]\n";
        let diags2 = run_cop_full_with_config(&ArrayAlignment, src2, config);
        assert_eq!(diags2.len(), 1, "with_fixed_indentation should flag first-element alignment");
    }
}
