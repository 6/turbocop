use crate::cop::shared::node_type::{HASH_NODE, KEYWORD_HASH_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

use super::multiline_literal_brace_layout::{self, BracePositions, HASH_BRACE};

/// Layout/MultilineHashBraceLayout
///
/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=0, FN=2.
///
/// FP=0: no corpus false positives are currently known.
///
/// FN=2:
/// - `elastic/elasticsearch-ruby`: the outer hash had a heredoc in an earlier
///   element, but the last element was a normal hash pair. RuboCop still checks
///   brace layout there; only a heredoc in the last element forces the closing
///   brace placement. Fixed by narrowing the heredoc skip to the last element.
/// - `peritor/webistrano`: the remaining FN is a commented-out snippet that has
///   not reproduced locally as a normal AST-based offense. Leave it for future
///   investigation if it persists after the next corpus oracle run.
pub struct MultilineHashBraceLayout;

impl Cop for MultilineHashBraceLayout {
    fn name(&self) -> &'static str {
        "Layout/MultilineHashBraceLayout"
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
        let enforced_style = config.get_str("EnforcedStyle", "symmetrical");

        // KeywordHashNode (keyword args `foo(a: 1)`) has no braces — skip
        if node.as_keyword_hash_node().is_some() {
            return;
        }

        let hash = match node.as_hash_node() {
            Some(h) => h,
            None => return,
        };

        let opening = hash.opening_loc();
        let closing = hash.closing_loc();

        // Only check brace hashes
        if opening.as_slice() != b"{" || closing.as_slice() != b"}" {
            return;
        }

        let elements = hash.elements();
        if elements.is_empty() {
            return;
        }

        let last_elem = elements.iter().last().unwrap();

        // Only the last element can force the closing brace to move because of
        // its heredoc terminator. Earlier heredocs do not exempt the hash.
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
            &HASH_BRACE,
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
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        MultilineHashBraceLayout,
        "cops/layout/multiline_hash_brace_layout"
    );

    #[test]
    fn earlier_heredoc_still_checks_closing_brace() {
        let source = br#"config = { subject: <<~BODY,
             body line
           BODY
           attachment: "report.yml"
}
"#;
        let diagnostics = run_cop_full(&MultilineHashBraceLayout, source);
        assert_eq!(
            diagnostics.len(),
            1,
            "Expected one offense: {diagnostics:?}"
        );
    }
}
