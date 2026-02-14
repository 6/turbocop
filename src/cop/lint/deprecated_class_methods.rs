use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct DeprecatedClassMethods;

impl Cop for DeprecatedClassMethods {
    fn name(&self) -> &'static str {
        "Lint/DeprecatedClassMethods"
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

        let method_name = call.name().as_slice();
        if method_name != b"exists?" {
            return Vec::new();
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let const_node = match receiver.as_constant_read_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let class_name = const_node.name().as_slice();
        let message = if class_name == b"File" {
            "`File.exists?` is deprecated in favor of `File.exist?`."
        } else if class_name == b"Dir" {
            "`Dir.exists?` is deprecated in favor of `Dir.exist?`."
        } else {
            return Vec::new();
        };

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
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
            &DeprecatedClassMethods,
            include_bytes!("../../../testdata/cops/lint/deprecated_class_methods/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &DeprecatedClassMethods,
            include_bytes!("../../../testdata/cops/lint/deprecated_class_methods/no_offense.rb"),
        );
    }
}
