use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct ArgumentAlignment;

impl Cop for ArgumentAlignment {
    fn name(&self) -> &'static str {
        "Layout/ArgumentAlignment"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let arguments = match call_node.arguments() {
            Some(args) => args,
            None => return Vec::new(),
        };

        let arg_list = arguments.arguments();
        if arg_list.len() < 2 {
            return Vec::new();
        }

        let first_arg = match arg_list.iter().next() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let (first_line, first_col) = source.offset_to_line_col(first_arg.location().start_offset());

        let mut diagnostics = Vec::new();

        for arg in arg_list.iter().skip(1) {
            let (arg_line, arg_col) = source.offset_to_line_col(arg.location().start_offset());
            // Only check arguments on different lines than the first
            if arg_line != first_line && arg_col != first_col {
                diagnostics.push(Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location {
                        line: arg_line,
                        column: arg_col,
                    },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Align the arguments of a method call if they span more than one line."
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
            &ArgumentAlignment,
            include_bytes!("../../../testdata/cops/layout/argument_alignment/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &ArgumentAlignment,
            include_bytes!("../../../testdata/cops/layout/argument_alignment/no_offense.rb"),
        );
    }

    #[test]
    fn single_line_call_no_offense() {
        let source = b"foo(1, 2, 3)\n";
        let diags = run_cop_full(&ArgumentAlignment, source);
        assert!(diags.is_empty());
    }
}
