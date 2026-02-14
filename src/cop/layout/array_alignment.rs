use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
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
                diagnostics.push(Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location {
                        line: elem_line,
                        column: elem_col,
                    },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message:
                        "Align the elements of an array literal if they span more than one line."
                            .to_string(),
                });
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full, run_cop_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &ArrayAlignment,
            include_bytes!("../../../testdata/cops/layout/array_alignment/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &ArrayAlignment,
            include_bytes!("../../../testdata/cops/layout/array_alignment/no_offense.rb"),
        );
    }

    #[test]
    fn single_line_array_no_offense() {
        let source = b"x = [1, 2, 3]\n";
        let diags = run_cop_full(&ArrayAlignment, source);
        assert!(diags.is_empty());
    }
}
