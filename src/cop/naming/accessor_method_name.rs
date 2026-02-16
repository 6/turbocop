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

        // Count ALL parameters (including optional, rest, keyword, block)
        // to match RuboCop which uses `node.arguments.one?` / `!node.arguments?`.
        let total_param_count = def_node.parameters().map_or(0, |params| {
            params.requireds().len()
                + params.optionals().len()
                + params.posts().len()
                + params.keywords().len()
                + if params.rest().is_some() { 1 } else { 0 }
                + if params.keyword_rest().is_some() { 1 } else { 0 }
                + if params.block().is_some() { 1 } else { 0 }
        });
        // For set_, also ensure the single argument is a regular arg (not block, rest, etc.)
        let has_one_regular_arg = total_param_count == 1
            && def_node
                .parameters()
                .map_or(false, |params| params.requireds().len() == 1);

        let message = if name_str.starts_with("get_") && total_param_count == 0 {
            // Reader methods: get_* with no arguments
            "Do not prefix reader method names with `get_`."
        } else if name_str.starts_with("set_") && has_one_regular_arg {
            // Writer methods: set_* with exactly one regular argument
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
