use crate::cop::shared::node_type::{CALL_NODE, DEF_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Investigation: synthetic corpus showed 1 FP and 1 FN because the offense was
/// reported at the `attr_reader` call location instead of the trailing comma position.
/// RuboCop reports at the comma after the last attribute argument (before the DefNode).
/// Fixed by scanning forward from the last non-DefNode argument to find the `,` character.
pub struct TrailingCommaInAttributeDeclaration;

const ATTR_METHODS: &[&[u8]] = &[b"attr_reader", b"attr_writer", b"attr_accessor", b"attr"];

impl Cop for TrailingCommaInAttributeDeclaration {
    fn name(&self) -> &'static str {
        "Lint/TrailingCommaInAttributeDeclaration"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, DEF_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be a bare call (no receiver) to an attr method
        if call.receiver().is_some() {
            return;
        }

        let method_name = call.name().as_slice();
        if !ATTR_METHODS.contains(&method_name) {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // Check if the last argument is a DefNode (method definition).
        // This happens when there's a trailing comma in the attribute declaration:
        // `attr_reader :foo,` followed by `def bar; end` causes the `def` to be
        // parsed as an argument to `attr_reader`.
        let last_arg = &arg_list[arg_list.len() - 1];
        if last_arg.as_def_node().is_some() && arg_list.len() >= 2 {
            // Find the trailing comma after the last real attribute argument
            // (the argument before the DefNode). RuboCop reports the offense
            // at the comma position, not at the attr_reader call.
            let last_attr_arg = &arg_list[arg_list.len() - 2];
            let search_start = last_attr_arg.location().end_offset();
            let src_bytes = source.as_bytes();
            let comma_offset = src_bytes[search_start..]
                .iter()
                .position(|&b| b == b',')
                .map(|pos| search_start + pos);
            let offset = comma_offset.unwrap_or(search_start);
            let (line, column) = source.offset_to_line_col(offset);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Avoid leaving a trailing comma in attribute declarations.".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        TrailingCommaInAttributeDeclaration,
        "cops/lint/trailing_comma_in_attribute_declaration"
    );
}
