use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{HASH_NODE, KEYWORD_HASH_NODE};

pub struct MultilineHashKeyLineBreaks;

impl Cop for MultilineHashKeyLineBreaks {
    fn name(&self) -> &'static str {
        "Layout/MultilineHashKeyLineBreaks"
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

        let (open_line, _) = source.offset_to_line_col(opening.start_offset());
        let (close_line, _) = source.offset_to_line_col(closing.start_offset());

        // Only check multiline hashes
        if open_line == close_line {
            return Vec::new();
        }

        let elements: Vec<ruby_prism::Node<'_>> = hash.elements().iter().collect();
        if elements.len() < 2 {
            return Vec::new();
        }

        let mut diagnostics = Vec::new();

        for i in 1..elements.len() {
            let prev = &elements[i - 1];
            let curr = &elements[i];

            let (prev_line, _) = source.offset_to_line_col(
                prev.location().end_offset().saturating_sub(1),
            );
            let (curr_line, curr_col) = source.offset_to_line_col(curr.location().start_offset());

            if prev_line == curr_line {
                diagnostics.push(self.diagnostic(
                    source,
                    curr_line,
                    curr_col,
                    "Each item in a multi-line hash must start on a separate line.".to_string(),
                ));
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        MultilineHashKeyLineBreaks,
        "cops/layout/multiline_hash_key_line_breaks"
    );
}
