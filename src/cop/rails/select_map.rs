use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SelectMap;

impl Cop for SelectMap {
    fn name(&self) -> &'static str {
        "Rails/SelectMap"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Outer method must be map or collect
        if chain.outer_method != b"map" && chain.outer_method != b"collect" {
            return Vec::new();
        }

        // Inner method must be select
        if chain.inner_method != b"select" {
            return Vec::new();
        }

        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // RuboCop checks: map must have a first_argument (like &:column_name)
        // and select must have exactly one symbol/string argument matching the column name.
        // The map argument should be &:column_name form.
        let map_column = match get_block_pass_symbol(source, &outer_call) {
            Some(name) => name,
            None => return Vec::new(),
        };

        // Inner call (select) must have exactly one symbol argument matching the column name
        let select_column = match get_single_symbol_arg(&chain.inner_call) {
            Some(name) => name,
            None => return Vec::new(),
        };

        if map_column != select_column {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `pluck(:{}` instead of `select` with `{}`.",
                String::from_utf8_lossy(&select_column),
                String::from_utf8_lossy(chain.outer_method),
            ),
        )]
    }
}

/// Get the symbol name from a `&:name` block argument on a CallNode.
fn get_block_pass_symbol<'a>(
    _source: &'a SourceFile,
    call: &ruby_prism::CallNode<'a>,
) -> Option<Vec<u8>> {
    let block = call.block()?;
    // Block argument: &:symbol
    let block_arg = block.as_block_argument_node()?;
    let expr = block_arg.expression()?;
    let sym = expr.as_symbol_node()?;
    Some(sym.unescaped().to_vec())
}

/// Get the single symbol argument from a CallNode like `select(:column_name)`.
fn get_single_symbol_arg(call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.len() != 1 {
        return None;
    }
    let sym = arg_list[0].as_symbol_node()?;
    Some(sym.unescaped().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SelectMap, "cops/rails/select_map");
}
