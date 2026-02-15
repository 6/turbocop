use crate::cop::util::indentation_of;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FirstHashElementIndentation;

impl Cop for FirstHashElementIndentation {
    fn name(&self) -> &'static str {
        "Layout/FirstHashElementIndentation"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Note: keyword_hash_node (keyword args like `foo(a: 1)`) intentionally not
        // handled — this cop only checks indentation relative to `{` braces in hash literals.
        let hash_node = match node.as_hash_node() {
            Some(h) => h,
            None => return Vec::new(),
        };

        let opening_loc = hash_node.opening_loc();

        // Skip implicit hashes (no literal { })
        if opening_loc.as_slice() != b"{" {
            return Vec::new();
        }

        let elements: Vec<_> = hash_node.elements().iter().collect();
        if elements.is_empty() {
            return Vec::new();
        }

        let first_element = &elements[0];

        let (open_line, _) = source.offset_to_line_col(opening_loc.start_offset());
        let first_loc = first_element.location();
        let (elem_line, elem_col) = source.offset_to_line_col(first_loc.start_offset());

        // Skip if first element is on same line as opening brace
        if elem_line == open_line {
            return Vec::new();
        }

        let style = config.get_str("EnforcedStyle", "special_inside_parentheses");
        let width = config.get_usize("IndentationWidth", 2);

        let open_line_bytes = source.lines().nth(open_line - 1).unwrap_or(b"");
        let open_line_indent = indentation_of(open_line_bytes);
        let (_, open_col) = source.offset_to_line_col(opening_loc.start_offset());

        let expected = match style {
            "consistent" => open_line_indent + width,
            "align_braces" => open_col,
            _ => {
                // "special_inside_parentheses" (default)
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
        FirstHashElementIndentation,
        "cops/layout/first_hash_element_indentation"
    );

    #[test]
    fn same_line_elements_ignored() {
        let source = b"x = { a: 1, b: 2 }\n";
        let diags = run_cop_full(&FirstHashElementIndentation, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn align_braces_style() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("align_braces".into())),
            ]),
            ..CopConfig::default()
        };
        // Element aligned with opening brace column (4)
        let src = b"x = {\n    a: 1\n}\n";
        let diags = run_cop_full_with_config(&FirstHashElementIndentation, src, config.clone());
        assert!(diags.is_empty(), "align_braces should accept element at brace column");

        // Element at indentation 2 should be flagged (brace is at col 4)
        let src2 = b"x = {\n  a: 1\n}\n";
        let diags2 = run_cop_full_with_config(&FirstHashElementIndentation, src2, config);
        assert_eq!(diags2.len(), 1, "align_braces should flag element not at brace column");
    }
}
