use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
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

        // Count required parameters (excludes optional, rest, keyword, block)
        let param_count = def_node
            .parameters()
            .map_or(0, |params| params.requireds().len());

        let message = if name_str.starts_with("get_") && param_count == 0 {
            // Reader methods: get_* with no arguments
            "Do not prefix reader method names with `get_`."
        } else if name_str.starts_with("set_") && param_count == 1 {
            // Writer methods: set_* with exactly one argument
            "Do not prefix writer method names with `set_`."
        } else {
            return Vec::new();
        };

        let loc = def_node.name_loc();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        vec![self.diagnostic(source, line, column, message.to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AccessorMethodName, "cops/naming/accessor_method_name");
}
