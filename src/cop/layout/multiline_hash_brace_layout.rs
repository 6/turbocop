use crate::cop::node_type::{HASH_NODE, KEYWORD_HASH_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

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

        let (open_line, _) = source.offset_to_line_col(opening.start_offset());
        let (close_line, close_col) = source.offset_to_line_col(closing.start_offset());

        // Get first and last element lines
        let first_elem = elements.iter().next().unwrap();
        let last_elem = elements.iter().last().unwrap();
        let (first_elem_line, _) = source.offset_to_line_col(first_elem.location().start_offset());
        let (last_elem_line, _) =
            source.offset_to_line_col(last_elem.location().end_offset().saturating_sub(1));

        // Only check multiline hashes
        if open_line == close_line {
            return;
        }

        let open_same_as_first = open_line == first_elem_line;
        let close_same_as_last = close_line == last_elem_line;

        match enforced_style {
            "symmetrical" => {
                if open_same_as_first && !close_same_as_last {
                    diagnostics.push(self.diagnostic(
                        source,
                        close_line,
                        close_col,
                        "Closing hash brace must be on the same line as the last hash element when opening brace is on the same line as the first hash element.".to_string(),
                    ));
                }
                if !open_same_as_first && close_same_as_last {
                    diagnostics.push(self.diagnostic(
                        source,
                        close_line,
                        close_col,
                        "Closing hash brace must be on the line after the last hash element when opening brace is on a separate line from the first hash element.".to_string(),
                    ));
                }
            }
            "new_line" => {
                if close_same_as_last {
                    diagnostics.push(
                        self.diagnostic(
                            source,
                            close_line,
                            close_col,
                            "Closing hash brace must be on the line after the last hash element."
                                .to_string(),
                        ),
                    );
                }
            }
            "same_line" => {
                if !close_same_as_last {
                    diagnostics.push(
                        self.diagnostic(
                            source,
                            close_line,
                            close_col,
                            "Closing hash brace must be on the same line as the last hash element."
                                .to_string(),
                        ),
                    );
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        MultilineHashBraceLayout,
        "cops/layout/multiline_hash_brace_layout"
    );
}
