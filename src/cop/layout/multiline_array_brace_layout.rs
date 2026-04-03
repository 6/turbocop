use crate::cop::shared::node_type::ARRAY_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

use super::multiline_literal_brace_layout::{self, ARRAY_BRACE, BracePositions};

/// Layout/MultilineArrayBraceLayout
///
/// ## Investigation findings
///
/// **FN root cause (83 FNs):** The cop only handled `[...]` bracket arrays,
/// skipping percent literal arrays (`%w()`, `%i()`, `%W()`, `%I()`, etc.).
/// Many corpus repos (especially devdocs) use `%w(` with the closing `)` on
/// the same line as the last element when the opening is on a separate line.
/// Fixed by removing the `[`/`]` check and accepting all explicit array types.
///
/// **FP root cause (3 FPs):** Arrays containing heredocs in the last element
/// (e.g., `[<<~MSG, ...]`) have a Prism end_offset on the opening delimiter
/// line, not the heredoc body end. This made the cop think the closing brace
/// was on a different line than the last element. RuboCop skips arrays where
/// the last element contains a heredoc. Fixed by adding `contains_heredoc`
/// check matching the sibling `MultilineHashBraceLayout` cop.
pub struct MultilineArrayBraceLayout;

impl Cop for MultilineArrayBraceLayout {
    fn name(&self) -> &'static str {
        "Layout/MultilineArrayBraceLayout"
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
        let enforced_style = config.get_str("EnforcedStyle", "symmetrical");

        let array = match node.as_array_node() {
            Some(a) => a,
            None => return,
        };

        // Implicit arrays have no opening/closing braces
        let opening = match array.opening_loc() {
            Some(loc) => loc,
            None => return,
        };
        let closing = match array.closing_loc() {
            Some(loc) => loc,
            None => return,
        };

        let elements = array.elements();
        if elements.is_empty() {
            return;
        }

        let last_elem = elements.iter().last().unwrap();

        // Skip arrays where the last element contains a heredoc — the heredoc
        // body forces the closing brace placement and adjusting it would
        // produce invalid code.
        if multiline_literal_brace_layout::contains_heredoc(&last_elem) {
            return;
        }

        let (open_line, _) = source.offset_to_line_col(opening.start_offset());
        let (close_line, close_col) = source.offset_to_line_col(closing.start_offset());

        let first_elem = elements.iter().next().unwrap();
        let (first_elem_line, _) = source.offset_to_line_col(first_elem.location().start_offset());
        let (last_elem_line, _) =
            source.offset_to_line_col(last_elem.location().end_offset().saturating_sub(1));

        multiline_literal_brace_layout::check_brace_layout(
            self,
            source,
            enforced_style,
            &ARRAY_BRACE,
            &BracePositions {
                open_line,
                close_line,
                close_col,
                first_elem_line,
                last_elem_line,
            },
            diagnostics,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        MultilineArrayBraceLayout,
        "cops/layout/multiline_array_brace_layout"
    );
}
