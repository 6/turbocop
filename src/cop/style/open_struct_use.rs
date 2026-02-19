use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CONSTANT_PATH_NODE, CONSTANT_READ_NODE};

pub struct OpenStructUse;

impl Cop for OpenStructUse {
    fn name(&self) -> &'static str {
        "Style/OpenStructUse"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CONSTANT_PATH_NODE, CONSTANT_READ_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Check ConstantReadNode (OpenStruct)
        if let Some(cr) = node.as_constant_read_node() {
            if cr.name().as_slice() == b"OpenStruct" {
                let loc = cr.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Avoid using `OpenStruct`; use `Struct`, `Hash`, a class, or ActiveModel attributes instead."
                        .to_string(),
                )];
            }
        }

        // Check ConstantPathNode (::OpenStruct or Module::OpenStruct)
        if let Some(cp) = node.as_constant_path_node() {
            if let Some(name) = cp.name() {
                if name.as_slice() == b"OpenStruct" {
                    let loc = cp.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Avoid using `OpenStruct`; use `Struct`, `Hash`, a class, or ActiveModel attributes instead."
                            .to_string(),
                    )];
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OpenStructUse, "cops/style/open_struct_use");
}
