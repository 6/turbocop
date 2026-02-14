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

        let width = config.get_usize("IndentationWidth", 2);

        let open_line_bytes = source.lines().nth(open_line - 1).unwrap_or(b"");
        let open_line_indent = indentation_of(open_line_bytes);
        let expected = open_line_indent + width;

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
}
