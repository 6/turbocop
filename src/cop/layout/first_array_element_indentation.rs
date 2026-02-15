use crate::cop::util::indentation_of;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FirstArrayElementIndentation;

impl Cop for FirstArrayElementIndentation {
    fn name(&self) -> &'static str {
        "Layout/FirstArrayElementIndentation"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let array_node = match node.as_array_node() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let opening_loc = match array_node.opening_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let elements: Vec<_> = array_node.elements().iter().collect();
        if elements.is_empty() {
            return Vec::new();
        }

        let first_element = &elements[0];

        let (open_line, _) = source.offset_to_line_col(opening_loc.start_offset());
        let first_loc = first_element.location();
        let (elem_line, elem_col) = source.offset_to_line_col(first_loc.start_offset());

        // Skip if first element is on same line as opening bracket
        if elem_line == open_line {
            return Vec::new();
        }

        let style = config.get_str("EnforcedStyle", "special_inside_parentheses");
        let width = config.get_usize("IndentationWidth", 2);

        // Get the indentation of the line where `[` appears
        let open_line_bytes = source.lines().nth(open_line - 1).unwrap_or(b"");
        let open_line_indent = indentation_of(open_line_bytes);
        let (_, open_col) = source.offset_to_line_col(opening_loc.start_offset());

        let expected = match style {
            "consistent" => open_line_indent + width,
            "align_brackets" => open_col,
            _ => {
                // "special_inside_parentheses" (default): indent relative to line start
                open_line_indent + width
            }
        };

        if elem_col != expected {
            return vec![self.diagnostic(
                source,
                elem_line,
                elem_col,
                format!(
                    "Use {} (not {}) spaces for indentation of the first element.",
                    width,
                    elem_col.saturating_sub(open_line_indent)
                ),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        FirstArrayElementIndentation,
        "cops/layout/first_array_element_indentation"
    );

    #[test]
    fn same_line_elements_ignored() {
        let source = b"x = [1, 2, 3]\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn align_brackets_style() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("align_brackets".into())),
            ]),
            ..CopConfig::default()
        };
        // Element aligned with opening bracket column
        let src = b"x = [\n    1\n]\n";
        let diags = run_cop_full_with_config(&FirstArrayElementIndentation, src, config.clone());
        assert!(diags.is_empty(), "align_brackets should accept element at bracket column");

        // Element indented normally (2 from line start) should be flagged
        let src2 = b"x = [\n  1\n]\n";
        let diags2 = run_cop_full_with_config(&FirstArrayElementIndentation, src2, config);
        assert_eq!(diags2.len(), 1, "align_brackets should flag element not at bracket column");
    }
}
