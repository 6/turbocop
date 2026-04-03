use crate::cop::shared::node_type::{BLOCK_ARGUMENT_NODE, CALL_NODE, STRING_NODE, SYMBOL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/SelectMap
///
/// Detects `select(:col).map(&:col)` chains (including with intermediate methods
/// like `.where(...)`) and suggests using `pluck(:col)` instead.
///
/// Root cause of prior FNs: only symbol arguments were matched in `select()`,
/// but RuboCop's pattern uses `({sym str} $_)` which accepts both symbols and strings.
/// Also, only direct 2-method chains were matched; RuboCop walks descendants to find
/// `select` anywhere in the receiver chain (handling intermediate methods like `.where`).
/// Report location was at the full expression start instead of the `select` method selector.
pub struct SelectMap;

impl Cop for SelectMap {
    fn name(&self) -> &'static str {
        "Rails/SelectMap"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_ARGUMENT_NODE, CALL_NODE, STRING_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let outer_method = outer_call.name();
        // Outer method must be map or collect
        if outer_method.as_slice() != b"map" && outer_method.as_slice() != b"collect" {
            return;
        }

        // map/collect must have a &:symbol block argument
        let map_column = match get_block_pass_symbol(&outer_call) {
            Some(name) => name,
            None => return,
        };

        // Walk the receiver chain to find a `select` call with a matching column argument.
        // RuboCop uses `node.descendants.select { |n| n.method?(:select) }` and requires
        // exactly one such node (select_method_nodes.one?).
        let select_offset = match find_select_in_chain(&outer_call, &map_column) {
            Some(offset) => offset,
            None => return,
        };

        // Report at the select method's selector, matching RuboCop behavior
        let (line, column) = source.offset_to_line_col(select_offset);
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `pluck(:{}` instead of `select` with `{}`.",
                String::from_utf8_lossy(&map_column),
                String::from_utf8_lossy(outer_method.as_slice()),
            ),
        ));
    }
}

/// Walk the receiver chain of a CallNode to find a `select` call whose single
/// symbol/string argument matches `column_name`. Returns the select method's
/// message_loc start offset if exactly one match is found.
fn find_select_in_chain(
    outer_call: &ruby_prism::CallNode<'_>,
    column_name: &[u8],
) -> Option<usize> {
    let mut match_offsets = Vec::new();
    let mut current = outer_call.receiver()?;

    while let Some(call) = current.as_call_node() {
        if call.name().as_slice() == b"select" {
            if let Some(arg_name) = get_single_symbol_or_string_arg(&call) {
                if arg_name == column_name {
                    let offset = call
                        .message_loc()
                        .map(|loc| loc.start_offset())
                        .unwrap_or_else(|| call.location().start_offset());
                    match_offsets.push(offset);
                }
            }
        }
        // Continue walking up the receiver chain
        match call.receiver() {
            Some(recv) => current = recv,
            None => break,
        }
    }

    // RuboCop requires exactly one matching select node
    if match_offsets.len() == 1 {
        Some(match_offsets[0])
    } else {
        None
    }
}

/// Get the symbol name from a `&:name` block argument on a CallNode.
fn get_block_pass_symbol(call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let block = call.block()?;
    // Block argument: &:symbol
    let block_arg = block.as_block_argument_node()?;
    let expr = block_arg.expression()?;
    let sym = expr.as_symbol_node()?;
    Some(sym.unescaped().to_vec())
}

/// Get the single symbol or string argument from a CallNode like `select(:column_name)` or `select('column_name')`.
/// Matches RuboCop's `({sym str} $_)` pattern.
fn get_single_symbol_or_string_arg(call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.len() != 1 {
        return None;
    }
    if let Some(sym) = arg_list[0].as_symbol_node() {
        return Some(sym.unescaped().to_vec());
    }
    if let Some(s) = arg_list[0].as_string_node() {
        return Some(s.unescaped().to_vec());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SelectMap, "cops/rails/select_map");
}
