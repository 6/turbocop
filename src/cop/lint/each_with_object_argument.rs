use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct EachWithObjectArgument;

fn is_immutable_literal(node: &ruby_prism::Node<'_>) -> bool {
    matches!(
        node,
        ruby_prism::Node::IntegerNode { .. }
            | ruby_prism::Node::FloatNode { .. }
            | ruby_prism::Node::SymbolNode { .. }
    )
}

impl Cop for EachWithObjectArgument {
    fn name(&self) -> &'static str {
        "Lint/EachWithObjectArgument"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"each_with_object" {
            return Vec::new();
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args = arguments.arguments();
        let first_arg = match args.first() {
            Some(a) => a,
            None => return Vec::new(),
        };

        if !is_immutable_literal(&first_arg) {
            return Vec::new();
        }

        let loc = first_arg.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message: "`each_with_object` called with an immutable argument.".to_string(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &EachWithObjectArgument,
            include_bytes!("../../../testdata/cops/lint/each_with_object_argument/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &EachWithObjectArgument,
            include_bytes!("../../../testdata/cops/lint/each_with_object_argument/no_offense.rb"),
        );
    }
}
