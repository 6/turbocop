use crate::cop::util::{is_camel_case, is_screaming_snake_case};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ConstantName;

impl Cop for ConstantName {
    fn name(&self) -> &'static str {
        "Naming/ConstantName"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        if let Some(cw) = node.as_constant_write_node() {
            let const_name = cw.name().as_slice();
            return self.check_constant(source, const_name, &cw.name_loc());
        }

        if let Some(cpw) = node.as_constant_path_write_node() {
            // Get the final segment of the path
            let target = cpw.target();
            let name_loc = target.name_loc();
            let const_name = target.name().map(|n| n.as_slice()).unwrap_or(b"");
            return self.check_constant(source, const_name, &name_loc);
        }

        Vec::new()
    }
}

impl ConstantName {
    fn check_constant(
        &self,
        source: &SourceFile,
        const_name: &[u8],
        loc: &ruby_prism::Location<'_>,
    ) -> Vec<Diagnostic> {
        // Allow SCREAMING_SNAKE_CASE (standard constant style)
        if is_screaming_snake_case(const_name) {
            return Vec::new();
        }

        // Allow CamelCase (class/module-like constants)
        if is_camel_case(const_name) {
            return Vec::new();
        }

        let (line, column) = source.offset_to_line_col(loc.start_offset());

        vec![self.diagnostic(
            source,
            line,
            column,
            "Use SCREAMING_SNAKE_CASE for constants.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ConstantName, "cops/naming/constant_name");
}
