use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct UnfreezeString;

impl Cop for UnfreezeString {
    fn name(&self) -> &'static str {
        "Performance/UnfreezeString"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            STRING_NODE,
        ]
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

        if call.name().as_slice() != b"new" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Only match bare `String` (ConstantReadNode), not `::String`
        // (constant_path_node) or qualified paths like `ActiveModel::Type::String`.
        // RuboCop's NodePattern `(const nil? :String)` requires a nil parent,
        // which matches ConstantReadNode but not ConstantPathNode (even `::String`
        // has a non-nil `cbase` parent in the Parser AST).
        let is_bare_string = if let Some(cr) = receiver.as_constant_read_node() {
            cr.name().as_slice() == b"String"
        } else {
            false
        };

        if !is_bare_string {
            return;
        }

        // Flag String.new with no args, or with a single string/dstr argument
        // (any content). RuboCop's pattern matches `{str dstr}` which covers
        // both plain strings and interpolated strings of any content.
        match call.arguments() {
            None => {} // String.new — flag it
            Some(arguments) => {
                let args = arguments.arguments();
                if args.len() != 1 {
                    return;
                }
                let first_arg = match args.iter().next() {
                    Some(a) => a,
                    None => return,
                };
                // Accept StringNode (str) or InterpolatedStringNode (dstr)
                if first_arg.as_string_node().is_none()
                    && first_arg.as_interpolated_string_node().is_none()
                {
                    return;
                }
            }
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use unary plus to get an unfrozen string literal.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(UnfreezeString, "cops/performance/unfreeze_string");
}
