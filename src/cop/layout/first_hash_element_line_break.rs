use crate::cop::shared::node_type::{HASH_NODE, KEYWORD_HASH_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// FP=30 investigation: All 30 false positives were from hashes where all elements
/// fit on one line but the closing `}` wraps to the next line. RuboCop's
/// `FirstElementLineBreak#check_children_line_break` has `return if line == max_line`,
/// skipping when the first element's line equals the last element's last_line.
/// Fix: added the same check — skip when first and last elements are on the same line.
pub struct FirstHashElementLineBreak;

impl Cop for FirstHashElementLineBreak {
    fn name(&self) -> &'static str {
        "Layout/FirstHashElementLineBreak"
    }

    fn default_enabled(&self) -> bool {
        false
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
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let _allow_multiline_final = config.get_bool("AllowMultilineFinalElement", false);

        // Skip keyword hashes (no braces)
        if node.as_keyword_hash_node().is_some() {
            return;
        }

        let hash = match node.as_hash_node() {
            Some(h) => h,
            None => return,
        };

        let opening = hash.opening_loc();
        let closing = hash.closing_loc();

        if opening.as_slice() != b"{" || closing.as_slice() != b"}" {
            return;
        }

        let elements: Vec<ruby_prism::Node<'_>> = hash.elements().iter().collect();
        if elements.is_empty() {
            return;
        }

        let (open_line, _) = source.offset_to_line_col(opening.start_offset());
        let (close_line, _) = source.offset_to_line_col(closing.start_offset());

        // Only check multiline hashes
        if open_line == close_line {
            return;
        }

        let first = &elements[0];
        let (first_line, first_col) = source.offset_to_line_col(first.location().start_offset());

        // RuboCop skips when all elements end on the same line as the opening brace
        // (only the closing brace wraps to a new line)
        let last = elements.last().unwrap();
        let (last_elem_line, _) =
            source.offset_to_line_col(last.location().end_offset().saturating_sub(1));
        if first_line == last_elem_line {
            return;
        }

        if first_line == open_line {
            diagnostics.push(self.diagnostic(
                source,
                first_line,
                first_col,
                "Add a line break before the first element of a multi-line hash.".to_string(),
            ));
        }
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
