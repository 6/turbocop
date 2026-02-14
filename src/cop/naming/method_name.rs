use crate::cop::util::is_snake_case;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct MethodName;

/// Returns true if the name consists entirely of non-alphabetic characters (operator methods).
fn is_operator_method(name: &[u8]) -> bool {
    !name.iter().any(|b| b.is_ascii_alphabetic())
}

impl Cop for MethodName {
    fn name(&self) -> &'static str {
        "Naming/MethodName"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        let method_name = def_node.name().as_slice();

        // Skip operator methods (e.g., +, -, [], <=>, ==)
        if is_operator_method(method_name) {
            return Vec::new();
        }

        if is_snake_case(method_name) {
            return Vec::new();
        }

        let loc = def_node.name_loc();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: Severity::Convention,
            cop_name: self.name().to_string(),
            message: "Use snake_case for method names.".to_string(),
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
            &MethodName,
            include_bytes!("../../../testdata/cops/naming/method_name/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &MethodName,
            include_bytes!("../../../testdata/cops/naming/method_name/no_offense.rb"),
        );
    }
}
