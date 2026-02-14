use crate::cop::util::is_camel_case;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct ClassAndModuleCamelCase;

impl Cop for ClassAndModuleCamelCase {
    fn name(&self) -> &'static str {
        "Naming/ClassAndModuleCamelCase"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        if let Some(class_node) = node.as_class_node() {
            let constant_path = class_node.constant_path();
            return self.check_constant_path(source, &constant_path);
        }

        if let Some(module_node) = node.as_module_node() {
            let constant_path = module_node.constant_path();
            return self.check_constant_path(source, &constant_path);
        }

        Vec::new()
    }
}

impl ClassAndModuleCamelCase {
    fn check_constant_path(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
    ) -> Vec<Diagnostic> {
        // Extract the name to check: for simple names it's ConstantReadNode,
        // for paths like Foo::Bar it's ConstantPathNode (we check the rightmost name)
        let (const_name, loc) = if let Some(read_node) = node.as_constant_read_node() {
            (read_node.name().as_slice(), node.location())
        } else if let Some(path_node) = node.as_constant_path_node() {
            // Check the rightmost segment (the `name` field)
            if let Some(name) = path_node.name() {
                (name.as_slice(), path_node.name_loc())
            } else {
                return Vec::new();
            }
        } else {
            return Vec::new();
        };

        if is_camel_case(const_name) {
            return Vec::new();
        }

        let (line, column) = source.offset_to_line_col(loc.start_offset());

        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: Severity::Convention,
            cop_name: self.name().to_string(),
            message: "Use CamelCase for classes and modules.".to_string(),
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
            &ClassAndModuleCamelCase,
            include_bytes!(
                "../../../testdata/cops/naming/class_and_module_camel_case/offense.rb"
            ),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &ClassAndModuleCamelCase,
            include_bytes!(
                "../../../testdata/cops/naming/class_and_module_camel_case/no_offense.rb"
            ),
        );
    }
}
