use crate::cop::util::is_camel_case;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
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

        vec![self.diagnostic(
            source,
            line,
            column,
            "Use CamelCase for classes and modules.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ClassAndModuleCamelCase, "cops/naming/class_and_module_camel_case");
}
