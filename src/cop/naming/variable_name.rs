use crate::cop::util::is_snake_case;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct VariableName;

impl Cop for VariableName {
    fn name(&self) -> &'static str {
        "Naming/VariableName"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let write_node = match node.as_local_variable_write_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let var_name = write_node.name().as_slice();

        // Skip names starting with _ (convention for unused vars)
        if var_name.starts_with(b"_") {
            return Vec::new();
        }

        if is_snake_case(var_name) {
            return Vec::new();
        }

        let loc = write_node.name_loc();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: Severity::Convention,
            cop_name: self.name().to_string(),
            message: "Use snake_case for variable names.".to_string(),
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
            &VariableName,
            include_bytes!("../../../testdata/cops/naming/variable_name/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &VariableName,
            include_bytes!("../../../testdata/cops/naming/variable_name/no_offense.rb"),
        );
    }
}
