use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct AccessorMethodName;

impl Cop for AccessorMethodName {
    fn name(&self) -> &'static str {
        "Naming/AccessorMethodName"
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
        let name_str = match std::str::from_utf8(method_name) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let message = if name_str.starts_with("get_") {
            "Do not prefix reader method names with `get_`."
        } else if name_str.starts_with("set_") {
            "Do not prefix writer method names with `set_`."
        } else {
            return Vec::new();
        };

        let loc = def_node.name_loc();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: Severity::Convention,
            cop_name: self.name().to_string(),
            message: message.to_string(),
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
            &AccessorMethodName,
            include_bytes!("../../../testdata/cops/naming/accessor_method_name/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &AccessorMethodName,
            include_bytes!("../../../testdata/cops/naming/accessor_method_name/no_offense.rb"),
        );
    }
}
