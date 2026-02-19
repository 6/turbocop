use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{HASH_NODE, KEYWORD_HASH_NODE};

pub struct FirstHashElementLineBreak;

impl Cop for FirstHashElementLineBreak {
    fn name(&self) -> &'static str {
        "Layout/FirstHashElementLineBreak"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[HASH_NODE, KEYWORD_HASH_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let _allow_multiline_final = config.get_bool("AllowMultilineFinalElement", false);

        // Skip keyword hashes (no braces)
        if node.as_keyword_hash_node().is_some() {
            return Vec::new();
        }

        let hash = match node.as_hash_node() {
            Some(h) => h,
            None => return Vec::new(),
        };

        let opening = hash.opening_loc();
        let closing = hash.closing_loc();

        if opening.as_slice() != b"{" || closing.as_slice() != b"}" {
            return Vec::new();
        }

        let elements: Vec<ruby_prism::Node<'_>> = hash.elements().iter().collect();
        if elements.is_empty() {
            return Vec::new();
        }

        let (open_line, _) = source.offset_to_line_col(opening.start_offset());
        let (close_line, _) = source.offset_to_line_col(closing.start_offset());

        // Only check multiline hashes
        if open_line == close_line {
            return Vec::new();
        }

        let first = &elements[0];
        let (first_line, first_col) = source.offset_to_line_col(first.location().start_offset());

        if first_line == open_line {
            return vec![self.diagnostic(
                source,
                first_line,
                first_col,
                "Add a line break before the first element of a multi-line hash.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        FirstHashElementLineBreak,
        "cops/layout/first_hash_element_line_break"
    );
}
