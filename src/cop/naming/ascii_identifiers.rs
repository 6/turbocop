use crate::cop::util::is_ascii_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct AsciiIdentifiers;

impl Cop for AsciiIdentifiers {
    fn name(&self) -> &'static str {
        "Naming/AsciiIdentifiers"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        if let Some(def_node) = node.as_def_node() {
            let method_name = def_node.name().as_slice();
            if !is_ascii_name(method_name) {
                let loc = def_node.name_loc();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use only ascii symbols in identifiers.".to_string(),
                )];
            }
        }

        if let Some(write_node) = node.as_local_variable_write_node() {
            let var_name = write_node.name().as_slice();
            if !is_ascii_name(var_name) {
                let loc = write_node.name_loc();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use only ascii symbols in identifiers.".to_string(),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AsciiIdentifiers, "cops/naming/ascii_identifiers");
}
