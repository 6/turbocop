use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ArrayAlignment;

impl Cop for ArrayAlignment {
    fn name(&self) -> &'static str {
        "Layout/ArrayAlignment"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let array_node = match node.as_array_node() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let elements = array_node.elements();
        if elements.len() < 2 {
            return Vec::new();
        }

        let first = match elements.iter().next() {
            Some(e) => e,
            None => return Vec::new(),
        };
        let (first_line, first_col) = source.offset_to_line_col(first.location().start_offset());

        let mut diagnostics = Vec::new();

        for elem in elements.iter().skip(1) {
            let (elem_line, elem_col) = source.offset_to_line_col(elem.location().start_offset());
            if elem_line != first_line && elem_col != first_col {
                diagnostics.push(self.diagnostic(
                    source,
                    elem_line,
                    elem_col,
                    "Align the elements of an array literal if they span more than one line."
                        .to_string(),
                ));
            }
        }

        diagnostics
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
}
