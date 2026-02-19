use crate::cop::util::is_camel_case;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CLASS_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, MODULE_NODE};

pub struct ClassAndModuleCamelCase;

impl Cop for ClassAndModuleCamelCase {
    fn name(&self) -> &'static str {
        "Naming/ClassAndModuleCamelCase"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, MODULE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allowed_names = config.get_string_array("AllowedNames");

        if let Some(class_node) = node.as_class_node() {
            let constant_path = class_node.constant_path();
            let diags = self.check_constant_path(source, &constant_path);
            if !diags.is_empty() {
                if let Some(allowed) = &allowed_names {
                    let name = self.extract_name(&constant_path);
                    if let Some(n) = name {
                        if allowed.iter().any(|a| a == n) {
                            return Vec::new();
                        }
                    }
                }
            }
            return diags;
        }

        if let Some(module_node) = node.as_module_node() {
            let constant_path = module_node.constant_path();
            let diags = self.check_constant_path(source, &constant_path);
            if !diags.is_empty() {
                if let Some(allowed) = &allowed_names {
                    let name = self.extract_name(&constant_path);
                    if let Some(n) = name {
                        if allowed.iter().any(|a| a == n) {
                            return Vec::new();
                        }
                    }
                }
            }
            return diags;
        }

        Vec::new()
    }
}

impl ClassAndModuleCamelCase {
    fn extract_name<'a>(&self, node: &'a ruby_prism::Node<'a>) -> Option<&'a str> {
        if let Some(read_node) = node.as_constant_read_node() {
            std::str::from_utf8(read_node.name().as_slice()).ok()
        } else if let Some(path_node) = node.as_constant_path_node() {
            path_node
                .name()
                .and_then(|n| std::str::from_utf8(n.as_slice()).ok())
        } else {
            None
        }
    }

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
